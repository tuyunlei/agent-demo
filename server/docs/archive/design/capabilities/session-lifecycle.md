# 会话生命周期设计（D-CAP-04）

**状态**：Draft（MVP 可实施）

**更新时间**：2026-02-24

**所属层**：Layer 2 Orchestration + Layer 3 Capability

**关联文档**：

- `docs/design/architecture.md`
- `docs/design/core/event-model.md`
- `docs/design/capabilities/context-builder.md`
- `crates/agent-storage/migrations/20260222000002_create_sessions_messages.sql`
- `crates/agent-storage/migrations/20260222000003_add_message_sequence.sql`

**参考实现**：

- OpenClaw: `src/config/sessions/store.ts`
- OpenClaw: `src/agents/session-write-lock.ts`
- OpenClaw: `src/agents/pi-embedded-runner/compact.ts`
- PicoClaw: `pkg/session/manager.go`
- PicoClaw: `pkg/agent/loop.go`

---

## 1. 设计目标

### 1.1 问题定义

agent-demo 的会话管理目前具备基础 `sessions/messages` 表结构。

但尚未形成完整的生命周期治理模型。

缺口主要在：

- 会话状态机不明确。
- 压缩流程缺少统一编排入口。
- 并发写入策略未在领域层抽象。
- 归档策略尚未定义接口。
- `session state = fold(events)` 的实现路径未落地。

### 1.2 生命周期目标

本设计定义完整会话生命周期：

1. 创建（Create）
2. 活跃（Active）
3. 压缩（Compacting）
4. 归档（Archived）

并将其映射到：

- Layer 2：`SessionLifecycle`（流程编排）
- Layer 3：`Session` + `EventStore` trait（领域抽象）
- Layer 4：`PostgresEventStore`（基础设施实现）

### 1.3 并发安全目标

在同一 `session_id` 下，多个 turn 并发到达时：

- 不丢事件。
- 不乱序提交。
- 不覆盖元数据。
- 不让压缩与普通写入互相破坏。

MVP 采用“**session 级串行化 + 乐观版本校验**”混合策略。

### 1.4 与事件流模型对齐目标

与 `event-model.md` 严格对齐：

- 事件流是事实真相源（source of truth）。
- 会话状态是 fold 结果，不是随意可变快照。
- 压缩用 `Summary + CompactionMarker` 表达，不删除历史事实。
- 生命周期动作本身也事件化（SystemEvent）。

### 1.5 MVP 边界

本期目标：

- 定义完整设计与 trait。
- 实现路径清晰可执行。
- 先支持 Active + Compacting 主链路。

本期不做：

- 冷存储真实迁移作业。
- 自动归档后台任务实现。
- 跨集群分布式锁。

---

## 2. Session 状态模型

### 2.1 设计原则

Session 实体承担两类信息：

- 领域元数据（谁、何时、状态）。
- 事件流索引信息（版本、序号、计数、token 估算）。

会话内容本体不在 Session 行里冗余保存。

会话内容来自事件流读取。

### 2.2 状态枚举

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SessionStatus {
    /// 正常对话状态，允许追加 turn 事件。
    Active,

    /// 正在执行压缩流程（生成 summary + marker）。
    /// 允许并发读；写策略见并发控制章节。
    Compacting,

    /// 已归档，不参与常规活跃会话列表。
    /// 是否允许只读访问由策略决定。
    Archived,
}
```

### 2.3 Session 实体（Rust 伪代码）

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Session {
    // ---- 身份维度 ----
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub agent_id: AgentId,

    // ---- 状态维度 ----
    pub status: SessionStatus,

    // ---- 元数据 ----
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,

    // ---- 事件流统计 ----
    /// 当前已写入事件总数（可用于阈值判断）。
    pub event_count: u64,

    /// session 内最新 sequence_number。
    pub last_sequence: u64,

    /// 估算上下文 token（用于压缩触发预判，非强一致）。
    pub estimated_prompt_tokens: Option<u32>,

    /// 最近一次 compaction 后的 token 估算快照。
    pub estimated_tokens_after_compaction: Option<u32>,

    /// 最近一次 compaction 覆盖到的 sequence（游标）。
    pub compacted_until_sequence: Option<u64>,

    /// 版本号，用于 optimistic lock。
    pub version: u64,

    // ---- 归档预留 ----
    pub archived_at: Option<DateTime<Utc>>,
    pub archive_tier: Option<ArchiveTier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArchiveTier {
    Warm,
    Cold,
}
```

