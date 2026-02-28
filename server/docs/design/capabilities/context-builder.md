# ContextBuilder 设计（D-CAP-03）

**状态**：Draft（MVP 可实施）

**更新时间**：2026-02-24

**所属层**：Layer 3 Capability / `agent-context`

**关联文档**：

- `docs/design/architecture.md`
- `docs/design/core/event-model.md`
- `docs/design/capabilities/llm-provider.md`
- `docs/design/archive/core/context-management.md`（历史参考）
- `~/code/references/zeroclaw/src/agent/prompt.rs`
- `~/code/references/picoclaw/pkg/agent/context.go`

---

## 1. 设计目标

本设计定义 `agent-context` 域中 ContextBuilder 的职责、接口、策略和与上下游的契约。

ContextBuilder 是把“事件流”转换为“LLM 输入”的唯一能力模块。

它的输出质量与稳定性，直接影响：

- 回复质量
- 工具循环稳定性
- token 成本
- prompt cache 命中率

### 1.1 核心目标一：独立于编排层

ContextBuilder 必须是 Layer 3 独立模块。

ContextBuilder 不嵌入 TurnExecutor。

TurnExecutor 只做流程调度，不做 prompt 组装细节。

边界约束：

- TurnExecutor 调用 `ContextBuilder` 接口
- TurnExecutor 不直接拼接 system prompt
- TurnExecutor 不直接从事件流手写筛选历史
- ContextBuilder 不直接调用 channel/infrastructure 细节

收益：

- 编排复杂度下降
- 上下文策略可独立测试
- 后续可替换实现（规则版/学习版）

### 1.2 核心目标二：可插拔 System Prompt

System Prompt 必须采用 section 组合模式。

不采用单一巨大字符串模板。

通过 `PromptSection` trait 实现可插拔与可扩展。

最小要求：

- section 具名（`name()`）
- section 可独立渲染（`build(ctx)`）
- section 顺序可配置
- section 可启用/禁用

收益：

- 变更粒度小
- 评审更可控
- A/B 更容易

### 1.3 核心目标三：Prompt Cache 友好

System Prompt 在 session 首轮构建后尽量不变。

稳定前缀带来更高 cache 命中。

动态内容不应频繁污染前缀。

需要实现：

- 缓存 key 与版本机制
- `ConfigChange` 触发重建
- 稳定段与动态段拆分

### 1.4 核心目标四：Token 预算感知

ContextBuilder 必须在预算内选择最有价值历史。

不是“全量历史拼进去”。

预算分配明确化：

- system prompt 预算
- 历史预算
- 当前用户消息预算
- 回复预留预算（由编排层提供）

### 1.5 设计原则汇总

1. 单一职责：ContextBuilder 只负责上下文组装。
2. 稳定优先：优先稳定 system prompt。
3. 价值优先：历史按价值而非按数量选取。
4. 可解释：每次裁剪可追溯。
5. 渐进演进：MVP 先规则，后续可智能化。

### 1.6 非目标（本阶段不做）

1. 不定义数据库 schema 迁移。
2. 不定义 MemoryProvider 检索算法细节。
3. 不实现自动学习型摘要质量评分。
4. 不在本文实现代码。

---

## 2. PromptSection Trait

本章定义 System Prompt 组装的最小抽象。

### 2.1 设计动机

来自 ZeroClaw 的核心经验：

- 使用 `PromptSection` trait 分解 prompt
- 使用 `SystemPromptBuilder` 顺序组装
- 每段可独立测试

本设计保留该模式。

并增加：

- Section 元数据（稳定性、缓存策略）
- Section 启用条件
- Section 输出可为空

### 2.2 PromptSection Trait（Rust 伪代码）

