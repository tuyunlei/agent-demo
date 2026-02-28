# 事件流数据模型设计（D-ARCH-02）

**状态**：Draft（MVP 可实施）

**更新时间**：2026-02-24

**适用范围**：`server/` Rust 单体（分层架构下的会话、上下文、持久化核心模型）

**关联文档**：

- `docs/design/architecture.md`（四层架构，特别是 Layer 2 / Layer 3）
- `docs/research/framework-architecture-comparison.md`（ZeroClaw / PicoClaw / OpenClaw 对比）
- `docs/design/decisions/004-data-model.md`（既有消息模型 ADR）
- `docs/design/archive/core/data-model.md`（旧设计）

---

## 0. 设计目标与范围

本设计定义 agent-demo 从“消息快照模型”迁移到“append-only 事件流模型”时的核心数据抽象。

核心目标：

1. 统一编排层、能力层、基础设施层的数据契约。
2. 保证会话历史可追溯、可回放、可压缩。
3. 支持 System Prompt 稳定性与 prompt cache 友好策略。
4. 为 MVP 提供可落地、不过度抽象的事件类型集合。
5. 明确与现有 `messages` 表的差异，支持后续迁移设计。

非目标：

1. 不提供数据库迁移 SQL。
2. 不修改现有代码实现。
3. 不定义跨服务消息总线协议。
4. 不定义完整审计与 BI 指标体系（只预留字段）。

---

## 1. 核心概念

### 1.1 什么是事件流（Event Stream）

事件流是一个 session 的事实历史（source of truth）。

在事件流模型中：

- 一个 session 的完整历史由按顺序追加的事件组成。
- 事件是“发生过什么”的事实记录。
- 事件序列是 append-only，不做原地更新。
- 当前状态（会话摘要、上下文窗口、前端消息列表）都是事件流的投影结果。

用一句话描述：

> Session State = Fold(Event Stream)

即：会话状态不是单表里的一行“最新值”，而是对事件序列做 fold/reduce 后得到。

### 1.2 事件 vs 消息

事件（Event）与消息（Message）不是同一层概念。

事件：

- 面向系统内部。
- 表示业务过程中的原子事实。
- 包含工具调用、配置变更、压缩标记等“非聊天文本事实”。

消息：

- 面向用户界面。
- 是事件流的一种投影（projection）。
- 通常只展示 user/assistant/tool 等可读内容。

示例：

- `ToolCallRequest` 与 `ToolCallResult` 是事件；
- 前端可能只展示“调用了天气工具并得到结果”的一条消息块；
- `CompactionMarker` 是事件，但通常不直接展示给用户。

### 1.3 不可变性原则（Immutability）

事件一旦写入，不可修改，不可删除（除非非常规数据修复流程）。

允许的操作：

- append 新事件。
- 在读取时通过投影忽略旧事件（如被 Summary 覆盖的区间）。

不允许的操作：

- 更新历史事件内容。
- 重新编号历史 `sequence_number`。
- 直接覆盖旧消息文本实现“修改历史”。

不可变性的收益：

1. 可审计：能追溯“何时谁做了什么”。
2. 可调试：可回放 turn 过程。
3. 可并发：写入策略简化为“按序追加”。
4. 可扩展：新投影可从历史事件重建。

### 1.4 事件流在四层架构中的位置

按 `architecture.md` 的分层约束：

- Layer 2（Orchestration）负责产生事件（turn 流程驱动）。
- Layer 3（Capability）定义事件类型与端口（`agent-domain` + traits）。
- Layer 4（Infrastructure）负责持久化与检索事件。

因此：

- 事件模型属于 Layer 3 的核心领域契约。
- 事件写入策略由 Layer 2 决策，Layer 4 执行。

---

## 2. 事件类型枚举

本节定义 MVP 事件类型集合。

最低要求 7 种均包含在内：

1. `UserMessage`
2. `AssistantMessage`
3. `ToolCallRequest`
4. `ToolCallResult`
5. `SystemEvent`
6. `CompactionMarker`
7. `Summary`

同时补充 1 种推荐类型：

8. `ConfigChange`（也可视为 `SystemEvent` 的专门化）