### 2.4 字段语义说明

`event_count`：

- 快速评估是否需要压缩。
- 避免每次 `COUNT(*)` 全量扫描。

`last_sequence`：

- 维护会话内有序追加。
- 支持范围读取和增量读取。

`estimated_prompt_tokens`：

- 为 ContextBuilder 和 Lifecycle 提供触发参考。
- 允许与真实 token 略有误差。

`version`：

- 每次 Session 元数据更新自增。
- 与 `WHERE version = ?` 配合做乐观并发控制。

`status`：

- 不是会话内容状态。
- 是生命周期治理状态。

### 2.5 Session 创建参数（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct CreateSessionParams {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub agent_id: AgentId,
    pub title: Option<String>,
    pub external_key: Option<String>,
}
```

`external_key` 典型用于：

- channel + chat + thread 组合键。
- 幂等 load_or_create。

---

## 3. EventStore Trait

### 3.1 角色定位

`EventStore` 是 Layer 3 端口（port）。

职责：

- 管理会话元数据。
- 维护事件流 append/read。
- 为 ContextBuilder / SessionLifecycle 提供统一读写抽象。

不负责：

- 压缩策略决策。
- 归档触发策略。
- 业务编排。

### 3.2 Trait 定义（Rust 伪代码）

```rust
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct EventRange {
    pub start_inclusive: Option<u64>,
    pub end_inclusive: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SessionListFilter {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub include_archived: bool,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum EventStoreError {
    #[error("session not found: {0}")]
    SessionNotFound(SessionId),

    #[error("session already exists: {0}")]
    SessionAlreadyExists(String),

    #[error("session status conflict: {0}")]
    SessionStatusConflict(String),

    #[error("version conflict: expected={expected}, actual={actual}")]
    VersionConflict { expected: u64, actual: u64 },

    #[error("sequence conflict in session {0}")]
    SequenceConflict(SessionId),

    #[error("storage error: {0}")]
    Storage(String),
}

#[async_trait]
pub trait EventStore: Send + Sync {
    /// 追加事件到会话流。
    /// 实现需保证 session 内 sequence 单调递增。
    async fn append_events(
        &self,
        session_id: &SessionId,
        events: Vec<NewEvent>,
    ) -> Result<AppendResult, EventStoreError>;

    /// 按 sequence 范围读取事件（升序）。
    async fn read_events(
        &self,
        session_id: &SessionId,
        range: EventRange,
    ) -> Result<Vec<EventEnvelope>, EventStoreError>;

    /// 读取最近 N 条事件（按时间/sequence 倒序读取后再升序返回）。
    async fn read_recent_events(
        &self,
        session_id: &SessionId,
        limit: usize,
    ) -> Result<Vec<EventEnvelope>, EventStoreError>;

    /// 获取会话元数据。
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, EventStoreError>;

    /// 创建会话。
    async fn create_session(&self, params: CreateSessionParams) -> Result<Session, EventStoreError>;

    /// 按用户列出会话。
    async fn list_sessions(&self, filter: SessionListFilter) -> Result<Vec<Session>, EventStoreError>;

    /// 原子更新会话元数据（带 version 校验）。
    async fn update_session(
        &self,
        session_id: &SessionId,
        expected_version: u64,
        patch: SessionPatch,
    ) -> Result<Session, EventStoreError>;

    /// 通过 external_key 定位会话（可选实现但推荐）。
    async fn find_session_by_external_key(
        &self,
        tenant_id: &TenantId,
        external_key: &str,
    ) -> Result<Option<Session>, EventStoreError>;
}
```

### 3.3 追加结果模型

```rust
#[derive(Debug, Clone)]
pub struct AppendResult {
    pub appended: usize,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub session_version: u64,
    pub session_event_count: u64,
}
```

### 3.4 NewEvent / SessionPatch 示例

```rust
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub event_id: EventId,
    pub causation_id: Option<EventId>,
    pub correlation_id: Option<String>,
    pub producer: EventProducer,
    pub payload: EventPayload,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Default)]