```rust
use std::borrow::Cow;

pub trait PromptSection: Send + Sync {
    /// section 唯一名称（用于日志、审计、缓存 key）
    fn name(&self) -> &'static str;

    /// 构建 section 文本；允许返回空字符串表示跳过
    fn build(&self, ctx: &PromptSectionContext) -> Result<String, ContextError>;

    /// 是否稳定：稳定 section 应尽量保持不变，利于 prompt cache
    fn is_stable(&self) -> bool {
        true
    }

    /// section 顺序权重（值越小越靠前）
    fn order(&self) -> u16 {
        100
    }

    /// 是否启用（可基于运行时配置决定）
    fn enabled(&self, _ctx: &PromptSectionContext) -> bool {
        true
    }
}
```

### 2.3 Section 上下文模型（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct PromptSectionContext {
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub session_id: String,

    /// 运行时配置快照（已应用最近 ConfigChange）
    pub config: EffectiveConfig,

    /// 工具摘要（非完整 schema）
    pub tool_briefs: Vec<ToolBrief>,

    /// 可用技能摘要
    pub skill_briefs: Vec<SkillBrief>,

    /// 主机/运行时信息
    pub runtime: RuntimeInfo,

    /// 当前时间信息（动态）
    pub now: DateTimeInfo,
}
```

### 2.4 内置 Section 列表（MVP）

至少内置以下 section：

1. `IdentitySection`
2. `SafetySection`
3. `ToolsSection`
4. `DateTimeSection`
5. `RuntimeSection`
6. `SkillsSection`

可选：

7. `MemoryHintSection`（未来）
8. `SessionPolicySection`

### 2.5 各内置 Section 语义

#### 2.5.1 IdentitySection

职责：

- 注入 agent 身份、角色边界、风格基线。

特点：

- 稳定
- 高优先级
- 强约束

#### 2.5.2 SafetySection

职责：

- 注入安全边界与禁止行为。

特点：

- 稳定
- 必选

#### 2.5.3 ToolsSection

职责：

- 注入可用工具能力摘要与使用原则。

特点：

- 准稳定（工具集合变更才变）
- 体积受控

#### 2.5.4 DateTimeSection

职责：

- 注入当前日期时间。

特点：

- 动态
- 不应污染稳定前缀

#### 2.5.5 RuntimeSection

职责：

- 注入运行环境（OS/模型/工作目录）相关上下文。

特点：

- 低频变化

#### 2.5.6 SkillsSection

职责：

- 注入技能目录摘要，指导模型按需读取技能文档。

特点：

- 低频变化
- 常驻但可压缩

### 2.6 Section 输出组合器（Rust 伪代码）

```rust
pub struct SystemPromptComposer {
    sections: Vec<Box<dyn PromptSection>>,
}

impl SystemPromptComposer {
    pub fn new(sections: Vec<Box<dyn PromptSection>>) -> Self {
        Self { sections }
    }

    pub fn compose(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        let mut ordered: Vec<&Box<dyn PromptSection>> =
            self.sections.iter().filter(|s| s.enabled(ctx)).collect();

        ordered.sort_by_key(|s| s.order());

        let mut out = String::new();
        for section in ordered {
            let content = section.build(ctx)?;
            if content.trim().is_empty() {
                continue;
            }
            out.push_str(content.trim_end());
            out.push_str("\n\n");
        }
        Ok(out)
    }
}
```

### 2.7 自定义 Section 扩展点

扩展方式：

1. 实现 `PromptSection` trait。
2. 在 `ContextBuilderFactory` 注册。
3. 配置 order 与 enabled 规则。

示例场景：

- 租户自定义规范 section
- 行业模板 section（医疗/法律）
- 记忆增强 section（未来）

扩展约束：

- 禁止直接访问数据库
- 禁止执行耗时 I/O
- 必须可在毫秒级构建

### 2.8 Section 级别缓存建议

可对稳定 section 做片段缓存：

- key：`tenant + agent + section_name + config_hash`
- value：section 文本

注意：

- DateTimeSection 默认不缓存
- ToolsSection 在 toolset hash 变化时失效

---

## 3. ContextBuilder Trait

本章定义 ContextBuilder 的主接口与输入输出模型。

### 3.1 角色定位

ContextBuilder 对外暴露两类能力：

1. 构建 system prompt
2. 构建历史 messages

最终组合为 ContextOutput。

### 3.2 ContextBuilder Trait（Rust 伪代码）

```rust
pub trait ContextBuilder: Send + Sync {
    /// 只负责 system prompt 组装（可缓存）
    fn build_system_prompt(&self, ctx: &PromptSectionContext) -> Result<String, ContextError>;

