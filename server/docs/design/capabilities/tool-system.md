# D-CAP-02 工具系统设计（Tool System）

- **文档 ID**: D-CAP-02
- **所属层**: Layer 3 Capability
- **所属域**: `agent-tools`
- **状态**: Draft（可进入实现阶段）
- **最后更新**: 2026-02-24
- **相关文档**:
  - `docs/design/architecture.md`
  - `docs/design/core/event-model.md`
  - `docs/design/capabilities/llm-provider.md`
  - 参考实现：`~/code/references/zeroclaw/src/tools/`、`~/code/references/picoclaw/pkg/tools/`

---

## 0. 范围与非目标

### 0.1 范围

本设计定义 Layer 3 `agent-tools` 域的核心抽象与运行时职责，包括：

1. 工具描述与调用接口（Tool trait）
2. 工具运行时（ToolRuntime trait）
3. 工具注册、发现、执行的标准流程
4. 工具结果与错误封装模型
5. 与编排层、LLM Provider、事件模型、基础设施层的协作边界

### 0.2 非目标

以下内容不在本设计范围内：

1. 具体某个工具（如 web_search、db_query）的业务实现细节
2. 多租户权限系统的完整策略定义（只定义挂接点）
3. 观测平台（Tracing/Metrics）具体产品选型
4. 前端工具管理 UI

---

## 1. 设计目标

### 1.1 统一的工具抽象

目标：**内置工具与扩展工具共用同一接口**。

- 对编排层（TurnExecutor）而言，不需要关心工具来自哪里。
- 对 LLM Provider 而言，只看到统一的 `ToolSpec` 列表。
- 对事件模型而言，只记录标准的请求/结果事件，不暴露内部差异。

### 1.2 工具注册与发现

目标：支持运行时动态注册，按需列出可用工具。

- 启动期：注册内置工具 + 基础设施适配工具。
- 运行期：允许按 tenant / channel / policy 过滤。
- 调用期：`list_specs()` 只返回当前 turn 可见的工具规格。

### 1.3 执行隔离

目标：单个工具失败不会中断整轮流程。

- 每个 ToolCall 独立执行与独立结果封装。
- 失败、超时、取消都转化为结构化 `ToolResult`。
- TurnExecutor 将结果回填给 LLM，让模型自行恢复或降级回答。

### 1.4 与 architecture 原则对齐

对齐 `architecture.md` 中的核心原则：

- trait-first
- 分层依赖单向
- 工具失败不直接炸穿 turn，而是**封装为可解释结果回给模型**

---

## 2. Tool Trait

> 对齐 `architecture.md` 附录 E.4：
>
> - `spec() -> ToolSpec`
> - `execute(input) -> Result<ToolOutput, ToolError>`

### 2.1 设计说明

`Tool` 是最小可执行单元，职责边界：

1. 提供对 LLM 可见的工具规格（名称、描述、参数 Schema）
2. 执行一次调用，返回成功输出或失败错误

`Tool` **不负责**：

- 工具集合管理（由 ToolRuntime 负责）
- 事件落盘（由 TurnExecutor/事件管道负责）
- 多轮循环控制（由 TurnExecutor 负责）

### 2.2 Rust 伪代码

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Tool: Send + Sync {
    /// 返回给 LLM 的工具定义
    fn spec(&self) -> ToolSpec;

    /// 执行一次工具调用
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError>;
}
```

### 2.3 ToolInput / ToolOutput 约束

- `ToolInput` 至少包含：
  - `request_id`（与 ToolCallRequest 关联）
  - `tool_name`
  - `arguments_json`
  - `deadline`/`timeout_ms`（可选）
  - `call_context`（tenant/session/user/trace）
- `ToolOutput` 应可序列化为 JSON 字符串，供事件与 LLM 回填使用

### 2.4 示例（内置工具）

```rust
pub struct GetCurrentTimeTool;