pub struct SessionPatch {
    pub status: Option<SessionStatus>,
    pub title: Option<Option<String>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_active_at: Option<chrono::DateTime<chrono::Utc>>,
    pub event_count: Option<u64>,
    pub last_sequence: Option<u64>,
    pub estimated_prompt_tokens: Option<Option<u32>>,
    pub estimated_tokens_after_compaction: Option<Option<u32>>,
    pub compacted_until_sequence: Option<Option<u64>>,
    pub archived_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub archive_tier: Option<Option<ArchiveTier>>,
}
```

### 3.5 Postgres 实现建议

基于当前 migration（`sessions/messages`）后续演进：

- 引入 `events` 表（event envelope）。
- `sessions` 表扩展状态、版本、计数字段。
- 索引：
  - `(session_id, sequence_number)` 唯一。
  - `(tenant_id, user_id, updated_at desc)` 列表查询。
  - `(tenant_id, external_key)` 唯一（可选）。

MVP 阶段可保留旧表并增量迁移读写路径。

---

## 4. SessionLifecycle（编排层）

### 4.1 角色定位

`SessionLifecycle` 位于 Layer 2。

负责生命周期编排，不做底层存储实现。

它通过 `EventStore` trait 操作会话与事件。

### 4.2 与 TurnExecutor 协作关系

标准 turn 时序：

1. `TurnExecutor.run_turn(input)`
2. `SessionLifecycle.load_or_create(key)`
3. `ContextBuilder.build(...)`
4. `LlmProvider.complete(...)`
5. 工具循环（可选）
6. `SessionLifecycle.persist_turn(delta)`
7. `SessionLifecycle.compact_if_needed(key)`
8. `SessionLifecycle.archive_if_needed(key)`

关键点：

- `persist_turn` 是写入主入口。
- 压缩和归档是后置治理动作。
- 不让 TurnExecutor 直接操作 session 元数据。

### 4.3 SessionLifecycle Trait（Rust 伪代码）

```rust
#[derive(Debug, Clone)]
pub struct SessionKey {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub agent_id: AgentId,
    pub external_key: String,
}