    /// 从事件流选择并映射历史消息
    fn build_messages(&self, input: &ContextInput) -> Result<Vec<ModelMessage>, ContextError>;

    /// 便捷接口：一次返回完整输入
    fn build(&self, input: &ContextInput) -> Result<ContextOutput, ContextError> {
        let system_prompt = self.build_system_prompt(&input.prompt_ctx)?;
        let messages = self.build_messages(input)?;
        Ok(ContextOutput {
            system_prompt,
            messages,
            diagnostics: ContextDiagnostics::default(),
        })
    }
}
```

### 3.3 ContextInput（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct ContextInput {
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub session_id: String,

    /// 当前 session 事件流（按 sequence_number 升序）
    pub events: Vec<EventEnvelope>,

    /// 当前用户新消息（通常也已写入事件流）
    pub current_user_message: Option<ModelMessage>,

    /// PromptSection 构建上下文
    pub prompt_ctx: PromptSectionContext,

    /// token 预算
    pub token_budget: TokenBudget,

    /// 历史窗口策略
    pub history_policy: HistoryPolicy,

    /// 映射策略
    pub mapping_policy: MappingPolicy,
}
```

### 3.4 ContextOutput（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct ContextOutput {
    pub system_prompt: String,
    pub messages: Vec<ModelMessage>,
    pub diagnostics: ContextDiagnostics,
}

#[derive(Debug, Clone, Default)]
pub struct ContextDiagnostics {
    pub used_summary_event_id: Option<String>,
    pub dropped_event_ids: Vec<String>,
    pub estimated_prompt_tokens: Option<u32>,
    pub estimated_history_tokens: Option<u32>,
    pub budget_exceeded: bool,
    pub rebuild_system_prompt: bool,
}
```

### 3.5 TokenBudget（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub model_context_limit: u32,
    pub reserved_for_completion: u32,
    pub reserved_for_tool_loop: u32,
    pub reserved_for_system_prompt: u32,
    pub reserved_for_current_user: u32,
}

impl TokenBudget {
    pub fn available_for_input(&self) -> u32 {
        self.model_context_limit
            .saturating_sub(self.reserved_for_completion)
            .saturating_sub(self.reserved_for_tool_loop)
    }

    pub fn available_for_history(&self, system_used: u32, user_used: u32) -> u32 {
        self.available_for_input()
            .saturating_sub(system_used)
            .saturating_sub(user_used)
    }
}
```

### 3.6 HistoryPolicy（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct HistoryPolicy {
    pub mode: HistoryWindowMode,
    pub max_recent_messages: usize,
    pub include_tool_events: bool,
    pub include_summary: bool,
}

#[derive(Debug, Clone)]
pub enum HistoryWindowMode {
    FixedN,
    TokenAware,
    Hybrid,
}
```

### 3.7 MappingPolicy（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct MappingPolicy {
    pub summary_role: SummaryRole,
    pub include_system_events: bool,
    pub drop_orphan_tool_results: bool,
}

#[derive(Debug, Clone)]
pub enum SummaryRole {
    System,
    Assistant,
}
```

### 3.8 默认实现骨架（Rust 伪代码）

