# D05 — 核心 Port Trait 精确定义（agent-domain）

> 目标：定义 Runtime 依赖的 5 个核心 Port trait，作为 Domain/Application 对 Infrastructure 的稳定契约。
> 约束：Rust 伪代码；统一错误 `DomainError`；全部 `#[async_trait]`；运行时持有方式为 `Arc<dyn Trait + Send + Sync>`。

---

## 0. 通用类型约定

```rust
use async_trait::async_trait;
use futures_core::Stream;
use std::pin::Pin;

pub type UserId = uuid::Uuid;
pub type AgentId = uuid::Uuid;
pub type SessionId = uuid::Uuid;
pub type MessageId = uuid::Uuid;
pub type RequestId = String;
pub type Cursor = String;

pub type DynTokenStream =
    Pin<Box<dyn Stream<Item = Result<LlmStreamEvent, DomainError>> + Send + 'static>>;

#[derive(Debug, Clone)]
pub enum DomainError {
    Validation(String),
    NotFound,
    Conflict(String),
    Cancelled,
    Timeout,
    Persistence(String),
    Provider(String),
    Internal(String),
}
```

---

## 1. LlmProvider Port

**职责**：封装对 LLM provider 的调用，产出可取消的流式事件（token/tool-call/usage/end）。

```rust
#[derive(Debug, Clone)]
pub struct LlmGenerateRequest {
    pub request_id: RequestId,
    pub user_id: UserId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
    pub messages: Vec<LlmChatMessage>,
    pub tools: Vec<LlmToolSpec>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum LlmStreamEvent {
    TextDelta { text: String },
    ToolCallStart { call_id: String, name: String },
    ToolCallDelta { call_id: String, arguments_delta: String },
    ToolCallEnd { call_id: String },
    Usage { usage: LlmUsage },
    Done { finish_reason: FinishReason },
}

#[derive(Debug, Clone)]
pub struct LlmUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub enum FinishReason {
    Stop,
    ToolCall,
    Length,
    ContentFilter,
    Error,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 发起一次流式生成；drop 返回的 stream 即视为取消请求。
    async fn stream_generate(&self, req: LlmGenerateRequest)
        -> Result<DynTokenStream, DomainError>;

    /// 可选能力探测（用于启动时健康检查/能力降级）。
    async fn supports_model(&self, model: &str) -> Result<bool, DomainError>;
}
```

方法语义：
- `stream_generate`：开始一次 LLM 流式推理，按顺序返回文本增量、工具调用事件、usage 与结束事件。
- `supports_model`：检查 provider 是否支持给定 model。

---

## 2. MessageStore Port

**职责**：会话消息历史读写，包含分页、批量写入与软删除；所有接口强制 `user_id` 租户隔离。

```rust
#[derive(Debug, Clone)]
pub enum MessageRole { User, Assistant, System, Tool }

#[derive(Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub user_id: UserId,
    pub agent_id: AgentId,
    pub session_id: SessionId,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: serde_json::Value,
    pub tool_result: serde_json::Value,
    pub token_count: u32,
    pub is_deleted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct NewMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: serde_json::Value,
    pub tool_result: serde_json::Value,
    pub token_count: u32,
}

#[derive(Debug, Clone)]
pub struct MessagePage {
    pub items: Vec<Message>,
    pub next_cursor: Option<Cursor>,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub struct MessageQuery {
    pub cursor: Option<Cursor>,
    pub limit: u32,
    pub include_deleted: bool,
    pub ascending: bool,
}

#[async_trait]
pub trait MessageStore: Send + Sync {
    /// 分页读取会话消息，必须按 user_id + session_id 限定。
    async fn list_by_session(
        &self,
        user_id: UserId,
        session_id: SessionId,
        query: MessageQuery,
    ) -> Result<MessagePage, DomainError>;

    /// 读取单条消息（用于审计/补偿写）。
    async fn get_by_id(
        &self,
        user_id: UserId,
        session_id: SessionId,
        message_id: MessageId,
    ) -> Result<Option<Message>, DomainError>;

    /// 追加单条消息。
    async fn append_one(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        session_id: SessionId,
        msg: NewMessage,
    ) -> Result<Message, DomainError>;

    /// 批量追加消息（同一 session，顺序保持输入顺序）。
    async fn append_batch(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        session_id: SessionId,
        msgs: Vec<NewMessage>,
    ) -> Result<Vec<Message>, DomainError>;

    /// 软删除消息（is_deleted=true），不做物理删除。
    async fn soft_delete(
        &self,
        user_id: UserId,
        session_id: SessionId,
        message_id: MessageId,
    ) -> Result<(), DomainError>;
}
```

方法语义：
- `list_by_session`：按 cursor/limit 分页读取会话历史。
- `get_by_id`：按租户边界读取单条消息。
- `append_one`：写入单条消息。
- `append_batch`：同事务批量写入消息。
- `soft_delete`：软删除指定消息。

---

## 3. SessionStore Port

**职责**：会话元数据与状态机管理，支持 CAS 更新防并发冲突。

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Active,
    Compacting,
    Archived,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub agent_id: AgentId,
    pub title: String,
    pub summary: String,
    pub state: SessionState,
    pub version: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateSession {
    pub title: String,
    pub initial_state: SessionState,
}

#[derive(Debug, Clone, Default)]
pub struct SessionPatch {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub state: Option<SessionState>,
}

#[derive(Debug, Clone)]
pub struct CasResult<T> {
    pub applied: bool,
    pub current: T,
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    /// 获取会话；不存在返回 None。
    async fn get(
        &self,
        user_id: UserId,
        session_id: SessionId,
    ) -> Result<Option<Session>, DomainError>;