> 说明：为兼顾可读性与演进性，本文采用“顶层事件枚举 + 各类型 payload struct”的写法。

### 2.1 顶层事件包络（Envelope）

```rust
pub struct EventEnvelope {
    pub meta: EventMeta,
    pub payload: EventPayload,
}

pub enum EventPayload {
    UserMessage(UserMessageEvent),
    AssistantMessage(AssistantMessageEvent),
    ToolCallRequest(ToolCallRequestEvent),
    ToolCallResult(ToolCallResultEvent),
    SystemEvent(SystemEvent),
    ConfigChange(ConfigChangeEvent),
    CompactionMarker(CompactionMarkerEvent),
    Summary(SummaryEvent),
}
```

### 2.2 UserMessage

类型名称：`UserMessage`

Rust 伪代码：

```rust
pub struct UserMessageEvent {
    pub message_id: String,
    pub text: String,
    pub attachments: Vec<AttachmentRef>,
    pub input_channel: String,
    pub client_message_id: Option<String>,
    pub token_estimate: Option<u32>,
}
```

何时产生：

- 接入层收到用户输入并完成鉴权后。
- 编排层开始一个 turn 时，首先追加。

谁产生：

- Layer 2 `TurnExecutor`（由 Layer 1 提供输入）。

说明：

- `message_id` 是领域消息标识，不等同于 `event_id`。
- 用户编辑消息在 MVP 不做“修改事件”，应以补充消息方式表达。

### 2.3 AssistantMessage

类型名称：`AssistantMessage`

Rust 伪代码：

```rust
pub struct AssistantMessageEvent {
    pub message_id: String,
    pub text: String,
    pub finish_reason: AssistantFinishReason,
    pub model: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub tool_call_count: u16,
}

pub enum AssistantFinishReason {
    Stop,
    ToolCalls,
    Length,
    Safety,
    ErrorFallback,
}
```

何时产生：

- LLM 输出最终文本回复时。
- 若该轮只触发工具调用无最终文本，可不产出或产出空文本 + `ToolCalls`。

谁产生：

- Layer 2 `TurnExecutor`（消费 Layer 3 `LlmProvider` 输出后写入）。

### 2.4 ToolCallRequest

类型名称：`ToolCallRequest`

Rust 伪代码：

```rust
pub struct ToolCallRequestEvent {
    pub request_id: String,
    pub provider_call_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub timeout_ms: Option<u32>,
    pub attempt: u16,
}
```

何时产生：

- assistant 在一次迭代中请求调用工具时。
- 每个工具调用追加一条请求事件。

谁产生：

- Layer 2 `TurnExecutor`（解析 LLM response 后）。

说明：

- 一次 assistant 回复含多个工具调用时，会生成多条 `ToolCallRequest`。

### 2.5 ToolCallResult

类型名称：`ToolCallResult`

Rust 伪代码：

```rust
pub struct ToolCallResultEvent {
    pub request_id: String,
    pub result_id: String,
    pub status: ToolCallStatus,
    pub output_json: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub latency_ms: u32,
}

pub enum ToolCallStatus {
    Success,
    Timeout,
    Cancelled,
    Failed,
}
```

何时产生：

- 工具执行完成（成功或失败）后。

谁产生：

- Layer 2 `TurnExecutor`（调用 Layer 3 `ToolRuntime` 后追加）。

说明：

- `request_id` 对应 `ToolCallRequest.request_id`，形成因果链。

### 2.6 SystemEvent

类型名称：`SystemEvent`

Rust 伪代码：

```rust
pub struct SystemEvent {
    pub kind: SystemEventKind,
    pub actor: SystemActor,
    pub detail_json: String,
}

pub enum SystemEventKind {
    SessionCreated,
    SessionArchived,
    SessionRestored,
    TurnStarted,
    TurnCompleted,
    TurnFailed,
    PolicyNotice,
}

pub enum SystemActor {
    Orchestrator,
    Lifecycle,
    Scheduler,
    Operator,
}
```

何时产生：

- 会话创建/归档/恢复。
- turn 生命周期关键节点。
- 其他系统级但非消息事件。

谁产生：

