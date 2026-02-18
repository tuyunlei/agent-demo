# Channel Port 精确定义（D03）

> 目标：定义一个**协议无关**的 `ChannelAdapter` Port，使 gRPC / WebSocket / Push 等实现都遵循同一套 Application 调用语义。

---

## 1. 设计边界

- 本文只定义 **Port 接口**（trait、类型、错误、语义约束）。
- 不展开具体 adapter 内部实现细节。
- `AgentEvent` 仅保留 MVP 需要的 4 个变体。

---

## 2. ChannelAdapter trait（Rust 伪代码）

```rust
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    type EventStream: Stream<Item = Result<AgentEventEnvelope, ChannelError>>
        + Send
        + Unpin
        + 'static;

    /// 适配器标识（如 "grpc", "websocket", "telegram"）
    fn adapter_id(&self) -> &'static str;

    /// 渠道类型能力声明（native/web/im/api）
    fn channel_kind(&self) -> ChannelKind;

    /// 能力声明（是否流式、最大消息长度、是否支持 tool request 等）
    fn capabilities(&self) -> ChannelCapabilities;

    /// 入站标准化：外部协议请求 -> InboundMessage
    async fn normalize_inbound(
        &self,
        raw: RawInboundRequest,
    ) -> Result<InboundMessage, ChannelError>;

    /// 注册订阅并返回事件流（用于 Subscribe / WS listen）
    async fn subscribe(
        &self,
        req: SubscriptionRequest,
    ) -> Result<Self::EventStream, ChannelError>;

    /// 向 session 的在线连接分发事件（支持单 session 多连接 fanout）
    async fn publish(
        &self,
        envelope: AgentEventEnvelope,
        policy: FanoutPolicy,
    ) -> Result<DeliveryReport, ChannelError>;

    /// 客户端提交工具结果（对应 SubmitToolResult）
    async fn submit_tool_result(
        &self,
        input: ToolResultSubmission,
    ) -> Result<ToolResultAck, ChannelError>;
}
```

### 2.1 持有方式

根据 S01 结论：

```rust
Arc<dyn ChannelAdapter<EventStream = ...> + Send + Sync>
```

---

## 3. 核心类型定义（精确）

## 3.1 InboundMessage

```rust
pub struct InboundMessage {
    pub request_id: String,                  // 幂等键（来自客户端）
    pub message_id: String,                  // 渠道侧消息ID（无则由 adapter 生成）
    pub session_id: String,                  // 统一会话ID（可由上游创建后回填）
    pub agent_id: String,                    // 目标 Agent
    pub user_id: String,                     // 内部用户ID（鉴权后）
    pub channel_type: ChannelKind,           // native/web/im/api
    pub channel_provider: String,            // grpc-ios/websocket/telegram...
    pub channel_user_id: Option<String>,     // 渠道用户ID
    pub channel_conversation_ref: Option<String>, // chat_id/thread_id 等
    pub content: Vec<ContentBlock>,
    pub metadata: BTreeMap<String, String>,
    pub received_at: DateTime<Utc>,
}
```

## 3.2 AgentEvent（MVP）

```rust
pub enum AgentEvent {
    TextChunk(TextChunkEvent),
    ToolRequest(ToolRequestEvent),
    RoundComplete(RoundCompleteEvent),
    ChatError(ChatErrorEvent),
}
```

对应负载：

```rust
pub struct TextChunkEvent {
    pub message_id: String,
    pub text: String,
    pub is_final: bool,
}

pub struct ToolRequestEvent {
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub timeout_ms: i32,
}

pub struct RoundCompleteEvent {
    pub request_id: String,
    pub last_message_id: String,
    pub usage: Usage,
}

pub struct ChatErrorEvent {
    pub error_code: ChannelErrorCode,
    pub message: String,
    pub request_id: Option<String>,
    pub retryable: bool,
}

pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}
```

## 3.3 事件包络与订阅