    /// 创建会话。
    async fn create(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        cmd: CreateSession,
    ) -> Result<Session, DomainError>;

    /// 普通更新（非并发敏感路径）。
    async fn update(
        &self,
        user_id: UserId,
        session_id: SessionId,
        patch: SessionPatch,
    ) -> Result<Session, DomainError>;

    /// CAS 更新：仅当 version == expected_version 时应用 patch。
    async fn compare_and_swap(
        &self,
        user_id: UserId,
        session_id: SessionId,
        expected_version: u64,
        patch: SessionPatch,
    ) -> Result<CasResult<Session>, DomainError>;
}
```

方法语义：
- `get`：按租户读取会话。
- `create`：创建新会话并初始化状态机状态。
- `update`：更新标题/摘要/状态。
- `compare_and_swap`：并发安全更新，避免覆盖其他 actor 写入。

---

## 4. MemoryStore Port

**职责**：管理记忆摘要读取与更新；预留向量检索接口（MVP 可返回空）。

```rust
#[derive(Debug, Clone)]
pub struct MemorySummary {
    pub user_id: UserId,
    pub agent_id: AgentId,
    pub session_id: Option<SessionId>,
    pub content: String,
    pub version: u64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct UpsertMemorySummary {
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct MemoryHit {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// 读取某 session 的记忆摘要（会话级）。
    async fn get_session_summary(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        session_id: SessionId,
    ) -> Result<Option<MemorySummary>, DomainError>;

    /// 读取某 agent 的全局记忆摘要（agent 级）。
    async fn get_agent_summary(
        &self,
        user_id: UserId,
        agent_id: AgentId,
    ) -> Result<Option<MemorySummary>, DomainError>;

    /// 写入或更新 session 级摘要（PostProcessor 在 turn 完成后调用）。
    async fn upsert_session_summary(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        session_id: SessionId,
        data: UpsertMemorySummary,
    ) -> Result<MemorySummary, DomainError>;

    /// 写入或更新 agent 级摘要。
    async fn upsert_agent_summary(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        data: UpsertMemorySummary,
    ) -> Result<MemorySummary, DomainError>;

    /// 语义检索预留接口（MVP 可返回空列表）。
    async fn search_semantic(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        query: &str,
        top_k: u32,
    ) -> Result<Vec<MemoryHit>, DomainError>;
}
```

方法语义：
- `get_session_summary`：按会话读取摘要。
- `get_agent_summary`：按 agent 读取全局摘要。
- `upsert_session_summary`：写入/更新会话摘要。
- `upsert_agent_summary`：写入/更新 agent 摘要。
- `search_semantic`：向量检索扩展点。

---

## 5. PersonaStore Port

**职责**：只读读取人格配置；支持 active 版本和指定版本读取。

```rust
#[derive(Debug, Clone)]
pub struct PersonaConfig {
    pub agent_id: AgentId,
    pub version: String,
    pub name: String,
    pub system_prompt: String,
    pub style: serde_json::Value,
    pub safety_rules: Vec<String>,
    pub is_active: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait PersonaStore: Send + Sync {
    /// 读取 agent 当前 active 人格。
    async fn get_active(
        &self,
        user_id: UserId,
        agent_id: AgentId,
    ) -> Result<Option<PersonaConfig>, DomainError>;

    /// 读取指定版本人格。
    async fn get_by_version(
        &self,
        user_id: UserId,
        agent_id: AgentId,
        version: &str,
    ) -> Result<Option<PersonaConfig>, DomainError>;

    /// 列出该 agent 可用人格版本（含 active 标记）。
    async fn list_versions(
        &self,
        user_id: UserId,
        agent_id: AgentId,
    ) -> Result<Vec<PersonaConfig>, DomainError>;
}
```

方法语义：
- `get_active`：取当前生效人格。
- `get_by_version`：按版本读取人格快照。
- `list_versions`：列出可选版本（用于管理或回滚）。

---

## 6. 装配与使用约束

```rust
pub struct RuntimeDeps {
    pub llm: std::sync::Arc<dyn LlmProvider>,
    pub messages: std::sync::Arc<dyn MessageStore>,
    pub sessions: std::sync::Arc<dyn SessionStore>,
    pub memories: std::sync::Arc<dyn MemoryStore>,
    pub personas: std::sync::Arc<dyn PersonaStore>,
}
```

- Runtime 内部模块通过以上 trait 依赖端口，不感知 adapter 细节。
- 错误统一上抛为 `DomainError`，D06 再细化映射策略。
- `LlmProvider` 的取消语义：上层停止消费并 drop stream，即触发 provider 端取消。

---

## 7. 方法数量汇总

- `LlmProvider`：**2** 个方法
- `MessageStore`：**5** 个方法
- `SessionStore`：**4** 个方法
- `MemoryStore`：**5** 个方法
- `PersonaStore`：**3** 个方法

合计：**19** 个方法。

---

## 8. LlmProvider 流式 token 表达（结论）

采用统一流事件枚举 `LlmStreamEvent` 表达 token 与工具调用：

1. `TextDelta { text }`：文本 token 增量
2. `ToolCallStart { call_id, name }`：工具调用开始
3. `ToolCallDelta { call_id, arguments_delta }`：工具参数流式增量（JSON 片段）
4. `ToolCallEnd { call_id }`：工具调用参数封闭
5. `Usage { usage }`：token 使用量（通常在结尾前后发出）
6. `Done { finish_reason }`：本次生成结束

即：`stream_generate -> Result<DynTokenStream, DomainError>`，消费方按事件驱动组装最终文本与工具调用请求。