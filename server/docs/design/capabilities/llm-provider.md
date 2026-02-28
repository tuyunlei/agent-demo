# LLM Provider 抽象设计（D-CAP-01）

**状态**：Draft（MVP 可实施）

**更新时间**：2026-02-24

**所属层**：Layer 3 Capability / `agent-llm`

**关联文档**：
- `docs/design/architecture.md`
- `docs/design/core/event-model.md`
- `docs/design/archive/llm-gateway/overview.md`（历史参考）
- `crates/agent-domain/src/ports.rs`（当前接口现状）

---

## 0. 背景与问题陈述

当前 `agent-demo` 已有最小可用 LLM 调用能力。

但现状接口仍偏单 provider 视角。

例如：

- `LlmProvider::generate` 单方法，未区分 complete/stream。
- `LlmError` 分类较粗，难以表达 fallback 决策。
- 对多 provider 路由、模型路由、熔断/降级缺乏统一协议。
- 编排层会越来越需要“provider-agnostic”的可替换语义。

随着 Layer 2 编排增强（工具循环、会话治理、压缩等），

如果 Layer 3 没有稳定抽象，

后续新增 OpenAI / Anthropic / OpenRouter / 本地模型时，

将把 provider 差异泄漏到编排层。

本设计目标：

在不引入过度复杂度的前提下，

定义一个 **可落地、可扩展、可 fallback** 的 LLM Provider 抽象。

---

## 1. 设计目标

本节对应验收项“设计目标”。

### 1.1 Provider-agnostic

编排层（Layer 2）不应知道调用的是 OpenAI 还是 Anthropic。

编排层只依赖：

- `Arc<dyn LlmProvider>`
- 统一请求模型 `LlmRequest`
- 统一响应模型 `LlmResponse`
- 统一错误模型 `LlmError`

禁止行为：

- 在编排层分支判断 provider 名称后拼不同 payload。
- 在编排层处理 provider 私有错误码。
- 在编排层解析 provider 私有 tool_call 格式。

### 1.2 多 provider 支持

系统应支持：

- 配置多个 provider endpoint/credential。
- 配置多个可用模型，并可映射到 provider。
- 运行时根据策略选主 provider。

最小能力：

- 主 provider + 至少 1 个 fallback provider。
- 支持按模型 key 路由（例如 `default-chat`, `tool-heavy`）。

### 1.3 Fallback 策略

目标不是“所有错误都兜底重试”。

而是：

- 对可恢复错误自动降级到备选 provider。
- 对不可恢复错误快速失败并返回。
- 避免错误放大（无限重试/级联超时）。

设计上要明确：

- 哪些错误触发 retry。
- 哪些错误触发 fallback。
- 哪些错误直接返回。

### 1.4 与四层架构一致

遵循 `architecture.md`：

- Trait 定义在 Layer 3。
- OpenAI/Anthropic adapter 在 Layer 4 实现。
- Layer 2 仅依赖 Layer 3 抽象。

### 1.5 与事件模型一致

`LlmResponse` 必须可稳定映射到事件流：

- `AssistantMessage`
- `ToolCallRequest`

并保留 usage / finish_reason 供审计与成本统计。

---

## 2. LlmProvider Trait 设计

本节给出核心 trait 与能力模型。

### 2.1 设计原则

1. 接口语义清晰：complete 与 stream 区分。
2. 请求/响应统一：屏蔽 provider 差异。
3. 错误可决策：可重试、可 fallback、不可恢复可判断。
4. 默认先支持 complete；stream 为 MVP 可选。

### 2.2 Rust 伪代码：核心 Trait

```rust
use std::sync::Arc;
use std::time::Duration;

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider 标识（如 openai / anthropic）
    fn provider_id(&self) -> &str;

    /// 非流式调用（MVP 必做）
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// 流式调用（MVP 可选）
    async fn stream(&self, req: LlmRequest) -> Result<LlmStream, LlmError> {
        Err(LlmError::UnsupportedCapability {
            provider: self.provider_id().to_string(),
            capability: "stream".to_string(),
        })
    }

    /// Provider 能力声明
    fn capabilities(&self) -> LlmProviderCapabilities {
        LlmProviderCapabilities::default()
    }
}

pub type LlmStream = std::pin::Pin<
    Box<dyn futures_core::Stream<Item = Result<LlmStreamChunk, LlmError>> + Send>
>;
```