#[async_trait]
impl Tool for GetCurrentTimeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "get_current_time".to_string(),
            description: "Return current time in ISO-8601".to_string(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "timezone": { "type": "string" }
                },
                "required": []
            }),
            strict: false,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, ToolError> {
        let _args: serde_json::Value = serde_json::from_str(&input.arguments_json)
            .map_err(|e| ToolError::invalid_arguments("invalid_json", e.to_string()))?;

        Ok(ToolOutput {
            content_json: serde_json::json!({
                "now": "2026-02-24T20:00:00+08:00"
            }).to_string(),
            metadata: Default::default(),
        })
    }
}
```

---

## 3. ToolRuntime Trait

> 对齐 `architecture.md` 附录 E.4：
>
> - `register`
> - `list_specs`
> - `execute_calls`

### 3.1 设计说明

`ToolRuntime` 是工具系统的编排内核（在 Layer 3），职责：

1. 管理工具注册表
2. 按上下文给出当前可见工具规格
3. 批量执行一轮 ToolCall 并返回结构化 ToolResult

### 3.2 Rust 伪代码（接口）

```rust
use async_trait::async_trait;

#[async_trait]
pub trait ToolRuntime: Send + Sync {
    /// 注册一个工具（启动期或热加载）
    fn register(&mut self, tool: Box<dyn Tool>);

    /// 列出当前上下文可见的工具规格（用于 LLM request.tool_specs）
    fn list_specs(&self, ctx: &ToolListContext) -> Vec<ToolSpec>;

    /// 执行一轮中所有 tool calls，逐个隔离并返回结果
    async fn execute_calls(
        &self,
        ctx: &ToolExecContext,
        calls: Vec<ToolCall>,
    ) -> Vec<ToolResult>;
}
```

### 3.3 Rust 伪代码（一个默认运行时）

```rust
use std::collections::HashMap;
use std::sync::Arc;

pub struct DefaultToolRuntime {
    registry: HashMap<String, Arc<dyn Tool>>,
    policy: Arc<dyn ToolPolicy>,
    timeout: Arc<dyn ToolTimeoutPolicy>,
}

#[async_trait]
impl ToolRuntime for DefaultToolRuntime {
    fn register(&mut self, tool: Box<dyn Tool>) {
        let spec = tool.spec();
        self.registry.insert(spec.name.clone(), Arc::from(tool));
    }

    fn list_specs(&self, ctx: &ToolListContext) -> Vec<ToolSpec> {
        self.registry
            .values()
            .map(|t| t.spec())
            .filter(|spec| self.policy.allow(spec, ctx))
            .collect()
    }