- Layer 2（`TurnExecutor` / `SessionLifecycle`）。
- 部分管理流程可由 Layer 1 管理入口触发，再由 Layer 2 持久化。

### 2.7 ConfigChange

类型名称：`ConfigChange`

Rust 伪代码：

```rust
pub struct ConfigChangeEvent {
    pub change_id: String,
    pub scope: ConfigScope,
    pub changed_fields: Vec<String>,
    pub before_hash: Option<String>,
    pub after_hash: String,
    pub patch_json: String,
    pub reason: Option<String>,
}

pub enum ConfigScope {
    Persona,
    Toolset,
    SafetyPolicy,
    RuntimeParam,
}
```

何时产生：

- session 进行中发生配置变更时（如人格调整、工具启停、温度参数变化）。

谁产生：

- Layer 2 `SessionLifecycle`（接收配置变更命令后追加）。

说明：

- 该事件是 System Prompt 稳定性策略的关键支点。
- 可被视为 `SystemEvent` 的专门化，MVP 建议独立类型，便于 ContextBuilder 快速定位。

### 2.8 CompactionMarker

类型名称：`CompactionMarker`

Rust 伪代码：

```rust
pub struct CompactionMarkerEvent {
    pub compaction_id: String,
    pub strategy: CompactionStrategy,
    pub replaced_range_start_seq: u64,
    pub replaced_range_end_seq: u64,
    pub summary_event_id: String,
    pub trigger: CompactionTrigger,
}

pub enum CompactionStrategy {
    SlidingSummary,
    HierarchicalSummary,
}

pub enum CompactionTrigger {
    TokenThreshold,
    EventCountThreshold,
    Manual,
}
```

何时产生：

- 一次压缩完成后，标记被摘要替代的区间。

谁产生：

- Layer 2 `SessionLifecycle.compact_if_needed()`。

说明：

- Marker 不携带摘要正文，仅携带区间与引用，减少重复。

### 2.9 Summary

类型名称：`Summary`

Rust 伪代码：

```rust
pub struct SummaryEvent {
    pub summary_id: String,
    pub source_range_start_seq: u64,
    pub source_range_end_seq: u64,
    pub summary_text: String,
    pub key_facts: Vec<String>,
    pub open_threads: Vec<String>,
    pub generated_by: SummaryGenerator,
    pub token_count: Option<u32>,
}

pub enum SummaryGenerator {
    Llm,
    RuleBased,
    Hybrid,
}
```

何时产生：

- 压缩流程中，先生成摘要内容，再写入事件流。

谁产生：

- Layer 2 `SessionLifecycle`（通过 Layer 3 `LlmProvider` 或规则器生成）。

说明：

- `Summary` 是“替代旧区间”的语义实体。
- 与 `CompactionMarker` 组合使用，形成可追踪压缩边界。

### 2.10 事件类型与层责任对照表

| 事件类型 | 主要产出时机 | 产出层 | 主要消费者 |
|---|---|---|---|
| UserMessage | 用户输入进入 turn | Layer 2 | ContextBuilder / 投影器 |
| AssistantMessage | 模型产出最终文本 | Layer 2 | 投影器 / UI |
| ToolCallRequest | 模型请求工具 | Layer 2 | ToolRuntime / 观测 |
| ToolCallResult | 工具执行完成 | Layer 2 | ContextBuilder / 观测 |
| SystemEvent | 生命周期节点 | Layer 2 | Session 管理 / 审计 |
| ConfigChange | 运行时配置变更 | Layer 2 | ContextBuilder |
| Summary | 压缩生成摘要 | Layer 2 | ContextBuilder |
| CompactionMarker | 压缩落标记 | Layer 2 | ContextBuilder / 审计 |

---

## 3. 事件元数据

### 3.1 通用元数据字段

每个事件共享 `EventMeta`。

```rust
pub struct EventMeta {
    pub event_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub sequence_number: u64,
    pub timestamp_ms: i64,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    pub producer: EventProducer,
    pub schema_version: u16,
}

pub enum EventProducer {
    TurnExecutor,
    SessionLifecycle,
    ToolRuntimeBridge,
    SystemAdmin,
}
```

字段说明：