### 2.3 Rust 伪代码：能力声明

```rust
#[derive(Debug, Clone, Default)]
pub struct LlmProviderCapabilities {
    pub supports_stream: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_json_mode: bool,
    pub supports_stateful: bool,
}
```

### 2.4 Rust 伪代码：stream chunk

```rust
#[derive(Debug, Clone)]
pub struct LlmStreamChunk {
    pub delta_text: Option<String>,
    pub tool_call_delta: Vec<ToolCallDelta>,
    pub finish_reason: Option<FinishReason>,
    pub usage_delta: Option<LlmUsageDelta>,
}

#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    pub id: String,
    pub name: Option<String>,
    pub arguments_fragment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LlmUsageDelta {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
}
```

### 2.5 与当前接口关系

当前 `agent-domain::LlmProvider` 是：

- `generate(request)` 单方法。

建议演进为：

- `complete(request)`
- `stream(request)`（默认 unsupported）

迁移策略：

- 先在 domain 新增 trait 版本。
- 旧 `generate` 通过 shim 过渡。
- 编排层先调用 `complete`。

---

## 3. 请求/响应类型定义（统一模型）

本节给出请求响应类型，满足“至少 6 个 struct/enum”。

### 3.1 统一消息模型

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelMessage {
    pub role: MessageRole,
    pub content: MessageContent,
    pub tool_call_id: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { url: String },
    InputJson { json: String },
}
```

### 3.2 工具定义模型

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub strict: bool,
}
```

### 3.3 工具调用模型

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}
```

### 3.4 请求模型

```rust
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<ModelMessage>,
    pub tool_specs: Vec<ToolSpec>,
    pub builtin_tools: Vec<String>,
    pub previous_response_id: Option<String>,
    pub config: LlmRequestConfig,
    pub metadata: LlmRequestMetadata,
}

#[derive(Debug, Clone)]
pub struct LlmRequestConfig {
    pub model: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<std::time::Duration>,
    pub json_mode: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LlmRequestMetadata {
    pub tenant_id: Option<String>,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub trace_id: Option<String>,
}
```

### 3.4.1 Stateful 模式

编排层始终构造完整 `messages`，不因为 provider 是否支持 stateful 而改变调用接口。

当 provider 声明 `supports_stateful = true` 时，可在请求中带上 `previous_response_id`，由 provider 侧利用该 ID 进行会话续接优化。

回退到 stateless（不传 `previous_response_id`）的条件：

1. system prompt 发生变更
2. toolset（`tool_specs` 或 `builtin_tools`）发生变更
3. model 发生变更

若 provider 返回“response id 无效/不可用”等错误，编排层应在**同一 turn**降级为 stateless 重试一次（保持其它参数不变，仅移除 `previous_response_id`）。

### 3.5 响应模型

```rust
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    pub usage: Option<LlmUsage>,
    pub model: String,
    pub provider: String,
    pub response_id: Option<String>,
}
```

### 3.6 Usage 模型

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,

    /// prompt cache 命中（provider 支持时）
    pub cache_read_tokens: Option<u32>,

    /// prompt cache 写入（provider 支持时）
    pub cache_write_tokens: Option<u32>,
}
```

### 3.7 FinishReason 枚举

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
    Error,
}
```

### 3.8 错误模型

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum LlmError {
    #[error("rate limited")]
    RateLimit,

    #[error("timeout")]
    Timeout,

    #[error("authentication failed")]
    AuthError,

    #[error("provider unavailable")]
    ProviderDown,

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("unsupported capability: provider={provider} capability={capability}")]
    UnsupportedCapability { provider: String, capability: String },

    #[error("transport error: {0}")]
    Transport(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl LlmError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::RateLimit
                | LlmError::Timeout
                | LlmError::ProviderDown
                | LlmError::Transport(_)
        )
    }

    pub fn is_fallbackable(&self) -> bool {
        matches!(
            self,
            LlmError::RateLimit
                | LlmError::Timeout
                | LlmError::ProviderDown
                | LlmError::Transport(_)
        )
    }
}
```

---

## 4. Provider 配置与选择

本节定义多 provider 配置、运行时选择、fallback chain。