#[derive(Debug, Clone)]
pub struct TurnDelta {
    pub session_id: SessionId,
    pub expected_version: u64,
    pub events: Vec<NewEvent>,
    pub estimated_prompt_tokens: Option<u32>,
    pub last_active_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionLifecycleError {
    #[error(transparent)]
    Store(#[from] EventStoreError),

    #[error("compaction already in progress")]
    CompactionInProgress,

    #[error("session archived")]
    SessionArchived,

    #[error("invalid lifecycle transition: {0}")]
    InvalidTransition(String),

    #[error("concurrency conflict, retry required")]
    RetryableConflict,
}

#[async_trait]
pub trait SessionLifecycle: Send + Sync {
    async fn load_or_create(&self, key: SessionKey) -> Result<Session, SessionLifecycleError>;

    async fn persist_turn(&self, delta: TurnDelta) -> Result<AppendResult, SessionLifecycleError>;

    async fn compact_if_needed(&self, key: &SessionKey) -> Result<CompactionOutcome, SessionLifecycleError>;

    async fn archive_if_needed(&self, key: &SessionKey) -> Result<ArchiveOutcome, SessionLifecycleError>;
}
```

### 4.4 `load_or_create(key)` 伪代码

```rust
pub async fn load_or_create(&self, key: SessionKey) -> Result<Session, SessionLifecycleError> {
    // 1) 先按 external_key 查找，避免重复会话。
    if let Some(existing) = self.store
        .find_session_by_external_key(&key.tenant_id, &key.external_key)
        .await? {
        return Ok(existing);
    }

    // 2) 不存在则创建。
    //    create_session 内部需处理唯一约束竞争（并发创建）。
    let created = self.store.create_session(CreateSessionParams {
        tenant_id: key.tenant_id.clone(),
        user_id: key.user_id.clone(),
        agent_id: key.agent_id.clone(),
        title: None,
        external_key: Some(key.external_key.clone()),
    }).await;

    match created {
        Ok(session) => Ok(session),
        Err(EventStoreError::SessionAlreadyExists(_)) => {
            // 并发创建时，回读即可。
            self.store
                .find_session_by_external_key(&key.tenant_id, &key.external_key)
                .await?
                .ok_or_else(|| SessionLifecycleError::RetryableConflict)
        }
        Err(e) => Err(SessionLifecycleError::Store(e)),
    }
}
```

### 4.5 `persist_turn(delta)` 伪代码

```rust
pub async fn persist_turn(&self, delta: TurnDelta) -> Result<AppendResult, SessionLifecycleError> {
    // 1) 读会话元数据。
    let session = self.store
        .get_session(&delta.session_id)
        .await?
        .ok_or(EventStoreError::SessionNotFound(delta.session_id.clone()))?;

    // 2) 状态检查。
    match session.status {
        SessionStatus::Archived => return Err(SessionLifecycleError::SessionArchived),
        SessionStatus::Compacting => {
            // MVP 策略：允许写入（压缩只压旧区间），不阻塞当前 turn。
            // 若后续改为强串行，可在此返回 RetryableConflict。
        }
        SessionStatus::Active => {}
    }

    // 3) 追加事件。
    let append = self.store
        .append_events(&delta.session_id, delta.events)
        .await?;

    // 4) 更新 session 元数据（乐观锁）。
    let patched = self.store
        .update_session(
            &delta.session_id,
            session.version,
            SessionPatch {
                updated_at: Some(chrono::Utc::now()),
                last_active_at: Some(delta.last_active_at),
                event_count: Some(append.session_event_count),
                last_sequence: Some(append.last_sequence),
                estimated_prompt_tokens: Some(delta.estimated_prompt_tokens),
                ..Default::default()
            },
        )
        .await;

    match patched {
        Ok(_) => Ok(append),
        Err(EventStoreError::VersionConflict { .. }) => {
            // 元数据冲突不影响事件已追加事实，返回可重试/可忽略策略。
            // MVP：返回 RetryableConflict 让上层决定是否补更新。
            Err(SessionLifecycleError::RetryableConflict)
        }
        Err(e) => Err(SessionLifecycleError::Store(e)),
    }
}
```

### 4.6 `compact_if_needed(key)` 伪代码

```rust
pub async fn compact_if_needed(&self, key: &SessionKey) -> Result<CompactionOutcome, SessionLifecycleError> {
    let session = self.resolve_session(key).await?;

    if session.status == SessionStatus::Archived {
        return Ok(CompactionOutcome::Skipped("archived".into()));
    }

    if !self.compaction_policy.should_compact(&session) {
        return Ok(CompactionOutcome::Skipped("threshold_not_reached".into()));
    }

    // CAS: Active -> Compacting
    let session = match self.store.update_session(
        &session.session_id,
        session.version,
        SessionPatch {
            status: Some(SessionStatus::Compacting),
            updated_at: Some(chrono::Utc::now()),
            ..Default::default()
        }
    ).await {
        Ok(s) => s,
        Err(EventStoreError::VersionConflict { .. }) => {
            return Ok(CompactionOutcome::Skipped("concurrent_transition".into()));
        }
        Err(e) => return Err(e.into()),
    };

    let result = self.run_compaction_flow(&session).await;

    // 结束态回写：Compacting -> Active（成功/失败都要回收状态）
    let latest = self.store.get_session(&session.session_id).await?
        .ok_or(EventStoreError::SessionNotFound(session.session_id.clone()))?;

    let _ = self.store.update_session(
        &session.session_id,
        latest.version,
        SessionPatch {
            status: Some(SessionStatus::Active),
            updated_at: Some(chrono::Utc::now()),
            ..Default::default()
        }
    ).await;

    result
}
```

### 4.7 `archive_if_needed(key)` 伪代码

```rust
pub async fn archive_if_needed(&self, key: &SessionKey) -> Result<ArchiveOutcome, SessionLifecycleError> {
    let session = self.resolve_session(key).await?;

    if session.status == SessionStatus::Archived {
        return Ok(ArchiveOutcome::AlreadyArchived);
    }

    if !self.archive_policy.should_archive(&session) {
        return Ok(ArchiveOutcome::Skipped("policy_not_matched".into()));
    }

    // MVP: 仅预留状态迁移，不做物理搬迁。
    let _updated = self.store.update_session(
        &session.session_id,
        session.version,
        SessionPatch {
            status: Some(SessionStatus::Archived),
            archived_at: Some(Some(chrono::Utc::now())),
            archive_tier: Some(Some(ArchiveTier::Warm)),
            updated_at: Some(chrono::Utc::now()),
            ..Default::default()
        }
    ).await?;

    Ok(ArchiveOutcome::Archived)
}
```

---

## 5. 压缩策略

### 5.1 触发条件（对齐 event-model）

触发策略与 `event-model.md` 保持一致，采用双阈值：

- Token 阈值：`estimated_prompt_tokens >= 0.70 * model_context_limit`
- 事件阈值：`event_count >= 120`（可配置）

补充触发：

- 手动触发（命令/API）。
- 后台维护触发（未来）。

### 5.2 压缩目标

压缩目标不是删除历史。

目标是降低上下文构建成本。

原则：

- 事实事件仍可审计。
- 上下文优先读取 summary + recent。

### 5.3 标准压缩流程

#### 步骤 1：读取旧事件区间

- 定位压缩起点：`compacted_until_sequence + 1` 或 `1`。
- 终点：`last_sequence - keep_recent_window`。
- 读取 `[start, end]` 的可映射事件。

#### 步骤 2：生成摘要

两种策略：

- LLM Summary：质量高，成本更高。
- Rule-based 截断：成本低，质量一般。

MVP 默认：先 rule-based，可选切到 LLM。

#### 步骤 3：写入 Summary + CompactionMarker

按顺序追加两条事件：

1. `Summary`
2. `CompactionMarker`

二者必须同一事务提交（逻辑原子）。

#### 步骤 4：标记区间

更新 Session 元数据：

- `compacted_until_sequence = end`
- `estimated_tokens_after_compaction = new_estimate`

### 5.4 压缩流程伪代码

```rust
async fn run_compaction_flow(&self, session: &Session) -> Result<CompactionOutcome, SessionLifecycleError> {
    let keep_recent = self.compaction_policy.keep_recent_events;
    if session.last_sequence <= keep_recent as u64 {
        return Ok(CompactionOutcome::Skipped("too_few_events".into()));
    }

    let start = session.compacted_until_sequence.unwrap_or(0) + 1;
    let end = session.last_sequence.saturating_sub(keep_recent as u64);

    if end < start {
        return Ok(CompactionOutcome::Skipped("no_compactable_range".into()));
    }

    let old_events = self.store.read_events(
        &session.session_id,
        EventRange {
            start_inclusive: Some(start),
            end_inclusive: Some(end),
        }
    ).await?;

    if old_events.is_empty() {
        return Ok(CompactionOutcome::Skipped("empty_range".into()));
    }

    let summary_text = self.summary_service.summarize(&old_events).await?;

    let summary_event = NewEvent {
        event_id: EventId::new(),
        causation_id: None,
        correlation_id: Some(format!("compaction:{}", session.session_id)),
        producer: EventProducer::SessionLifecycle,
        payload: EventPayload::Summary(SummaryEvent {
            summary_id: SummaryId::new().to_string(),
            source_range_start_seq: start,
            source_range_end_seq: end,
            summary_text: summary_text.clone(),
            key_facts: vec![],
            open_threads: vec![],
            generated_by: SummaryGenerator::RuleBased,
            token_count: None,
        }),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };

    let marker_event = NewEvent {
        event_id: EventId::new(),
        causation_id: Some(summary_event.event_id.clone()),
        correlation_id: Some(format!("compaction:{}", session.session_id)),
        producer: EventProducer::SessionLifecycle,
        payload: EventPayload::CompactionMarker(CompactionMarkerEvent {
            compaction_id: CompactionId::new().to_string(),
            strategy: CompactionStrategy::SlidingSummary,
            replaced_range_start_seq: start,
            replaced_range_end_seq: end,
            summary_event_id: summary_event.event_id.to_string(),
            trigger: CompactionTrigger::EventCountThreshold,
        }),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };

    let append = self.store.append_events(&session.session_id, vec![summary_event, marker_event]).await?;

    let _ = self.store.update_session(
        &session.session_id,
        session.version,
        SessionPatch {
            compacted_until_sequence: Some(Some(end)),
            estimated_tokens_after_compaction: Some(self.token_estimator.estimate_after_compaction(summary_text)),
            last_sequence: Some(append.last_sequence),
            event_count: Some(append.session_event_count),
            updated_at: Some(chrono::Utc::now()),
            ..Default::default()
        }
    ).await;

    Ok(CompactionOutcome::Compacted {
        range_start: start,
        range_end: end,
        appended_events: append.appended,
    })
}
```

### 5.5 MVP 建议

MVP 分两阶段：

阶段 A（立即可做）：

- 保留最近 N 条。
- 老区间做截断摘要（规则模板）。
- 写 Summary + Marker。

阶段 B（后续迭代）：

- 接入 LLM 生成结构化 summary。
- 增加 key facts / open threads。
- 支持层级摘要（summary of summaries）。

### 5.6 压缩期间并发保护

压缩期间允许新 turn 到达。

策略：

- 压缩只处理“压缩启动时刻之前的旧区间”。
- 新事件会落在更大 sequence，不会落入本次区间。
- 通过 `CompactionMarker` 精确指明覆盖区间。

这样可避免“压缩锁全会话导致用户阻塞”。

---

## 6. 并发控制

### 6.1 设计目标

并发模型需要同时满足：

- 高可用（不能因锁放大阻塞）。
- 一致性（sequence/version 不冲突）。
- 易实现（MVP 不引入复杂分布式组件）。

### 6.2 候选策略对比

#### 方案 A：纯悲观锁

优点：

- 实现直观。
- 逻辑冲突少。

缺点：

- 高并发时吞吐低。
- 压缩可能长时间占锁。

#### 方案 B：纯乐观锁

优点：

- 锁开销低。
- 并发吞吐好。

缺点：

- 冲突时重试复杂。
- 压缩与写入交错逻辑更难。

#### 方案 C：混合策略（推荐）

- 事件追加路径：轻量悲观（session 级短事务）保证 sequence。
- 元数据更新：乐观 version CAS。
- 压缩状态迁移：CAS（Active->Compacting）。

兼顾一致性与吞吐。

### 6.3 MVP 最终选择

采用**混合策略**：

1. `append_events` 在 DB 事务内分配 sequence。
2. `update_session` 使用 `expected_version` 乐观校验。
3. `compact_if_needed` 先 CAS 状态，再处理压缩。
4. 冲突返回 retryable error，由编排层有限重试。

### 6.4 同 session 多 turn 到达处理

处理规则：

- 每个 turn 各自产生事件列表。
- `append_events` 串行化提交（同 session）。
- 后提交 turn 拿到更高 sequence。
- 上下文构建永远按 sequence 读取。

结果：

- 不会写乱序。
- 不会互相覆盖。

### 6.5 死锁避免设计

遵守固定锁顺序：

1. session store lock（若有进程内锁）
2. DB session row lock
3. event rows insert

禁止：

- 持有 DB 锁时调用外部 LLM。
- 交叉 session 锁（A->B 与 B->A）。

压缩流程中：

- 读旧事件可不持久占锁。
- 写 summary/marker 时使用短事务。

### 6.6 重试策略建议

仅对可重试错误重试：

- VersionConflict
- SequenceConflict
- 短暂锁超时

建议：

- 最多重试 2~3 次。
- 指数退避（20ms, 50ms, 100ms）。
- 记录冲突指标。

### 6.7 参考框架启发

OpenClaw 实践显示：

- “写前锁 + 锁内重读 + 原子写回”能显著减少并发覆盖。
- 明确锁队列比无序并发更可控。

PicoClaw 使用 `sync.RWMutex` 保护 SessionManager：

- 简洁有效。
- 但属于单进程内存态，跨进程不适用。

本设计在此基础上升级到 DB 级并发语义。

---

## 7. 归档策略

### 7.1 触发条件

归档用于处理长期不活跃会话。

建议触发条件（可配置）：

- `last_active_at` 距今 > 30 天。
- 且会话状态为 Active。
- 且不在最近访问白名单。

### 7.2 归档后访问模型

两种主流模式：

#### 模式 A：冷存储可读

- 元数据保留在线。
- 事件搬迁到冷库（低频读取）。
- 需要恢复流程。

#### 模式 B：直接不可访问

- 标记 archived 后默认不在常规 API 返回。
- 管理接口可恢复。

### 7.3 MVP 建议

MVP 采用“**暂不实现物理归档，先预留接口**”：

- 支持 `status=Archived` 状态。
- `list_sessions` 默认过滤 Archived。
- 读取接口可加 `include_archived`。
- 物理冷存储迁移留到后续。

### 7.4 归档接口预留（Rust 伪代码）

```rust
#[async_trait]
pub trait SessionArchiveStore: Send + Sync {
    async fn archive_session(&self, session_id: &SessionId) -> Result<ArchiveRef, ArchiveError>;
    async fn restore_session(&self, session_id: &SessionId) -> Result<(), ArchiveError>;
    async fn get_archive_ref(&self, session_id: &SessionId) -> Result<Option<ArchiveRef>, ArchiveError>;
}

#[derive(Debug, Clone)]
pub struct ArchiveRef {
    pub session_id: SessionId,
    pub tier: ArchiveTier,
    pub uri: String,
    pub archived_at: chrono::DateTime<chrono::Utc>,
}
```

### 7.5 生命周期迁移约束

允许：

- Active -> Archived
- Archived -> Active（恢复）

不允许：

- Compacting -> Archived（需先回 Active）

理由：

- 简化状态机。
- 降低竞态复杂度。

---

## 8. 与其他层的关系

### 8.1 层间依赖关系

符合 `architecture.md`：

- Layer 2 `SessionLifecycle` 依赖 Layer 3 `EventStore` trait。
- Layer 3 定义 `Session/Event` 领域模型。
- Layer 4 实现 `PostgresEventStore`。
- Layer 1 在 Composition Root 注入具体实现。

### 8.2 与 ContextBuilder 关系

`ContextBuilder` 从 `EventStore` 读取事件流。

读取策略：

- 优先 `Summary + recent events`。
- 识别 `CompactionMarker` 跳过已替代区间。

Lifecycle 不直接参与 prompt 组装。

但 Lifecycle 维护的 `estimated_prompt_tokens` 会作为触发输入。

### 8.3 与 TurnExecutor 关系

TurnExecutor 是生命周期调用方。

它只关心：

- 何时 load/create。
- 何时 persist。
- 何时触发 compact/archive。

它不关心：

- sequence 分配细节。
- version CAS 细节。
- 存储事务细节。

### 8.4 与基础设施层关系

`PostgresEventStore` 实现点：

- `append_events` 事务写入。
- `read_events/read_recent_events` 索引查询。
- `create_session/list_sessions/get_session/update_session`。

推荐在 Layer 4 同时提供：

- SQL 级唯一约束。
- 必要索引。
- 错误映射到 `EventStoreError`。

### 8.5 接入层注入职责

Layer 1（channel/server）负责：

- 创建 Postgres 连接池。
- 实例化 PostgresEventStore。
- 实例化 SessionLifecycle impl。
- 注入 TurnExecutor。

不得：

- 接入层直接操作 events 表。

---

## 9. 与参考框架对比

### 9.1 OpenClaw 会话治理模式

观察点：

1. 会话写操作通过统一更新入口，避免并发覆盖。
2. 明确存在 session 写锁与锁队列。
3. compaction 有独立流程和状态信号。
4. 归档采用“逻辑删除 + transcript 归档文件”治理思路。

可借鉴点：

- `updateSessionStore` 式“锁内重读再写”。
- 锁超时与 stale lock 清理机制。
- 压缩失败可恢复，不直接破坏会话。

本设计取舍：

- 保留“统一写入口”思想。
- 用 DB + trait 实现，而非 JSON store 文件。
- 将 compaction 语义提升为事件模型（Summary/Marker）。

### 9.2 PicoClaw SessionManager

观察点：

1. `SessionManager` 用 `sync.RWMutex` 管理 in-memory session。
2. 提供 `GetOrCreate/AddMessage/GetHistory/SetSummary/TruncateHistory`。
3. `maybeSummarize` 在阈值触发异步摘要。
4. 压缩本质是“截断旧历史 + 写摘要”。

优点：

- 简单直接。
- MVP 速度快。

局限：

- 单进程语义。
- 事件不可审计重放。
- 压缩后区间语义不显式。

本设计取舍：

- 保留其“阈值触发 + 保留最近窗口”MVP 思路。
- 但使用事件流化表达，支持审计与跨进程一致性。

### 9.3 ZeroClaw 相关启发

ZeroClaw 在会话层面更多体现为：

- 历史压缩与上下文控制配合。
- 工具结果超限时上下文防护。
- 会话清理/归档配置化。

本设计吸收点：

- 压缩是上下文治理的一部分，而非孤立功能。
- 压缩与运行时错误恢复（overflow/retry）要联动。

### 9.4 最终取舍总结

本方案定位：

- 比 PicoClaw 更强的一致性与可追溯性。
- 比 OpenClaw 文件型治理更贴近 agent-demo 的 Rust + Postgres 分层。
- 保持 MVP 可落地，不一次性引入过重系统。

---

## 附录 A：状态迁移图（文本）

```text
Create -> Active -> Compacting -> Active -> ...
                     |
                     v
                  Archived

Archived --(restore)--> Active
```

约束：

- Compacting 不直接进入 Archived。
- Archived 默认不接收常规 turn 写入。

---

## 附录 B：Turn 与 Lifecycle 时序（文本）

```text
Channel Handler
  -> TurnExecutor.run_turn
      -> SessionLifecycle.load_or_create
      -> ContextBuilder.build
      -> LlmProvider.complete
      -> ToolRuntime.execute (optional loop)
      -> SessionLifecycle.persist_turn
      -> SessionLifecycle.compact_if_needed
      -> SessionLifecycle.archive_if_needed
  -> return response
```

---

## 附录 C：MVP 参数建议

- `compact_token_ratio = 0.70`
- `compact_event_threshold = 120`
- `keep_recent_events = 24`
- `archive_inactive_days = 30`
- `retry_max_attempts = 3`
- `retry_backoff_ms = [20, 50, 100]`

---

## 附录 D：与现有 schema 的衔接建议

当前 schema（migration）现状：

- `sessions` 有 `archived` 布尔字段。
- `messages` 以 role/content 存储。
- `messages.sequence_num` 为全局 BIGSERIAL。

为适配本设计，建议后续增量迁移：

1. 新增 `events` 表（session 内 sequence）。
2. 扩展 `sessions`：
   - `status`
   - `version`
   - `event_count`
   - `last_sequence`
   - `estimated_prompt_tokens`
   - `compacted_until_sequence`
3. `messages` 逐步降级为投影表（可选）。

---

## 附录 E：验收映射

对应任务要求 1（设计目标）：

- 已覆盖第 1 章。

对应任务要求 2（Session 状态模型）：

- 已覆盖第 2 章（含 struct + enum）。

对应任务要求 3（EventStore trait）：

- 已覆盖第 3 章（含指定 6 个方法 + 伪代码）。

对应任务要求 4（SessionLifecycle）：

- 已覆盖第 4 章（含 4 个核心方法 + 与 TurnExecutor 关系）。

对应任务要求 5（压缩策略）：

- 已覆盖第 5 章（触发、流程、MVP、并发保护）。

对应任务要求 6（并发控制）：

- 已覆盖第 6 章（乐观/悲观对比 + MVP 选择）。

对应任务要求 7（归档策略）：

- 已覆盖第 7 章（触发、访问模型、MVP 预留）。

对应任务要求 8（层关系）：

- 已覆盖第 8 章。

对应任务要求 9（参考框架对比）：

- 已覆盖第 9 章（OpenClaw / PicoClaw / ZeroClaw）。

---

## 结论

本设计将会话治理从“消息列表存储”提升为“事件流生命周期管理”。

核心收益：

- 生命周期明确。
- 并发语义可验证。
- 与 ContextBuilder、EventModel 天然契合。
- MVP 可先落地，再演进归档与高级摘要。

下一步实现建议：

1. 在 `agent-domain` 落 Session/EventStore trait。
2. 在 `agent-orchestrator` 落 SessionLifecycle 默认实现。
3. 在 `agent-storage` 落 PostgresEventStore。
4. 将 TurnExecutor 接入 Lifecycle 主链路。