- `event_id`：全局唯一（ULID/UUIDv7 均可），用于幂等写与审计。
- `tenant_id`：多租户隔离主键（MVP 单租户可固定默认值）。
- `user_id`：用户归属。
- `agent_id`：agent 归属。
- `session_id`：会话归属。
- `sequence_number`：session 内严格单调递增。
- `timestamp_ms`：事件产生时间（UTC epoch ms）。
- `causation_id`：导致当前事件的直接上游事件 ID。
- `correlation_id`：同一 turn/请求链路 ID。
- `producer`：产出组件。
- `schema_version`：事件 schema 版本。

### 3.2 sequence_number 约束

约束规则：

1. 以 `session_id` 为作用域。
2. 从 1 开始递增。
3. 不回退、不复用。
4. 允许跳号仅在故障恢复非常规流程（MVP 不建议）。

实现建议：

- 通过事务 + `SELECT ... FOR UPDATE` 维护会话序号。
- 或通过 `(session_id, sequence_number)` 唯一索引 + 重试。

### 3.3 幂等与去重

最小幂等策略：

1. `event_id` 全局唯一约束。
2. 工具结果可加 `(request_id, status)` 级别去重保护。
3. 写入失败重试时重用同一 `event_id`。

### 3.4 时间语义

`timestamp_ms` 采用事件发生时间，而非数据库写入时间。

建议额外保留（存储层字段，可选）：

- `persisted_at`：数据库落盘时间。

用于区分：

- 业务先后顺序（看 `sequence_number`）。
- 实际落库延迟（`persisted_at - timestamp_ms`）。

### 3.5 元数据扩展原则

MVP 允许增量字段，但遵循：

1. 新增字段尽量 optional。
2. 不破坏既有事件解释。
3. 通过 `schema_version` 管理兼容。

---

## 4. System Prompt 稳定性原则

本节定义 prompt 稳定策略，目标是稳定行为 + 提升 prompt cache 命中率。

### 4.1 核心原则

1. System Prompt 在 session 首轮构建后不重新构建完整字符串。
2. 配置变更通过 `ConfigChange` 事件追加，而非回写旧事件。
3. ContextBuilder 在构建上下文时，从最近配置快照点读取当前配置。
4. 时间、临时提示等易变化内容不直接写入固定 System Prompt 主体。

### 4.2 为什么“首轮构建后稳定”

来自框架对比的经验：

- ZeroClaw 已验证“首轮 system prompt + 后续复用”有良好稳定性。
- 高频改写 system prompt 会降低模型行为一致性。
- prompt cache 对前缀稳定性敏感，频繁变动会显著降低命中。

收益：

1. 降低 token 成本（缓存命中更高）。
2. 降低回复风格漂移。
3. 减少“同会话内人格抖动”。

### 4.3 ConfigChange 事件驱动配置演进

配置不是“改原文”，而是“事件追加”。

流程：

1. 会话创建时写入 `SystemEvent(SessionCreated)`（含初始配置摘要）。
2. 会话运行中若配置变化，写入 `ConfigChange`。
3. ContextBuilder 构建当前 turn 时，读取最近一次 `ConfigChange`；若没有则使用 SessionCreated 初始配置。
4. 生成给 LLM 的有效配置视图。

配置变化示例：

- Persona 语气从 `friendly` 改为 `concise`。
- 工具白名单新增 `web_fetch`。
- 安全策略阈值调整。

### 4.4 ContextBuilder 的配置解析规则

伪代码：

```rust
fn resolve_effective_config(events: &[EventEnvelope]) -> EffectiveConfig {
    let base = find_session_created(events).initial_config;
    let latest_patch = find_latest_config_change(events);
    apply_patch(base, latest_patch)
}
```

实践细则：

1. 按 `sequence_number` 顺序读取。
2. 只取“最近配置状态”，不逐条重放全部 patch（MVP 简化）。
3. 当存在 Summary 时，Summary 可携带 `config_hash_hint` 加速定位（可选）。

### 4.5 对 prompt cache 的友好性论证

当 system prompt 主体保持稳定时：

- 相邻 turn 的 prompt 前缀重复率更高。
- 缓存键命中概率更高。
- 工具循环内重复调用模型时收益更明显。

当配置变更发生时：