    async fn execute_calls(
        &self,
        ctx: &ToolExecContext,
        calls: Vec<ToolCall>,
    ) -> Vec<ToolResult> {
        let mut out = Vec::with_capacity(calls.len());

        for call in calls {
            let started_at = std::time::Instant::now();

            let result = match self.registry.get(&call.name) {
                None => ToolResult::framework_error(
                    &call,
                    "TOOL_NOT_FOUND",
                    format!("tool '{}' is not registered", call.name),
                    started_at.elapsed(),
                ),
                Some(tool) => {
                    let input = ToolInput::from_call(ctx, &call);
                    let deadline = self.timeout.deadline_for(&call, ctx);

                    match with_timeout(deadline, tool.execute(input)).await {
                        TimeoutOutcome::Ok(Ok(output)) => {
                            ToolResult::success(&call, output.content_json, started_at.elapsed())
                        }
                        TimeoutOutcome::Ok(Err(err)) => {
                            ToolResult::from_tool_error(&call, err, started_at.elapsed())
                        }
                        TimeoutOutcome::TimedOut => {
                            ToolResult::timeout(&call, started_at.elapsed())
                        }
                    }
                }
            };

            out.push(result);
        }

        out
    }
}
```

### 3.4 为什么是 `execute_calls` 而不是 `execute_one`

- 与 LLM 响应形态一致：一轮 assistant 可能有多个 tool calls
- 便于统一做：批量限流、并发窗口、观测聚合
- 保持 TurnExecutor 接口简洁

---

## 4. 类型定义（对齐现有文档，引用优先）

> 关键要求：**与 `llm-provider.md`、`event-model.md` 对齐，不重复发明语义**。

### 4.1 ToolSpec（引用 llm-provider）

`ToolSpec` 直接采用 `llm-provider.md` 已定义模型：

```rust
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub strict: bool,
    pub execution_class: ExecutionClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionClass {
    #[default]
    Local,
    Remote,
    ProviderBuiltin,
    Client,
}
```

对齐点：

- 字段名与含义保持一致
- 由 ToolRuntime.list_specs 直接提供给 `LlmRequest.tool_specs`
- `agent-tools` 不再单独定义另一套 ToolSpec

### 4.2 ToolCall（引用 llm-provider）

`ToolCall` 采用 `llm-provider.md` 统一模型：

```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}
```

语义：

- `id`：provider 侧调用 ID（映射为 event-model 里的 `request_id`）
- `name`：工具名
- `arguments`：JSON 字符串

### 4.3 ToolCallRequest / ToolCallResult（对齐 event-model）

工具域内部类型与事件模型字段一一映射：

| 工具域逻辑字段 | 事件字段 | 来源 |
|---|---|---|
| call.id | ToolCallRequest.request_id | llm-provider ToolCall |
| provider response id / piece id | ToolCallRequest.provider_call_id | orchestrator 注入 |
| call.name | ToolCallRequest.tool_name | llm-provider ToolCall |
| call.arguments | ToolCallRequest.arguments_json | llm-provider ToolCall |
| timeout policy | ToolCallRequest.timeout_ms | tool runtime policy |
| attempt | ToolCallRequest.attempt | turn loop |

`ToolCallResult` 对齐事件定义：

- `request_id`
- `result_id`
- `status` (`Success|Timeout|Cancelled|Failed`)
- `output_json`
- `error_code`
- `error_message`
- `latency_ms`

### 4.4 ToolInput / ToolOutput（agent-tools 内部类型）

```rust
pub struct ToolInput {
    pub request_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub timeout_ms: Option<u32>,
    pub context: ToolCallContext,
}

pub struct ToolOutput {
    /// 必须是可回填给 LLM 的 JSON 字符串
    pub content_json: String,
    /// 调试/观测附加信息，不直接暴露给 LLM
    pub metadata: std::collections::BTreeMap<String, String>,
}
```

### 4.5 ToolResult（运行时统一产物）

```rust
pub struct ToolResult {
    pub request_id: String,
    pub result_id: String,
    pub status: ToolCallStatus,
    pub output_json: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub latency_ms: u32,
}
```

### 4.6 ToolError（区分工具内部错误 vs 框架级错误）

```rust
pub enum ToolError {
    /// 工具内部错误：参数校验、业务失败、第三方 API 报错
    ToolInternal {
        code: String,
        message: String,
        retryable: bool,
    },

    /// 框架级错误：tool 未注册、runtime 中断、序列化失败
    Framework {
        code: String,
        message: String,
    },

    /// 统一超时
    Timeout {
        timeout_ms: u32,
    },