```rust
pub struct DefaultContextBuilder {
    composer: SystemPromptComposer,
    estimator: Arc<dyn TokenEstimator>,
    cache: Arc<dyn SystemPromptCache>,
}

impl ContextBuilder for DefaultContextBuilder {
    fn build_system_prompt(&self, ctx: &PromptSectionContext) -> Result<String, ContextError> {
        // 1) 尝试缓存
        // 2) miss 时 compose
        // 3) 写入缓存
        unimplemented!()
    }

    fn build_messages(&self, input: &ContextInput) -> Result<Vec<ModelMessage>, ContextError> {
        // 1) 选 Summary
        // 2) 选 recent history
        // 3) 映射事件 -> ModelMessage
        // 4) token 裁剪
        unimplemented!()
    }
}
```

---

## 4. System Prompt 构建策略

本章定义 section 顺序、缓存与动态内容处理。

### 4.1 默认执行顺序

推荐默认顺序：

1. Identity
2. Safety
3. Tools
4. Skills
5. Runtime
6. DateTime

排序理由：

- 先“身份+安全”建立最高优先级约束
- 再“能力描述”补充工具与技能
- 最后放“运行时+时间”等动态信息

### 4.2 组合方式

组合规则：

- section 之间用双换行分隔
- 空 section 跳过
- 输出统一 trim_end

建议附带 section header（可选）：

- `## Identity`
- `## Safety`
- `## Tools`

便于审计和 diff。

### 4.3 首轮构建与复用

策略：

- session 首轮构建完整 system prompt
- 后续 turn 优先复用缓存版本
- 非变更场景不重建

缓存 key 建议：

`tenant_id + agent_id + session_id + system_prompt_version`

其中 `system_prompt_version` 来源：

- 初始配置 hash
- 最近 ConfigChange after_hash

### 4.4 ConfigChange 触发重建

当事件流出现有效 `ConfigChange` 且 hash 变化：

- 标记 cache miss
- 重建 system prompt
- 写入新版本缓存

不触发重建的事件：

- UserMessage
- AssistantMessage
- ToolCallRequest/Result
- Summary

### 4.5 Prompt Cache 关系

与 provider prompt cache 的关系：

- 稳定 system prompt 前缀 -> 更高 cache hit
- 每轮只变化 history 尾部 -> 复用效率更好

实践建议：

- 避免把时间戳放在最前段
- 避免每轮插入随机字符串
- 避免在稳定段中写 turn 序号

### 4.6 动态内容处理策略

动态内容（如时间）处理建议：

方案 A（推荐）：

- 将 DateTimeSection 放末尾
- 且该 section 体积极小

方案 B：

- 不放 system prompt
- 改为一条 lightweight system/history 消息注入

MVP 采用方案 A。

### 4.7 稳定段与动态段拆分（Rust 伪代码）

```rust
pub struct SplitPrompt {
    pub stable_prefix: String,
    pub dynamic_suffix: String,
}

fn build_split_prompt(composer: &SystemPromptComposer, ctx: &PromptSectionContext) -> Result<SplitPrompt, ContextError> {
    let mut stable = String::new();
    let mut dynamic = String::new();

    for section in composer.sections() {
        if !section.enabled(ctx) { continue; }
        let text = section.build(ctx)?;
        if text.trim().is_empty() { continue; }

        if section.is_stable() {
            stable.push_str(text.trim_end());
            stable.push_str("\n\n");
        } else {
            dynamic.push_str(text.trim_end());
            dynamic.push_str("\n\n");
        }
    }

    Ok(SplitPrompt { stable_prefix: stable, dynamic_suffix: dynamic })
}
```

### 4.8 系统提示构建失败兜底

当某个非关键 section 构建失败：

- 记录告警
- 跳过该 section
- 不阻断整轮

当关键 section（Identity/Safety）失败：

- 返回 ContextError
- 由编排层决定 fail fast 或使用上次缓存

---

## 5. 历史消息选取策略

本章定义从事件流选取与映射历史的规则。

### 5.1 参与历史的事件类型