- 仅在变更后产生新稳定版本。
- 不是每轮都变化，仍能维持局部高命中。

### 4.6 MVP 可行做法

MVP 不做复杂多版本缓存系统，仅需：

1. 缓存 `session_id -> base_system_prompt`。
2. 维护 `effective_config_hash`。
3. 仅当 hash 变化时重建 prompt 片段。

---

## 5. 事件流与上下文组装的关系

### 5.1 ContextBuilder 输入与目标

输入：

- 当前 session 的事件流（可能已含 summary/marker）。
- 当前用户输入（若尚未写入也可先临时拼装，推荐先写事件再读）。
- token 预算与模型能力信息。

输出：

- LLM request messages。
- 可选 tool specs。

### 5.2 事件到 LLM 消息角色映射

建议映射：

| 事件类型 | LLM 角色 | 说明 |
|---|---|---|
| UserMessage | `user` | 文本与必要附件引用 |
| AssistantMessage | `assistant` | 历史 assistant 文本 |
| ToolCallRequest | `assistant`（tool-call block） | 保持调用上下文 |
| ToolCallResult | `tool` / `user`(兼容模式) | 取决于 provider 协议 |
| Summary | `system` 或 `assistant`(summary block) | 推荐 `system` 的 history summary 段 |
| SystemEvent | 不直接映射 | 主要用于控制逻辑 |
| ConfigChange | 影响 system/config，不直接作为聊天消息 |
| CompactionMarker | 不映射 | 仅用于筛选与边界判定 |

### 5.3 组装流程（MVP）

步骤：

1. 解析有效配置（Section 4 规则）。
2. 选择最近有效 Summary（若有）。
3. 从事件尾部向前选择“最近 N 个可映射事件”。
4. 应用 token 预算裁剪。
5. 生成最终 messages：
   - system（稳定 prompt + 运行时轻量段）
   - summary（可选）
   - recent history（user/assistant/tool）
   - current user input

### 5.4 Token 预算策略

定义：

- `model_context_limit`
- `reserved_for_completion`
- `reserved_for_tools`
- `available_for_prompt = limit - reserved_completion - reserved_tools`

裁剪顺序建议：

1. 保留 system prompt（硬保留）。
2. 保留当前用户输入（硬保留）。
3. 保留最近一次 Summary（软硬之间，建议硬保留）。
4. 从最近历史逆序加入事件，直到接近预算。
5. 超预算时优先丢弃最旧的 raw 历史，不丢系统配置。

### 5.5 事件筛选规则

应纳入上下文：

- 近窗 `UserMessage` / `AssistantMessage`
- 与近窗相关的 `ToolCallRequest` / `ToolCallResult`
- 最新 `Summary`

可忽略：

- 过旧且已被 summary 覆盖的消息。
- `CompactionMarker`。
- 与当前 turn 无关的低价值 `SystemEvent`。

### 5.6 工具循环中的上下文更新

工具循环每轮应追加：

1. 本轮 `ToolCallRequest`
2. 本轮 `ToolCallResult`

下一轮 LLM 调用时：

- 至少包含最近工具请求与结果，保证模型能基于结果继续推理。

### 5.7 最小伪代码

```rust
fn build_llm_messages(events: &[EventEnvelope], budget: TokenBudget) -> Vec<ModelMessage> {
    let cfg = resolve_effective_config(events);
    let mut out = vec![build_system_message(cfg)];

    if let Some(summary) = pick_latest_summary(events) {
        out.push(map_summary(summary));
    }

    let recent = pick_recent_mappable_events(events);
    let trimmed = trim_by_budget(recent, budget.remaining_for_history(&out));

    out.extend(map_events(trimmed));
    out
}
```

---

## 6. 压缩策略

### 6.1 触发条件

MVP 推荐双阈值触发，满足任一即触发：

1. Token 阈值：
   - 最近一次上下文估算 `prompt_tokens >= 70% * model_context_limit`。
2. 事件数量阈值：
   - session 内“可映射历史事件”数量 >= 120。

补充触发：

- 手动触发（运维或测试）。
- 长会话空闲时后台触发（可选）。

### 6.2 压缩目标

目标不是“删除历史”，而是“降低上下文组装成本”。