    /// 上下文取消（session cancel / shutdown）
    Cancelled,
}
```

建议错误码前缀：

- 工具内部：`TOOL_*`（如 `TOOL_INVALID_ARGS`, `TOOL_UPSTREAM_5XX`）
- 框架级：`RUNTIME_*`（如 `RUNTIME_TOOL_NOT_FOUND`, `RUNTIME_SERIALIZATION_ERROR`）

---

## 5. 内置工具 vs 扩展工具

### 5.1 分类原则

#### 内置工具（Built-in）

- 位置：`agent-tools` crate
- 示例：`get_current_time`、`web_search`
- 特征：
  - 通用能力
  - 无强业务耦合
  - 对外依赖可控

#### 扩展工具（Extension）

- 位置：Layer 4 基础设施实现 + Layer 3 适配器
- 示例：数据库查询、企业内部 API、业务系统操作
- 特征：
  - 与具体基础设施强耦合
  - 生命周期由部署环境决定

### 5.2 注册方式

#### 启动期注册

```rust
fn bootstrap_tools(runtime: &mut dyn ToolRuntime, infra: &InfraDeps) {
    // built-in
    runtime.register(Box::new(GetCurrentTimeTool::new()));
    runtime.register(Box::new(WebSearchTool::new(infra.web_client.clone())));

    // extension adapters
    runtime.register(Box::new(DbQueryToolAdapter::new(infra.db_pool.clone())));
    runtime.register(Box::new(CrmApiToolAdapter::new(infra.crm_client.clone())));
}
```

#### 运行期注册（可选）

- 热插拔扩展工具（feature flag / 租户级插件）
- 注册后通过 policy 控制是否对当前 turn 可见

### 5.3 区分而不分裂接口

关键策略：

- 区分仅体现在**实现归属和依赖来源**
- 对 TurnExecutor/LLM 来说，都是 `ToolSpec + ToolCall + ToolResult`
- 避免 “内置一套接口、扩展另一套接口” 的双轨系统

### 5.4 内置工具与执行位置

Provider 内置工具（例如 provider 侧 `web_search`）不经过 `ToolRuntime` 执行链路。

这类工具在 `LlmRequest.builtin_tools` 中声明即可，由 provider 侧自行执行并把结果注入模型上下文。

`ToolSpec.execution_class` 是轻量元数据，主要用于审计、策略检查与可观测性标注：

- `Local`：本服务执行
- `Remote`：外部远端服务执行
- `ProviderBuiltin`：provider 内置执行
- `Client`：客户端执行

注意：编排层不依赖 `execution_class` 做主流程分支。主流程仍以统一 tool loop 为核心，`execution_class` 只作为附加信息。


---

## 6. 工具失败处理

> 对齐 architecture 原则：**工具失败 => 错误封装后回填给 LLM，而不是直接中断整轮**。

### 6.1 失败分类

1. 工具内部失败（ToolInternal）
2. 框架级失败（Framework）
3. 超时（Timeout）
4. 取消（Cancelled）

### 6.2 统一封装流程

```text
LLM 返回 ToolCall
  -> TurnExecutor 产出 ToolCallRequest 事件
  -> ToolRuntime.execute_calls
      -> 单 call 执行
      -> 成功: status=Success + output_json
      -> 失败: status=Failed/Timeout/Cancelled + error_code/error_message
  -> TurnExecutor 产出 ToolCallResult 事件
  -> ContextBuilder 将 ToolCallResult 映射为 tool message
  -> 再次调用 LLM，让模型基于“失败信息”继续回答/降级