应纳入候选集：

1. `UserMessage`
2. `AssistantMessage`
3. `ToolCallRequest`
4. `ToolCallResult`
5. `Summary`

默认不纳入：

6. `SystemEvent`
7. `ConfigChange`
8. `CompactionMarker`

例外：

- 某些关键 `SystemEvent` 可作为诊断，不进入模型输入。

### 5.2 事件到 ModelMessage 映射规则

#### 5.2.1 UserMessage -> user

- role: `user`
- content: 用户文本或 parts

#### 5.2.2 AssistantMessage -> assistant

- role: `assistant`
- content: assistant 文本

#### 5.2.3 ToolCallRequest -> assistant(tool_calls)

- role: `assistant`
- content: 可为空字符串
- tool_calls: 按 provider-neutral `ToolCall` 填充

#### 5.2.4 ToolCallResult -> tool

- role: `tool`
- tool_call_id: 对应请求 id
- content: 工具结果文本/json 字符串

#### 5.2.5 Summary -> system/assistant

- role: 由配置决定（默认 `system`）
- content: 摘要正文

### 5.3 映射伪代码

```rust
fn map_event_to_message(ev: &EventEnvelope, policy: &MappingPolicy) -> Option<ModelMessage> {
    match &ev.payload {
        EventPayload::UserMessage(e) => Some(ModelMessage::user_text(&e.text)),

        EventPayload::AssistantMessage(e) => {
            Some(ModelMessage::assistant_text(&e.text))
        }

        EventPayload::ToolCallRequest(e) => {
            Some(ModelMessage {
                role: MessageRole::Assistant,
                content: MessageContent::Text(String::new()),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    id: e.request_id.clone(),
                    name: e.tool_name.clone(),
                    arguments: e.arguments_json.clone(),
                }],
                name: None,
            })
        }

        EventPayload::ToolCallResult(e) => {
            Some(ModelMessage {
                role: MessageRole::Tool,
                content: MessageContent::Text(e.output_json.clone().unwrap_or_default()),
                tool_call_id: Some(e.request_id.clone()),
                tool_calls: vec![],
                name: None,
            })
        }

        EventPayload::Summary(e) => {
            let role = match policy.summary_role {
                SummaryRole::System => MessageRole::System,
                SummaryRole::Assistant => MessageRole::Assistant,
            };
            Some(ModelMessage {
                role,
                content: MessageContent::Text(e.summary_text.clone()),
                tool_call_id: None,
                tool_calls: vec![],
                name: Some("summary".to_string()),
            })
        }

        _ => None,
    }
}
```

### 5.4 Token 预算分配策略

定义：

- `L` = `model_context_limit`
- `Rc` = `reserved_for_completion`
- `Rt` = `reserved_for_tool_loop`
- `Bs` = system prompt 实际 token
- `Bu` = 当前用户消息 token

则历史预算：

`Bh = L - Rc - Rt - Bs - Bu`

约束：

- 若 `Bh <= 0`，仅保留系统与当前用户
- Summary 视为 history 优先级最高项

### 5.5 预算执行顺序

1. 先估算 system prompt token（Bs）
2. 估算当前用户消息 token（Bu）
3. 计算 `Bh`
4. 在 `Bh` 内装配 Summary + 历史

### 5.6 历史窗口策略

支持三种模式：

1. Fixed-N
2. TokenAware
3. Hybrid

#### 5.6.1 Fixed-N

- 从尾部回溯最近 N 条可映射消息
- 再做一次 token 检查

适用：

- MVP
- 实现简单

#### 5.6.2 TokenAware

- 从尾部回溯按 token 累加
- 达到预算即停止

适用：

- 消息长度波动大的会话

#### 5.6.3 Hybrid（推荐默认）

- 先取最近 N 条
- 若超预算再 token 裁剪

### 5.7 工具调用完整性规则

为避免工具上下文断裂：