```rust
pub struct AgentEventEnvelope {
    pub event_id: String,       // session 内单调递增，用于断线续传
    pub session_id: String,
    pub user_id: String,
    pub event: AgentEvent,
    pub emitted_at: DateTime<Utc>,
}

pub struct SubscriptionRequest {
    pub session_id: String,
    pub user_id: String,
    pub connection_id: String,      // 每条连接唯一
    pub last_event_id: Option<String>,
}
```

## 3.4 工具结果提交

```rust
pub struct ToolResultSubmission {
    pub session_id: String,
    pub user_id: String,
    pub tool_call_id: String,
    pub result_json: String,
    pub is_error: bool,
    pub error_message: Option<String>,
}

pub struct ToolResultAck {
    pub accepted: bool,
}
```

## 3.5 分发策略与报告

```rust
pub enum FanoutPolicy {
    AllConnectionsInSession,   // 默认：同 session 所有在线连接都收
    OnlyConnection(String),    // 定向单连接（调试/定向）
}

pub struct DeliveryReport {
    pub session_id: String,
    pub event_id: String,
    pub attempted: u32,
    pub delivered: u32,
    pub dropped_connections: Vec<String>,
}
```

---

## 4. 错误类型

```rust
pub enum ChannelErrorCode {
    InvalidArgument,
    Unauthenticated,
    PermissionDenied,
    NotFound,
    Conflict,
    PreconditionFailed,
    RateLimited,
    Timeout,
    Unavailable,
    Internal,
}

pub struct ChannelError {
    pub code: ChannelErrorCode,
    pub message: String,
    pub request_id: Option<String>,
    pub retryable: bool,
}
```

约束：
- trait 所有异步方法统一 `Result<_, ChannelError>`。
- adapter 必须保证错误可映射到 gRPC `Status` + `ErrorCode`（或其它协议等价语义）。

---

## 5. gRPC ChatService 到 ChannelAdapter 的映射

- `SendMessage`（unary）  
  - gRPC handler 收到 `SendMessageRequest`
  - 调 `normalize_inbound(raw)` 得到 `InboundMessage`
  - 交给 Application / Runtime 继续处理

- `Subscribe`（server streaming）  
  - gRPC handler 把 `session_id + user_id + connection_id + last_event_id` 封装为 `SubscriptionRequest`
  - 调 `subscribe(req)` 获取 `EventStream`
  - 将流中 `AgentEventEnvelope` 映射为 proto `ChatEvent` 并写入 gRPC stream

- `SubmitToolResult`（unary）  
  - gRPC handler 映射为 `ToolResultSubmission`
  - 调 `submit_tool_result(input)`
  - 返回 `accepted`

---

## 6. 多并发订阅（同一用户多设备）与 fanout 规则

以 `(user_id, session_id)` 为订阅分组键：

1. `subscribe` 注册连接到组内（`connection_id` 唯一）。
2. `publish(..., AllConnectionsInSession)` 时，向组内所有在线连接广播。
3. 连接写入失败或背压超时，adapter 可主动移除连接，并在 `DeliveryReport.dropped_connections` 记录。
4. 该策略保证同 session 多设备体验一致；若产品需“活跃端优先”，由上层路由决定是否使用 `OnlyConnection`。

---

## 7. 断线重连语义

- 每个 session 维护单调 `event_id`。
- `SubscriptionRequest.last_event_id` 不为空时：
  1. 先补发 `last_event_id` 之后的事件（来自短期 replay buffer）；
  2. 再切换到实时流。
- 若 `last_event_id` 过旧（超出 buffer）：
  - 返回 `ChannelError { code: NotFound | PreconditionFailed(可映射), retryable: false }` 或渠道等价错误，
  - 客户端需走会话重拉/历史同步。

---

## 8. 最小验收清单

- [x] trait 方法签名精确（async + 参数 + 返回 + 错误）
- [x] `AgentEvent` 完整（4 个 MVP 变体）
- [x] `InboundMessage` 字段精确定义
- [x] 给出 gRPC 三接口映射关系
- [x] 明确并发订阅 fanout 与断线重连处理