```

### 6.3 回填给 LLM 的建议格式

为降低 provider 差异，建议 tool message 的 content 使用统一 JSON：

```json
{
  "request_id": "call_123",
  "ok": false,
  "status": "Failed",
  "error": {
    "code": "TOOL_UPSTREAM_5XX",
    "message": "weather service temporarily unavailable",
    "retryable": true
  }
}
```

成功时：

```json
{
  "request_id": "call_123",
  "ok": true,
  "status": "Success",
  "data": {
    "temperature": 18,
    "condition": "cloudy"
  }
}
```

### 6.4 超时策略

#### 超时来源

- 全局默认超时（如 8s）
- 工具级覆盖（如 web_search 15s）
- call 级动态覆盖（由策略引擎决定）

#### 超时行为

- `ToolCallResult.status = Timeout`
- `error_code = "RUNTIME_TIMEOUT"`
- `error_message` 包含 timeout 毫秒
- `output_json = None`

#### 超时后的 turn 行为

- 不中断整轮
- 把 timeout 结果回填 LLM
- 由模型决定：重试同工具、换工具、或直接给解释性回答

### 6.5 未注册工具的处理

如果 LLM 请求了未注册工具：

- 生成 `Failed` 结果而非 panic
- `error_code = "RUNTIME_TOOL_NOT_FOUND"`
- 回填 LLM 以便其选择其他工具或转文本说明

### 6.6 错误信息脱敏

- `error_message` 面向模型应简洁可解释，避免泄漏内部凭据/堆栈
- 详细异常写入日志/trace，不直接进入 LLM 可见内容

---

## 7. 与其他层的关系

### 7.1 与编排层（TurnExecutor）

职责边界：

- TurnExecutor：
  - 调用 `list_specs` 构造 LlmRequest
  - 解析 LLM `tool_calls`
  - 记录 ToolCallRequest/Result 事件
  - 控制 tool loop 次数（`max_tool_iterations`）
- ToolRuntime：
  - 注册与发现
  - 执行与错误封装

交互顺序（简化）：

1. `specs = tools.list_specs(ctx)`
2. `llm.complete(messages, specs)`
3. parse `tool_calls`
4. append `ToolCallRequest` events
5. `results = tools.execute_calls(ctx, tool_calls)`
6. append `ToolCallResult` events
7. 将结果注入上下文，再次请求 LLM

### 7.2 与 LLM Provider

- 输入方向：`ToolSpec[]` -> `LlmRequest.tool_specs`
- 输出方向：`LlmResponse.tool_calls` -> `ToolCall[]`
- 规范：`agent-tools` 复用 `llm-provider.md` 的 `ToolSpec/ToolCall` 定义，不再重复

### 7.3 与事件模型（Event Model）

每次工具调用至少产生两类事件：

1. `ToolCallRequest`
2. `ToolCallResult`

字段必须对齐 `event-model.md`（尤其 request_id 因果链）。

### 7.4 与基础设施层

- Layer 4 实现外部依赖 client（DB、HTTP API、队列等）
- Layer 3 通过适配器把这些能力包装成 `Tool`
- 基础设施故障通过 `ToolError::ToolInternal/Framework` 封装，不向上泄漏实现细节

---

## 8. 与参考框架对比（ZeroClaw / PicoClaw）

### 8.1 ZeroClaw

观察到的做法：

- `Tool` trait 提供 `name/description/parameters_schema/execute/spec`
- 返回 `ToolResult { success, output, error }`
- 偏向“工具本体自描述 + 轻量结果模型”

本设计吸收点：

- trait-first 的工具抽象方式
- `spec()` 统一输出定义

本设计差异：

1. 增加 `ToolRuntime.execute_calls` 批量语义（贴合 turn 一轮多 call）
2. 明确 `ToolCallRequest/Result` 事件对齐要求
3. 将错误拆为 ToolInternal / Framework / Timeout / Cancelled
4. 强调失败回填 LLM，而非只在日志层处理

### 8.2 PicoClaw

观察到的做法：

- Go 接口 `Tool` + `ToolRegistry`
- 注册表负责 name -> tool 映射与执行
- 有 async callback、context 注入等工程化增强

本设计吸收点：

- registry 模式
- 运行时注册与发现
- 执行前注入上下文

本设计差异：

1. 不把 provider schema 转换逻辑耦合在 registry 中，保持 Layer 3 边界清晰
2. 结果模型直接对齐 event model（支持 request/result 因果链）
3. 将超时与取消作为一等状态（不是普通 error string）

### 8.3 差异理由总结

`agent-demo` 目标是分层长期演进，不只是工具能跑通，因此需要：

- 与 LLM provider 统一模型对齐
- 与事件溯源模型一致
- 失败可观测且可反馈给模型
- 支持内置与扩展同构演进

---

## 9. 关键流程（端到端）

### 9.1 正常成功路径

```text
[TurnExecutor]
  -> list_specs
  -> LLM complete (returns tool_calls)
  -> append ToolCallRequest events
  -> execute_calls
  -> append ToolCallResult(Success) events
  -> build tool messages
  -> LLM complete again
  -> final assistant response
```

### 9.2 单工具失败路径（不中断）

```text
[tool A success] + [tool B failed]
  -> 两条 ToolCallResult（Success / Failed）
  -> 都进入上下文
  -> LLM 根据失败信息生成降级回答