### 4.1 配置结构（Rust 伪代码）

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LlmProvidersConfig {
    pub providers: HashMap<String, ProviderConfig>,
    pub routes: HashMap<String, ModelRoute>,
    pub defaults: LlmDefaults,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub endpoint: Option<String>,
    pub api_key_env: String,
    pub request_timeout_ms: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    OpenRouter,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct ModelRoute {
    /// 例如 default-chat / tool-heavy / long-context
    pub route_key: String,

    /// primary + fallbacks，格式 provider/model
    pub chain: Vec<ModelRef>,
}

#[derive(Debug, Clone)]
pub struct ModelRef {
    pub provider_id: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct LlmDefaults {
    pub route_key: String,
    pub max_retries_per_provider: u8,
    pub retry_backoff_ms: u64,
    pub retry_backoff_max_ms: u64,
}
```

### 4.2 示例配置（YAML 概念）

```yaml
llm:
  defaults:
    route_key: default-chat
    max_retries_per_provider: 2
    retry_backoff_ms: 300
    retry_backoff_max_ms: 3000

  providers:
    openai-main:
      kind: openai
      endpoint: https://api.openai.com/v1
      api_key_env: OPENAI_API_KEY
      request_timeout_ms: 60000
      enabled: true

    anthropic-main:
      kind: anthropic
      endpoint: https://api.anthropic.com
      api_key_env: ANTHROPIC_API_KEY
      request_timeout_ms: 60000
      enabled: true

    openrouter-backup:
      kind: openrouter
      endpoint: https://openrouter.ai/api/v1
      api_key_env: OPENROUTER_API_KEY
      request_timeout_ms: 45000
      enabled: true

  routes:
    default-chat:
      chain:
        - provider_id: openai-main
          model: gpt-5-mini
        - provider_id: anthropic-main
          model: claude-sonnet-4-6
        - provider_id: openrouter-backup
          model: openai/gpt-4.1-mini

    tool-heavy:
      chain:
        - provider_id: openai-main
          model: gpt-5
        - provider_id: anthropic-main
          model: claude-opus-4-1
```

### 4.3 运行时选择流程

#### 输入

- `route_key`（来自业务场景或默认）
- `LlmRequest.config.model`（可覆盖 route 默认模型）

#### 流程

1. 读取 `route_key` 对应 `ModelRoute`。
2. 构建候选链 `[(provider, model), ...]`。
3. 过滤禁用 provider。
4. 对每个候选执行：retry -> success/fail。
5. 当前候选不可恢复则直接返回。
6. 当前候选可 fallback 则进入下一个候选。
7. 全部失败返回 `FallbackExhausted`（可包装在 `LlmError::ProviderDown`）。

### 4.4 Fallback 链要求

必须支持：

`primary -> fallback1 -> fallback2`

例如：

1. `openai-main/gpt-5-mini`（primary）
2. `anthropic-main/claude-sonnet-4-6`（fallback1）
3. `openrouter-backup/openai/gpt-4.1-mini`（fallback2）

### 4.5 Fallback 具体流程描述

```text
start
  -> try primary
      -> success => return response
      -> retryable error => retry same provider (exponential backoff)
      -> still fail & fallbackable => try fallback1
      -> non-fallbackable => return error

  -> try fallback1
      -> success => return response
      -> retryable error => retry same provider
      -> still fail & fallbackable => try fallback2
      -> non-fallbackable => return error

  -> try fallback2
      -> success => return response
      -> fail => return exhausted error
end
```

---

## 5. 错误处理与重试策略

本节定义 LlmError 分类、fallback 触发条件、重试算法。

### 5.1 错误分类

核心类型：

- `RateLimit`
- `Timeout`
- `AuthError`
- `InvalidResponse`
- `ProviderDown`
- `InvalidRequest`
- `Transport`

### 5.2 错误分类到动作映射

| 错误类型 | retry | fallback | 直接返回 |
|---|---:|---:|---:|
| RateLimit | ✅ | ✅ | ❌ |
| Timeout | ✅ | ✅ | ❌ |
| ProviderDown | ✅ | ✅ | ❌ |
| Transport | ✅ | ✅ | ❌ |
| AuthError | ❌ | ❌ | ✅ |
| InvalidRequest | ❌ | ❌ | ✅ |
| InvalidResponse | ❌(默认) | ✅(可选) | ✅(默认) |
| UnsupportedCapability | ❌ | ✅(当链中有支持者) | ✅ |

说明：

- `AuthError` 通常是配置问题，fallback 可能掩盖真实故障，不建议自动继续。
- `InvalidRequest` 是调用方 bug，必须尽早暴露。
- `InvalidResponse` 默认直接返回；若某 provider 偶发返回脏数据，可按策略设为 fallbackable。

### 5.3 重试策略

推荐指数退避：

`delay = min(base * 2^attempt, max_delay) + jitter`

Rust 伪代码：

```rust
pub struct RetryPolicy {
    pub max_attempts: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter_ms: u64,
}

impl RetryPolicy {
    pub fn backoff(&self, attempt: u8) -> std::time::Duration {
        let exp = self.base_delay_ms.saturating_mul(1u64 << attempt.min(10));
        let capped = exp.min(self.max_delay_ms);
        let jitter = fastrand::u64(..=self.jitter_ms);
        std::time::Duration::from_millis(capped + jitter)
    }
}
```

### 5.4 单 provider 调用伪代码

```rust
async fn call_with_retry(
    provider: &dyn LlmProvider,
    req: LlmRequest,
    retry: &RetryPolicy,
) -> Result<LlmResponse, LlmError> {
    let mut attempt = 0u8;

    loop {
        let result = provider.complete(req.clone()).await;

        match result {
            Ok(resp) => return Ok(resp),
            Err(err) if err.is_retryable() && attempt + 1 < retry.max_attempts => {
                tokio::time::sleep(retry.backoff(attempt)).await;
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}
```

### 5.5 多 provider fallback 编排伪代码

```rust
async fn call_with_fallback_chain(
    chain: Vec<(Arc<dyn LlmProvider>, String)>,
    req: LlmRequest,
    retry: RetryPolicy,
) -> Result<LlmResponse, LlmError> {
    let mut last_err: Option<LlmError> = None;

    for (provider, model) in chain {
        let mut req_for_candidate = req.clone();
        req_for_candidate.config.model = model;

        match call_with_retry(provider.as_ref(), req_for_candidate, &retry).await {
            Ok(resp) => return Ok(resp),
            Err(err) if err.is_fallbackable() => {
                last_err = Some(err);
                continue;
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_err.unwrap_or(LlmError::ProviderDown))
}
```

---

## 6. 与其他层关系

本节明确 Layer 2 / Layer 3 / Layer 4 协作方式，并映射事件模型。

### 6.1 编排层如何使用

编排层依赖：

```rust
pub struct TurnExecutor {
    llm: Arc<dyn LlmProvider>,
    tools: Arc<dyn ToolRuntime>,
    store: Arc<dyn EventStore>,
}
```

调用方式：

1. 组装 `LlmRequest(messages + tool_specs + config)`。
2. 调用 `llm.complete(req)`。
3. 根据 `LlmResponse.finish_reason` 进入：
   - `Stop`：写 AssistantMessage 并结束。
   - `ToolCalls`：写 ToolCallRequest，执行工具后继续下一轮。

### 6.2 基础设施层如何实现

Layer 4 实现示例：

- `OpenAiProviderAdapter` implements `LlmProvider`
- `AnthropicProviderAdapter` implements `LlmProvider`
- `OpenRouterProviderAdapter` implements `LlmProvider`

职责边界：

- 适配 HTTP/SDK 协议。
- provider 错误映射到 `LlmError`。
- provider payload 映射到统一 `LlmResponse`。

不应做：

- 不在 adapter 内写业务事件。
- 不在 adapter 内做跨 provider fallback 编排（可有薄封装，但策略仍属 Layer 3/2）。

### 6.3 与事件模型关系

参考 `event-model.md`，`LlmResponse` 映射规则：

#### 映射到 AssistantMessage

当 `content` 非空时：

- 生成 `AssistantMessageEvent`
- 字段映射：
  - `text <- LlmResponse.content`
  - `model <- LlmResponse.model`
  - `prompt_tokens/completion_tokens <- usage`
  - `finish_reason <- FinishReason`

#### 映射到 ToolCallRequest

当 `tool_calls` 非空时：

- 每个 `ToolCall` 生成一条 `ToolCallRequestEvent`
- 字段映射：
  - `request_id <- ToolCall.id`
  - `tool_name <- ToolCall.name`
  - `arguments_json <- ToolCall.arguments`

#### 事件顺序建议

1. `AssistantMessage`（可选，取决于 provider 是否同时给文字）
2. `ToolCallRequest`（一个或多个）

与 `event-model.md` 保持一致：

- 事件是事实记录。
- 编排层负责写入顺序与因果链。

### 6.4 观测与审计建议

建议在 Layer 2 记录：

- 实际命中 provider/model。
- 重试次数。
- fallback 路径。
- 失败分类（rate_limit/timeout/auth 等）。

用于：

- 评估路由策略。
- 成本与稳定性分析。

---

## 7. 与参考框架对比与设计选择

本节回答“ZeroClaw vs 本设计差异与理由”。

### 7.1 ZeroClaw 借鉴点

从 `zeroclaw/src/providers/traits.rs` 与 `openai.rs` 可见：

- 有清晰 Provider trait。
- 统一 chat 请求/响应。
- 工具调用与 usage 已有抽象。
- 包含 streaming 能力占位。

本设计继承：

- trait-first 思路。
- provider 能力声明。
- tool_calls 与 usage 的统一模型。

### 7.2 PicoClaw 借鉴点

从 `picoclaw/pkg/providers/types.go` 与 `fallback.go` 可见：

- `LLMProvider` 接口稳定。
- 有明确 `FailoverReason` 分类。
- fallback 链执行流程清晰。
- cooldown / classify-error 思路成熟。

本设计继承：

- 分类错误驱动 fallback。
- primary + fallbacks 配置。
- “不可恢复错误立即返回”原则。

### 7.3 与本设计的主要差异

#### 差异 1：事件流对齐更明确

本设计直接约束 `LlmResponse -> AssistantMessage/ToolCallRequest` 映射。

理由：agent-demo 正在推进事件流模型，

LLM 抽象必须天然对齐事件类型。

#### 差异 2：Layer 边界更严格

本设计明确：

- Trait 在 Layer 3。
- Adapter 在 Layer 4。
- 编排层仅注入 `Arc<dyn LlmProvider>`。

理由：与 `architecture.md` 依赖规则一致。

#### 差异 3：请求模型包含 metadata

`LlmRequestMetadata` 增加 trace/session/turn 语义。

理由：便于可观测性与跨层诊断，不影响 provider 纯调用。

#### 差异 4：finish_reason 与 cache usage 显式化

本设计扩展 `FinishReason` 与 `LlmUsage` cache 字段。

理由：为 prompt cache 成本分析与策略优化留接口。

### 7.4 为什么不是“更简单单接口”

如果保持单 `generate()`：

- stream 能力会变成临时 hack。
- fallback 决策信息不足。
- provider 差异会回流到编排层。

因此本设计采用“轻量但完整”的接口集：

- `complete`（必做）
- `stream`（可选）
- `LlmError` 分类
- 路由 + fallback 配置

### 7.5 为什么不是“过度平台化”

本设计避免引入：

- 复杂策略引擎 DSL
- 动态插件系统
- 多级调度中心

原因：

- 当前是 MVP + 单体。
- 优先保证接口稳定与实现可落地。
- 保留演进空间但不提前复杂化。

---

## 8. 实施建议（非强制）

### 8.1 迭代顺序

1. 在 `agent-domain` 定义新版 trait 与类型。
2. 在 `agent-llm` 实现 OpenAI adapter 适配新版 trait。
3. 引入 provider registry + route 选择。
4. 引入 fallback chain（先 complete 路径）。
5. 再实现 stream（若业务需要）。

### 8.2 兼容策略

过渡期可保留旧接口：

- `generate(req)` 内部调用 `complete(req)`。

待编排层全部迁移后删除。

### 8.3 测试建议

应覆盖：

- 请求/响应映射单测。
- 错误分类单测。
- retry/backoff 单测。
- fallback 主备切换单测。
- 事件映射单测（AssistantMessage / ToolCallRequest）。

---

## 9. 验收对照

- [x] 包含 7 个章节（1~7）。
- [x] LlmProvider trait 完整 Rust 伪代码。
- [x] 请求/响应类型 >= 6 个 struct/enum。
- [x] 包含 ToolSpec / ToolCall / Usage / FinishReason。
- [x] 包含配置、运行时选择、fallback 流程。
- [x] 包含错误分类、fallback 触发、重试策略。
- [x] 包含与其他层关系与事件模型映射。
- [x] 包含 ZeroClaw 对比与设计理由。

---

## 10. 结论

该设计在 MVP 复杂度内提供了：

- provider-agnostic 调用抽象
- 多 provider 配置与运行时路由
- 可解释的 fallback + retry 机制
- 与事件流模型一致的数据映射

可以作为 `agent-llm` 域下一阶段实现基线。