- 保留 ToolCallRequest 时应尽量保留对应 ToolCallResult
- 保留 ToolCallResult 时应保证前方可找到 request
- orphan result 默认丢弃

### 5.8 历史选取伪代码

```rust
fn select_history(events: &[EventEnvelope], budget: u32, policy: &HistoryPolicy) -> Vec<EventEnvelope> {
    let candidates = collect_mappable(events, policy);

    match policy.mode {
        HistoryWindowMode::FixedN => take_last_n(candidates, policy.max_recent_messages),
        HistoryWindowMode::TokenAware => take_by_token_from_tail(candidates, budget),
        HistoryWindowMode::Hybrid => {
            let recent = take_last_n(candidates, policy.max_recent_messages);
            trim_by_token(recent, budget)
        }
    }
}
```

---

## 6. 压缩后的上下文组装

本章定义 Summary 与 CompactionMarker 存在时的组装规则。

### 6.1 目标

当会话已压缩：

- 使用摘要代表旧历史
- 避免重复注入被覆盖区间原始事件
- 保留近期细节消息

### 6.2 Summary 的放置位置

Summary 放在历史消息开头。

推荐顺序：

1. system prompt（独立字段，不在 messages 里）
2. summary message（若存在）
3. recent raw history
4. current user message（通常由编排层追加）

### 6.3 CompactionMarker 的处理

`CompactionMarker` 不映射为 ModelMessage。

它只用于筛选边界。

规则：

- marker 指定 `[start_seq, end_seq]` 被 summary 替代
- ContextBuilder 构建时跳过该区间原始事件

### 6.4 多个 Summary 的选择

当存在多条 Summary：

默认选择“最新且覆盖范围最大”的一条。

简化规则：

- 先按 `source_range_end_seq` 降序
- 再按 `timestamp` 降序
- 取第一条

### 6.5 压缩场景组装算法（伪代码）

```rust
fn assemble_with_compaction(events: &[EventEnvelope], policy: &MappingPolicy) -> Vec<ModelMessage> {
    let mut out = Vec::new();

    let selected_summary = pick_latest_summary(events);

    let mut replaced_range: Option<(u64, u64)> = None;
    if let Some(summary) = &selected_summary {
        out.push(map_summary(summary, policy));
        replaced_range = Some((summary.source_range_start_seq, summary.source_range_end_seq));
    }

    let recent_events = events.iter()
        .filter(|e| is_mappable(&e.payload))
        .filter(|e| !in_replaced_range(e.meta.sequence_number, replaced_range))
        .collect::<Vec<_>>();

    out.extend(map_events(recent_events, policy));
    out
}
```

### 6.6 边界案例

#### 案例 A：有 Summary，无 Marker

处理：

- 仍可使用 summary
- 但不主动删除旧事件
- 由 token 裁剪兜底

#### 案例 B：有 Marker，无 Summary

处理：

- marker 视为不完整压缩
- 忽略 marker
- 走普通历史选取

#### 案例 C：Summary 范围重叠

处理：

- 仅使用最新摘要
- 其余摘要不注入

### 6.8 Compaction 分层

`CompactionService` 可以有两类实现，并通过同一接口对上层暴露：

1. `ProviderCompaction`：委托 provider 原生 compaction 能力
2. `SelfCompaction`：服务端自建 summary（规则/模型均可）

同一 turn 内两种路径互斥，只能选择其中一条，避免重复压缩或冲突摘要。

为保证审计与回放可解释性，Summary event 增加 `source` 字段用于区分来源，例如：

- `source = "provider"`
- `source = "self"`

上层 ContextBuilder 仅消费统一 Summary 事件，不关心具体压缩实现细节。

### 6.7 与事件模型一致性

与 `event-model.md` 对齐：

- Summary 是事实事件
- CompactionMarker 是边界事件
- ContextBuilder 基于事件而非 session 可变字段构建

---

## 7. 与其他层的关系