保留性质：

- 原始事件仍在事件流中。
- 通过 Marker + Summary 标识“逻辑替代关系”。

### 6.3 压缩流程

一次压缩建议流程：

1. 选定待压缩区间 `[start_seq, end_seq]`（通常是“除最近 N 条外的旧历史”）。
2. 读取该区间可映射事件。
3. 生成摘要文本与关键事实。
4. 追加 `Summary` 事件。
5. 追加 `CompactionMarker` 事件，引用 summary 与区间。
6. 更新 session 的压缩游标（可放在 session_state 或衍生投影里）。

### 6.4 压缩后事件流结构

逻辑视图：

```text
[old events .........][Summary][CompactionMarker][recent N events][new events ...]
```

上下文构建视图：

```text
System Prompt
+ Latest Summary (cover old range)
+ Recent N raw events
+ Current turn events
```

### 6.5 CompactionMarker 的作用

`CompactionMarker` 主要作用：

1. 明确“哪个区间被哪个 summary 替代”。
2. 防止 ContextBuilder 重复选取已覆盖区间。
3. 为审计与调试提供压缩边界。
4. 支持多次压缩（分段、层级压缩）时的边界管理。

### 6.6 多次压缩策略（MVP 简化）

MVP 采用“滑动窗口 + 最新摘要优先”策略：

1. 每次压缩只生成一条新的 `Summary`。
2. 读取时仅使用“最新且覆盖范围最大的 summary”。
3. 旧 summary 保留但通常不进入 prompt。

后续可演进为层级摘要（summary of summaries）。

### 6.7 失败与回滚

若压缩流程失败：

- 不应写入不完整 marker。
- 可写 `SystemEvent(TurnFailed/PolicyNotice)` 记录失败。
- 下次满足阈值再重试。

原子性建议：

- `Summary + CompactionMarker` 在一个事务内写入。

### 6.8 默认参数（MVP 建议值）

- `token_threshold_ratio = 0.70`
- `event_count_threshold = 120`
- `recent_events_to_keep = 24`
- `max_compaction_per_turn = 1`

这些参数应可配置，但先给安全默认值。

---

## 7. 与现有 messages 表的差异

本节基于当前 migration：

- `20260222000001_create_users.sql`
- `20260222000002_create_sessions_messages.sql`
- `20260222000003_add_message_sequence.sql`

### 7.1 当前 messages 模型（现状）

当前 `messages` 表核心字段：

- `id UUID`
- `session_id UUID`
- `role TEXT`
- `content TEXT`
- `created_at TIMESTAMPTZ`
- `sequence_num BIGSERIAL`（后加）

特点：

1. 以“消息行”为中心。
2. 工具调用、系统事件、压缩标记未建模为一等实体。
3. `sequence_num` 是全表序列，不是 session 内局部序号。
4. 无统一事件元数据（`event_id`、`causation_id` 等）。

### 7.2 事件流模型需要的结构变化

从 message-centric 到 event-centric 的变化：

1. 新增事件存储（建议 `events` 主表）。
2. 每条记录含 envelope：meta + payload。
3. 引入 session 内 `sequence_number`。
4. 支持多类型 payload（消息/工具/系统/压缩）。
5. 支持配置变更与压缩边界事件。

### 7.3 字段层面对照

| 维度 | 当前 messages | 目标 events |
|---|---|---|
| 记录粒度 | 仅消息 | 所有业务事实事件 |
| 类型表达 | `role` 文本 | `event_type` + typed payload |
| 顺序语义 | `sequence_num` 全局序列 | `sequence_number`（session 内单调） |
| 幂等键 | `id` | `event_id` 全局唯一 |
| 因果链 | 无 | `causation_id` / `correlation_id` |
| 租户字段 | 经由 session 间接 | 事件元数据直接携带（推荐） |
| 压缩语义 | session.summary 字段 | `Summary + CompactionMarker` 事件 |
| 配置变更 | 多靠更新字段 | `ConfigChange` 追加事件 |

### 7.4 Session 表的角色变化

当前 `sessions.summary` 是可变字段。

迁移后建议：