```

### 9.3 超时路径

```text
call tool
  -> timeout reached
  -> ToolCallResult(status=Timeout, error_code=RUNTIME_TIMEOUT)
  -> 回填 LLM
  -> LLM 可解释“数据服务超时，请稍后重试”
```

---

## 10. 伪代码：TurnExecutor 如何调用 ToolRuntime

```rust
pub async fn run_turn(&self, turn_ctx: TurnContext) -> Result<TurnOutcome, TurnError> {
    let mut iteration = 0;

    loop {
        if iteration >= self.config.max_tool_iterations {
            return Ok(TurnOutcome::tool_iteration_exceeded());
        }

        let tool_specs = self.tools.list_specs(&ToolListContext::from(&turn_ctx));

        let llm_resp = self.llm.complete(LlmRequest {
            messages: self.context.build_messages(&turn_ctx)?,
            tool_specs,
            config: turn_ctx.llm_config.clone(),
            metadata: turn_ctx.llm_meta.clone(),
        }).await?;

        if llm_resp.tool_calls.is_empty() {
            return Ok(TurnOutcome::final_text(llm_resp.content));
        }

        for call in &llm_resp.tool_calls {
            self.events.append_tool_call_request(call, iteration, &turn_ctx).await?;
        }

        let results = self.tools
            .execute_calls(&ToolExecContext::from(&turn_ctx), llm_resp.tool_calls)
            .await;

        for result in &results {
            self.events.append_tool_call_result(result, &turn_ctx).await?;
        }

        self.context.attach_tool_results(&turn_ctx, results).await?;

        iteration += 1;
    }
}
```

---

## 11. 兼容与迁移建议

### 11.1 对现有 `agent-domain` 旧类型的处理

当前 `agent-domain/src/ports.rs` 存在历史 `ToolSpec/ToolCall/ToolRuntime` 草案。建议：

1. 新实现以 `llm-provider.md` 与 `event-model.md` 的定义为准
2. 旧类型标注 deprecated（迁移期适配）
3. 避免并存两套字段名（如 `parameters` vs `parameters_schema`）长期存在

### 11.2 渐进迁移步骤

1. 在 `agent-tools` 引入新 trait 与类型
2. 编排层改为调用 `execute_calls`
3. 事件写入切换到 ToolCallRequest/Result 标准字段
4. 清理旧接口

---

## 12. 可测试性与验收建议

### 12.1 单元测试建议

1. `register + list_specs`：可见性过滤正确
2. `execute_calls`：
   - 全成功
   - 部分失败
   - 未注册工具
   - 超时
3. `ToolError -> ToolResult` 映射稳定

### 12.2 集成测试建议

1. 一轮双工具调用，验证事件序列完整
2. 工具失败后 LLM 能收到失败消息并给出降级回答
3. `max_tool_iterations` 触发时能安全退出

### 12.3 观测建议

指标（建议）：

- `tool_call_total{tool,status}`
- `tool_latency_ms{tool}`
- `tool_timeout_total{tool}`
- `tool_not_found_total`

日志字段（建议）：

- `trace_id`, `session_id`, `turn_id`, `request_id`, `tool_name`, `status`, `latency_ms`

---

## 13. 开放问题（待实现阶段决策）

1. `execute_calls` 默认串行还是受限并发（例如 N=4）？
2. 工具级 timeout 配置放在静态配置还是 registry 元数据？
3. 扩展工具的权限策略由 ToolRuntime 统一判断，还是前置到 Orchestrator？
4. 是否需要异步回调型工具（类似 PicoClaw AsyncTool）进入 v1？

---

## 14. 小结

本设计在 `agent-tools` 域提供了：

1. 统一 Tool / ToolRuntime 抽象
2. 与 `llm-provider`、`event-model` 对齐的类型语义
3. 内置与扩展工具同构注册机制
4. 可落地的失败封装与回填流程
5. 与 TurnExecutor 的清晰协作边界

它满足 D-CAP-02 的核心目标：

- **统一抽象**
- **动态注册发现**
- **执行隔离与失败可恢复**

并为后续 `agent-tools` crate 实现提供了直接可编码的接口蓝图。