本章明确 ContextBuilder 在四层架构中的协作位置。

### 7.1 与编排层（Layer 2）

TurnExecutor 的调用方式：

1. 收集 ContextInput（事件流 + 配置 +预算）
2. 调用 `ContextBuilder.build(...)`
3. 将结果传给 LlmProvider

约束：

- TurnExecutor 不自己筛选历史
- TurnExecutor 不自己拼 system prompt

### 7.2 与 LLM Provider（Layer 3）

对接方式：

- `ContextOutput.messages` 类型即 `Vec<ModelMessage>`
- 直接作为 `LlmProvider.complete()` 输入的一部分

关系：

- ContextBuilder 负责“选什么”
- LlmProvider 负责“怎么发给具体模型”

### 7.3 与事件模型（Layer 3 domain）

输入来源：

- 事件流 `Vec<EventEnvelope>`

语义依赖：

- 事件类型
- sequence_number
- Summary/Marker 边界

### 7.4 与基础设施层（Layer 4）

ContextBuilder 不直接依赖存储实现。

它只消费上层已提供的数据。

缓存接口可抽象为 trait，由 Layer 4 实现。

### 7.5 与记忆系统（未来）

MemoryProvider 可作为额外 PromptSection 注入。

可能路径：

- `MemoryHintSection`：把记忆摘要注入 system prompt
- 或在 build_messages 阶段追加 memory message block

本期建议：

- 先预留扩展点
- 不强耦合 memory 检索

### 7.6 与配置系统关系

ConfigChange 事件是 ContextBuilder 的重建触发器。

因此配置系统应保证：

- ConfigChange 事件可追溯
- after_hash 稳定

### 7.7 与可观测性关系

ContextBuilder 应输出诊断信息：

- 选了哪个 summary
- 丢弃了哪些事件
- token 估算结果
- 是否触发重建

这些信息用于：

- 成本分析
- 质量问题定位

---

## 8. 与参考框架对比

本章说明采纳点与差异化取舍。

### 8.1 ZeroClaw 采纳点

采纳：

1. PromptSection trait 模式
2. SystemPromptBuilder 组合式构建
3. 内置 section（identity/tools/safety/runtime/datetime）

理由：

- Rust trait 模式天然契合
- 可维护性高
- 易测且可审计

### 8.2 ZeroClaw 差异点

差异一：

- 本设计把 `ConfigChange` 事件作为显式重建触发器
- ZeroClaw 更偏运行时上下文直接读取

差异二：

- 本设计强调 event-stream 到 message 的映射契约
- ZeroClaw 文件更偏 system prompt 构建

差异三：

- 本设计把 token 预算作为 ContextBuilder 一等输入
- 不只是“拼字符串”

### 8.3 PicoClaw 采纳点

采纳：

1. ContextBuilder 模块独立
2. BuildSystemPrompt 与 BuildMessages 分离
3. 在 builder 内做历史清洗（如 tool 消息合法性）

理由：

- 模块边界清晰
- 便于编排层瘦身

### 8.4 PicoClaw 差异点

差异一：

- PicoClaw 主要基于消息历史输入
- 本设计基于事件流输入

差异二：

- PicoClaw 的 summary 是参数传入
- 本设计从事件流中选 summary

差异三：

- 本设计显式处理 CompactionMarker 边界
- PicoClaw 没有对应事件语义

### 8.5 本设计关键取舍

取舍 1：

- 先规则化策略，不引入复杂学习算法

取舍 2：

- 先 Hybrid 窗口（N + token 裁剪），兼顾稳定与成本

取舍 3：

- Summary 默认单条注入，避免提示重复

取舍 4：

- DateTime 保留但后置，尽量减少 cache 破坏

### 8.6 取舍理由总结

这些取舍满足 MVP 三目标：

- 可实现
- 可解释
- 可演进

---

## 附录 A：端到端时序（ContextBuilder 视角）