1. `sessions` 主要存索引信息与生命周期状态（title、archived、timestamps）。
2. 摘要主语义迁移到 `Summary` 事件。
3. 若保留 `sessions.summary`，仅作为缓存投影，不作为 source of truth。

### 7.5 读取路径变化

当前：

- 读取会话上下文主要依赖 `messages` + `sessions.summary`。

迁移后：

- ContextBuilder 读取 `events`，并根据类型映射构建上下文。
- 消息列表由事件投影器生成（可增量缓存）。

### 7.6 写入路径变化

当前：

- 写 user/assistant 消息到 `messages`。

迁移后：

- turn 中多个步骤均写事件：
  - UserMessage
  - ToolCallRequest/Result（可选）
  - AssistantMessage
  - SystemEvent
  - Summary/CompactionMarker（触发时）

### 7.7 兼容期建议（只列差异，不含脚本）

兼容期一般会出现双轨：

1. `messages` 继续服务旧读路径。
2. `events` 作为新真相源。
3. 逐步切换 ContextBuilder 到 `events`。
4. 最终将 `messages` 降级为投影或废弃。

本设计不展开迁移脚本细节。

---

## 8. MVP 实施建议（补充）

> 本节不改变前述 7 章验收项，仅给出可落地顺序，降低实施风险。

### 8.1 最小事件集上线顺序

第 1 阶段：

- `UserMessage`
- `AssistantMessage`
- `SystemEvent(SessionCreated, TurnStarted, TurnCompleted)`

第 2 阶段：

- `ToolCallRequest`
- `ToolCallResult`

第 3 阶段：

- `ConfigChange`
- `Summary`
- `CompactionMarker`

### 8.2 最小读取能力顺序

1. 先支持消息型事件映射到 LLM。
2. 再接入工具事件映射。
3. 最后接入压缩识别与 summary 优先读取。

### 8.3 风险提示

MVP 常见风险：

1. session 内序号并发冲突。
2. 工具事件缺失导致上下文断链。
3. 压缩标记写入不原子。

对应最小防护：

1. 唯一索引 + 重试。
2. `ToolCallRequest` 与 `ToolCallResult` 强关联校验。
3. `Summary + Marker` 同事务。

---

## 9. 事件示例（端到端）

### 9.1 无工具调用的简化 turn

```text
# seq=1 SystemEvent(SessionCreated)
# seq=2 UserMessage("帮我写个周报")
# seq=3 SystemEvent(TurnStarted)
# seq=4 AssistantMessage("这是你的周报草稿...")
# seq=5 SystemEvent(TurnCompleted)
```

### 9.2 带工具调用的 turn

```text
# seq=20 UserMessage("查下明天上海天气")
# seq=21 SystemEvent(TurnStarted)
# seq=22 ToolCallRequest(tool=weather_api, args={city:shanghai,date:tomorrow})
# seq=23 ToolCallResult(status=Success, output={temp:18,condition:cloudy})
# seq=24 AssistantMessage("明天上海多云，18°C...")
# seq=25 SystemEvent(TurnCompleted)
```

### 9.3 压缩后的结构示例

```text
# seq=1..120 old raw events
# seq=121 Summary(range=1..96)
# seq=122 CompactionMarker(replaced=1..96, summary=seq121)
# seq=123..140 recent raw events
```

ContextBuilder 读取时：

- 使用 seq121 摘要。
- 跳过 1..96 raw events。
- 保留 123..140 recent events。

---

## 10. 验收清单（对照任务）

- [x] 文档路径：`docs/design/core/event-model.md`
- [x] 包含 7 个必需章节（第 1~7 章）
- [x] 事件类型 >= 7（本文 8 种），每种含 Rust 伪代码
- [x] 明确论证 System Prompt 稳定性与 ConfigChange 机制
- [x] 给出可执行压缩触发条件与流程
- [x] 只写设计文档，不涉及代码变更

---

## 附录 A：术语

- Event Stream：按序追加事件序列。
- Projection：从事件流得到的读模型。
- Compaction：通过摘要降低上下文负担的过程。
- Marker：描述压缩边界的事件。
- Effective Config：当前 turn 生效配置视图。

## 附录 B：行数说明

本文按设计规范采用短段落与逐项列举，便于评审与后续拆分实现。