```text
Channel Handler
  -> TurnExecutor
      -> load session events
      -> resolve effective config
      -> ContextBuilder.build(input)
          -> build_system_prompt(prompt_ctx)
          -> build_messages(input)
      -> LlmProvider.complete(messages + system)
      -> persist assistant/tool events
      -> return output
```

---

## 附录 B：ContextBuilder 构建主流程伪代码

```rust
fn build_context(input: &ContextInput) -> Result<ContextOutput, ContextError> {
    // 1. 构建或读取 system prompt
    let system_prompt = self.build_system_prompt(&input.prompt_ctx)?;

    // 2. 估算 system / current user token
    let system_tokens = estimator.estimate_text(&system_prompt)?;
    let current_user_tokens = estimate_current_user(input.current_user_message.as_ref())?;

    // 3. 计算历史预算
    let history_budget = input.token_budget.available_for_history(system_tokens, current_user_tokens);

    // 4. 组装压缩视图（summary + recent history）
    let history_events = select_history_with_compaction(&input.events, history_budget, &input.history_policy);

    // 5. 事件映射
    let mut messages = map_events_to_messages(history_events, &input.mapping_policy);

    // 6. 二次 token 裁剪
    messages = trim_messages_to_budget(messages, history_budget)?;

    // 7. 输出
    Ok(ContextOutput {
        system_prompt,
        messages,
        diagnostics: ContextDiagnostics {
            estimated_prompt_tokens: Some(system_tokens),
            estimated_history_tokens: Some(estimate_messages_tokens(&messages)?),
            ..Default::default()
        }
    })
}
```

---

## 附录 C：配置建议（MVP 默认值）

- `history.mode = hybrid`
- `history.max_recent_messages = 16`
- `history.include_tool_events = true`
- `history.include_summary = true`
- `mapping.summary_role = system`
- `token.reserved_for_completion = 1024`
- `token.reserved_for_tool_loop = 512`
- `token.reserved_for_system_prompt = 2048`
- `token.reserved_for_current_user = 1024`

---

## 附录 D：评审检查清单

### D.1 接口

- [ ] 是否包含 `PromptSection` trait
- [ ] 是否包含 `ContextBuilder` trait
- [ ] 输入输出模型是否完整

### D.2 策略

- [ ] system prompt 是否可缓存
- [ ] ConfigChange 是否触发重建
- [ ] 历史窗口是否 token 感知

### D.3 映射

- [ ] UserMessage -> user
- [ ] AssistantMessage -> assistant
- [ ] ToolCallRequest -> assistant(tool_calls)
- [ ] ToolCallResult -> tool
- [ ] Summary -> system/assistant

### D.4 压缩

- [ ] Summary 是否位于历史开头
- [ ] CompactionMarker 是否只作边界
- [ ] 被替代区间是否跳过

### D.5 边界

- [ ] TurnExecutor 是否未嵌入上下文细节
- [ ] LlmProvider 是否直接消费 Vec<ModelMessage>
- [ ] Memory 扩展点是否预留

---

## 附录 E：术语

- **ContextBuilder**：将事件流组装成 LLM 输入的能力模块。
- **PromptSection**：system prompt 的可插拔片段。
- **Summary**：压缩后的历史摘要事件。
- **CompactionMarker**：标记被摘要替代区间的事件。
- **ConfigChange**：触发 system prompt 重建的配置变更事件。

---

## 结论

本设计将 ContextBuilder 明确为 Layer 3 独立能力，

并通过：

- PromptSection 可插拔机制
- 事件流到 ModelMessage 的稳定映射
- Token 预算感知的历史选择
- Summary/CompactionMarker 的压缩视图
- Prompt Cache 友好的稳定 system prompt

实现“质量、成本、可维护性”的平衡。

这为后续 TurnExecutor 简化、LLM Provider 统一调用、Memory 能力注入，提供了稳定接口基础。
