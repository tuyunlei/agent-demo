# gRPC 协议详细设计（`design/protocols/grpc-services.md`）

> 适用范围：多用户托管式 AI Agent SaaS 平台（Backend: Rust + tonic）  
> 设计基线：ADR-003（gRPC 统一协议）、ADR-006（核心服务划分）  
> 版本：`v1`（MVP + 可演进预留）

---

## 1. 服务总览

本协议按 **Application Service** 边界定义，不在传输层做领域抽象。`v1` 包含 7 个服务：

1. `AuthService`：注册、登录、令牌刷新、登出
2. `ChatService`：消息发送、事件订阅、客户端工具结果提交
3. `AgentService`：Agent CRUD + Persona 配置
4. `SessionService`：会话 CRUD + 历史消息分页查询
5. `MemoryService`：记忆列表/详情/更新/删除
6. `UserService`：用户资料管理 + 设备会话管理
7. （通用）`Common Types`：分页、错误、内容块等跨服务复用

### 包与版本策略

- 包名采用 `ai.agent.platform.v1`。
- `v1` 内新增字段遵循向后兼容（仅追加字段、不复用 tag）。
- 破坏性变更通过 `v2` 新包发布。

---

## 2. 完整 Proto3 定义（内嵌）

> 说明：以下为建议的单文件汇总视图。实际工程可拆分为 `common.proto` / `auth.proto` / `chat.proto` 等，并通过 `import` 复用。

```proto
syntax = "proto3";

package ai.agent.platform.v1;

import "google/protobuf/timestamp.proto";
import "google/protobuf/struct.proto";

option go_package = "ai/agent/platform/v1;platformv1";

// =========================
// 通用枚举与类型
// =========================

// 统一业务错误码（与 gRPC status 配合使用）
enum ErrorCode {
  ERROR_CODE_UNSPECIFIED = 0;        // 未指定
  ERROR_CODE_INVALID_ARGUMENT = 1;   // 参数非法
  ERROR_CODE_UNAUTHENTICATED = 2;    // 未认证
  ERROR_CODE_PERMISSION_DENIED = 3;  // 无权限
  ERROR_CODE_NOT_FOUND = 4;          // 资源不存在
  ERROR_CODE_CONFLICT = 5;           // 资源冲突
  ERROR_CODE_RATE_LIMITED = 6;       // 触发限流
  ERROR_CODE_QUOTA_EXCEEDED = 7;     // 配额不足
  ERROR_CODE_PRECONDITION_FAILED = 8;// 前置条件不满足
  ERROR_CODE_INTERNAL = 9;           // 服务内部错误
  ERROR_CODE_UNAVAILABLE = 10;       // 依赖不可用
  ERROR_CODE_TIMEOUT = 11;           // 超时
  ERROR_CODE_TOOL_EXECUTION_FAILED = 12; // 工具调用失败
  ERROR_CODE_MODEL_EXECUTION_FAILED = 13; // 模型推理失败
}

// 业务错误详情（建议通过 grpc-status-details 传递）
message ErrorDetail {
  ErrorCode code = 1;                // 业务错误码
  string message = 2;                // 人类可读错误信息
  string request_id = 3;             // 请求追踪 ID
  map<string, string> metadata = 4;  // 扩展上下文
}

// 分页请求
message PaginationRequest {
  int32 page_size = 1;               // 页大小（建议默认 20，最大 100）
  string page_token = 2;             // 分页游标（首请求为空）
}

// 分页响应
message PaginationResponse {
  string next_page_token = 1;        // 下一页游标（空表示结束）
  int64 total_size = 2;              // 总量（可选，未知时为 0）
}

// 排序方向
enum SortOrder {
  SORT_ORDER_UNSPECIFIED = 0;        // 默认（服务端决定）
  SORT_ORDER_ASC = 1;                // 升序
  SORT_ORDER_DESC = 2;               // 降序
}

// 内容块类型（统一消息内容承载）
message ContentBlock {
  oneof kind {
    TextBlock text = 1;              // 文本块
    ImageBlock image = 2;            // 图片块（URL / 对象引用）
    ToolCallBlock tool_call = 3;     // 工具调用块
    ToolResultBlock tool_result = 4; // 工具结果块
  }
}

message TextBlock {
  string text = 1;                   // 文本内容
}

message ImageBlock {
  string uri = 1;                    // 图片 URI（https://... 或对象存储 URI）
  string mime_type = 2;              // MIME 类型（如 image/png）
  string alt = 3;                    // 辅助描述
}

message ToolCallBlock {
  string tool_call_id = 1;           // 工具调用 ID（同一轮唯一）
  string tool_name = 2;              // 工具名
  string arguments_json = 3;         // 入参 JSON
}

message ToolResultBlock {
  string tool_call_id = 1;           // 对应工具调用 ID
  string result_json = 2;            // 结果 JSON
  bool is_error = 3;                 // 是否错误结果
}

// =========================
// 领域实体（对外 DTO）
// =========================

message Persona {
  string role = 1;                   // 人设角色（如 “学习教练”）
  string tone = 2;                   // 语气风格（如 “友好、专业”）
  repeated string traits = 3;        // 人设特征标签
  string instruction = 4;            // 高层系统指令（已清洗）
}

message Agent {
  string agent_id = 1;               // Agent ID
  string user_id = 2;                // 所属用户 ID
  string name = 3;                   // Agent 名称
  string description = 4;            // Agent 描述
  Persona persona = 5;               // 人设配置
  string model = 6;                  // 默认模型标识
  bool archived = 7;                 // 是否归档
  google.protobuf.Timestamp created_at = 8; // 创建时间
  google.protobuf.Timestamp updated_at = 9; // 更新时间
}

message ChatMessage {
  string message_id = 1;             // 消息 ID
  string session_id = 2;             // 所属会话 ID
  string role = 3;                   // 角色（user/assistant/tool/system）
  repeated ContentBlock blocks = 4;  // 消息内容块
  google.protobuf.Timestamp created_at = 5; // 创建时间
}

message Session {
  string session_id = 1;             // 会话 ID
  string user_id = 2;                // 用户 ID
  string agent_id = 3;               // 绑定 Agent ID
  string title = 4;                  // 会话标题
  string summary = 5;                // 会话摘要（可异步生成）
  google.protobuf.Timestamp created_at = 6; // 创建时间
  google.protobuf.Timestamp updated_at = 7; // 更新时间
  google.protobuf.Timestamp last_message_at = 8; // 最后消息时间
  bool archived = 9;                 // 是否归档
}

message Memory {
  string memory_id = 1;              // 记忆 ID
  string user_id = 2;                // 所属用户 ID
  string agent_id = 3;               // 关联 Agent ID（可选）
  string session_id = 4;             // 来源会话 ID（可选）
  string content = 5;                // 记忆内容（文本）
  repeated string tags = 6;          // 标签
  float importance = 7;              // 重要度（0~1）
  google.protobuf.Timestamp created_at = 8; // 创建时间
  google.protobuf.Timestamp updated_at = 9; // 更新时间
}

message UserProfile {
  string user_id = 1;                // 用户 ID
  string email = 2;                  // 邮箱（登录账号）
  string display_name = 3;           // 显示名
  string avatar_url = 4;             // 头像 URL
  string locale = 5;                 // 语言区域（如 zh-CN）
  string timezone = 6;               // 时区（如 Asia/Shanghai）
  google.protobuf.Timestamp created_at = 7; // 创建时间
  google.protobuf.Timestamp updated_at = 8; // 更新时间
}

message DeviceSession {
  string device_session_id = 1;      // 设备会话 ID
  string user_id = 2;                // 用户 ID
  string device_name = 3;            // 设备名称
  string platform = 4;               // 平台（ios/android/web/desktop）
  string ip = 5;                     // 最近访问 IP（脱敏）
  string user_agent = 6;             // UA 摘要
  google.protobuf.Timestamp created_at = 7; // 会话创建时间
  google.protobuf.Timestamp last_seen_at = 8; // 最近活跃时间
  bool current = 9;                  // 是否当前设备会话
}

// =========================
// AuthService
// =========================

service AuthService {
  rpc Register(RegisterRequest) returns (RegisterResponse);
  rpc Login(LoginRequest) returns (LoginResponse);
  rpc RefreshToken(RefreshTokenRequest) returns (RefreshTokenResponse);
  rpc Logout(LogoutRequest) returns (LogoutResponse);

  // 预留演进：
  // rpc VerifyEmail(VerifyEmailRequest) returns (VerifyEmailResponse);
  // rpc ResetPassword(ResetPasswordRequest) returns (ResetPasswordResponse);
}

message RegisterRequest {
  string invite_code = 1;            // 邀请码（MVP 必填）
  string email = 2;                  // 注册邮箱
  string password = 3;               // 注册密码（明文经 TLS 传输）
  string display_name = 4;           // 显示名
}

message RegisterResponse {
  string user_id = 1;                // 新建用户 ID
  TokenPair token_pair = 2;          // 登录态令牌
}

message LoginRequest {
  string email = 1;                  // 登录邮箱
  string password = 2;               // 登录密码
  string device_name = 3;            // 设备名（用于会话管理）
  string platform = 4;               // 平台标识
}

message LoginResponse {
  string user_id = 1;                // 用户 ID
  TokenPair token_pair = 2;          // 登录态令牌
}

message RefreshTokenRequest {
  string refresh_token = 1;          // 刷新令牌
}

message RefreshTokenResponse {
  TokenPair token_pair = 1;          // 新令牌对
}

message LogoutRequest {
  string refresh_token = 1;          // 待注销刷新令牌（或当前会话）
  bool logout_all_devices = 2;       // 是否登出全部设备
}

message LogoutResponse {
  bool success = 1;                  // 操作是否成功
}

message TokenPair {
  string access_token = 1;           // 访问令牌（短期）
  string refresh_token = 2;          // 刷新令牌（长期）
  google.protobuf.Timestamp access_token_expires_at = 3; // access 过期时间
  google.protobuf.Timestamp refresh_token_expires_at = 4; // refresh 过期时间
}

// =========================
// ChatService
// =========================
//
// 设计说明（2026-02-17 更新）：
// MVP 产品形态类似 IM 聊天软件，按 boundary 分块而非逐字符流式。
// 三接口设计，不使用双向流：
//   1. SendMessage  — unary，用户发消息，服务端返回确认
//   2. Subscribe    — server streaming，客户端订阅会话事件推送
//   3. SubmitToolResult — unary，客户端工具执行完后提交结果
//
// client-side tools 通过 Subscribe 收到 ToolRequest，执行后调 SubmitToolResult，
// 服务端通过 pending map（key=tool_call_id）协调等待，无需双向流。

service ChatService {
  // 用户发送消息，服务端接受后立即返回确认（异步处理）
  rpc SendMessage(SendMessageRequest) returns (SendMessageResponse);

  // 客户端订阅会话事件流（AI 回复分块、工具调用请求、完成通知等）
  rpc Subscribe(SubscribeRequest) returns (stream ChatEvent);

  // 客户端提交 client-side tool 执行结果
  rpc SubmitToolResult(SubmitToolResultRequest) returns (SubmitToolResultResponse);
}

// --- SendMessage ---

message SendMessageRequest {
  string request_id = 1;                     // 幂等 ID，重发保证只处理一次
  string session_id = 2;                     // 会话 ID（空时服务端自动创建）
  string agent_id = 3;                       // Agent ID
  repeated ContentBlock content = 4;         // 用户输入内容块（文字/图片等）
  map<string, string> metadata = 5;          // 客户端附加上下文（设备类型等）
}

message SendMessageResponse {
  string request_id = 1;                     // 回传 request_id 供客户端追踪
  string session_id = 2;                     // 确认或新建的会话 ID
  string user_message_id = 3;               // 已入库的用户消息 ID
}

// --- Subscribe ---

message SubscribeRequest {
  string session_id = 1;                     // 要订阅的会话 ID
  string last_event_id = 2;                  // 断线重连时传入，服务端从此 event 后续推
}

// 服务端 -> 客户端事件（按 boundary 分块，不是逐字符）
message ChatEvent {
  string event_id = 1;                       // 单调递增，用于断线重连去重
  oneof event {
    TextChunk text_chunk = 2;               // 一段完整文字（工具调用前/后的文字块）
    ToolRequest tool_request = 3;           // 请求客户端执行 client-side tool
    RoundComplete round_complete = 4;       // 一轮 agent 循环完成
    ChatError error = 5;                    // 错误（含是否可重试）
  }
}

message TextChunk {
  string message_id = 1;                    // assistant 消息 ID（同一消息可有多个 chunk）
  string text = 2;                          // 文字内容（按 boundary 分块，非逐字符）
  bool is_final = 3;                        // 是否为该 message_id 的最后一块
}

message ToolRequest {
  string tool_call_id = 1;                  // 唯一 ID，SubmitToolResult 时原样回传
  string tool_name = 2;                     // 工具名（命名空间化，如 client.calendar.read）
  string arguments_json = 3;               // 参数 JSON
  int32 timeout_ms = 4;                     // 建议的执行超时（客户端参考）
}

message RoundComplete {
  string request_id = 1;                    // 对应 SendMessage 的 request_id
  string last_message_id = 2;              // 本轮最后一条 assistant 消息 ID
  Usage usage = 3;                          // token 消耗统计
}

message ChatError {
  ErrorCode code = 1;                       // 业务错误码
  string message = 2;                       // 错误描述
  bool retryable = 3;                       // 是否可重试
  string request_id = 4;                    // 关联的 request_id（如有）
}

// --- SubmitToolResult ---

message SubmitToolResultRequest {
  string session_id = 1;                    // 会话 ID
  string tool_call_id = 2;                  // 对应 ToolRequest.tool_call_id
  string result_json = 3;                   // 工具执行结果 JSON
  bool is_error = 4;                        // 是否执行失败
  string error_message = 5;                // 失败原因
}

message SubmitToolResultResponse {
  bool accepted = 1;                        // 服务端是否接受（false 表示已超时或 ID 不存在）
}

message Usage {
  int32 prompt_tokens = 1;                   // 输入 token 数
  int32 completion_tokens = 2;               // 输出 token 数
  int32 total_tokens = 3;                    // 总 token 数
}

// =========================
// AgentService
// =========================

service AgentService {
  rpc CreateAgent(CreateAgentRequest) returns (CreateAgentResponse);
  rpc GetAgent(GetAgentRequest) returns (GetAgentResponse);
  rpc ListAgents(ListAgentsRequest) returns (ListAgentsResponse);
  rpc UpdateAgent(UpdateAgentRequest) returns (UpdateAgentResponse);
  rpc DeleteAgent(DeleteAgentRequest) returns (DeleteAgentResponse);

  // 预留演进：
  // rpc CloneAgent(CloneAgentRequest) returns (CloneAgentResponse);
}

message CreateAgentRequest {
  string name = 1;                           // Agent 名称
  string description = 2;                    // Agent 描述
  Persona persona = 3;                       // 人设配置
  string model = 4;                          // 默认模型
}

message CreateAgentResponse {
  Agent agent = 1;                           // 创建后的 Agent
}

message GetAgentRequest {
  string agent_id = 1;                       // Agent ID
}

message GetAgentResponse {
  Agent agent = 1;                           // Agent 详情
}

message ListAgentsRequest {
  PaginationRequest pagination = 1;          // 分页参数
  bool include_archived = 2;                 // 是否包含归档
}

message ListAgentsResponse {
  repeated Agent agents = 1;                 // Agent 列表
  PaginationResponse pagination = 2;         // 分页信息
}

message UpdateAgentRequest {
  string agent_id = 1;                       // Agent ID
  optional string name = 2;                  // 新名称
  optional string description = 3;           // 新描述
  Persona persona = 4;                       // 新人设（整对象替换）
  optional string model = 5;                 // 新模型
  optional bool archived = 6;                // 归档状态
}

message UpdateAgentResponse {
  Agent agent = 1;                           // 更新后的 Agent
}

message DeleteAgentRequest {
  string agent_id = 1;                       // Agent ID
  bool hard_delete = 2;                      // 是否硬删除（默认软删）
}

message DeleteAgentResponse {
  bool success = 1;                          // 删除是否成功
}

// =========================
// SessionService
// =========================

service SessionService {
  rpc CreateSession(CreateSessionRequest) returns (CreateSessionResponse);
  rpc GetSession(GetSessionRequest) returns (GetSessionResponse);
  rpc ListSessions(ListSessionsRequest) returns (ListSessionsResponse);
  rpc UpdateSession(UpdateSessionRequest) returns (UpdateSessionResponse);
  rpc DeleteSession(DeleteSessionRequest) returns (DeleteSessionResponse);
  rpc ListSessionMessages(ListSessionMessagesRequest) returns (ListSessionMessagesResponse);

  // 预留演进：
  // rpc SummarizeSession(SummarizeSessionRequest) returns (SummarizeSessionResponse);
}

message CreateSessionRequest {
  string agent_id = 1;                       // Agent ID
  string title = 2;                          // 初始标题（可空）
}

message CreateSessionResponse {
  Session session = 1;                       // 创建后的会话
}

message GetSessionRequest {
  string session_id = 1;                     // 会话 ID
}

message GetSessionResponse {
  Session session = 1;                       // 会话详情
}

message ListSessionsRequest {
  PaginationRequest pagination = 1;          // 分页参数
  string agent_id = 2;                       // 按 Agent 过滤（可选）
  bool include_archived = 3;                 // 是否包含归档
  SortOrder updated_at_order = 4;            // 按更新时间排序
}

message ListSessionsResponse {
  repeated Session sessions = 1;             // 会话列表
  PaginationResponse pagination = 2;         // 分页信息
}

message UpdateSessionRequest {
  string session_id = 1;                     // 会话 ID
  optional string title = 2;                 // 新标题
  optional bool archived = 3;                // 归档状态
}

message UpdateSessionResponse {
  Session session = 1;                       // 更新后的会话
}

message DeleteSessionRequest {
  string session_id = 1;                     // 会话 ID
  bool hard_delete = 2;                      // 是否硬删除（默认软删）
}

message DeleteSessionResponse {
  bool success = 1;                          // 删除是否成功
}

message ListSessionMessagesRequest {
  string session_id = 1;                     // 会话 ID
  PaginationRequest pagination = 2;          // 分页参数
  SortOrder created_at_order = 3;            // 时间排序
}

message ListSessionMessagesResponse {
  repeated ChatMessage messages = 1;         // 历史消息
  PaginationResponse pagination = 2;         // 分页信息
}

// =========================
// MemoryService
// =========================

service MemoryService {
  rpc ListMemories(ListMemoriesRequest) returns (ListMemoriesResponse);
  rpc GetMemory(GetMemoryRequest) returns (GetMemoryResponse);
  rpc UpdateMemory(UpdateMemoryRequest) returns (UpdateMemoryResponse);
  rpc DeleteMemory(DeleteMemoryRequest) returns (DeleteMemoryResponse);

  // 预留演进：
  // rpc CreateMemory(CreateMemoryRequest) returns (CreateMemoryResponse);
  // rpc SearchMemories(SearchMemoriesRequest) returns (SearchMemoriesResponse);
}

message ListMemoriesRequest {
  PaginationRequest pagination = 1;          // 分页参数
  string agent_id = 2;                       // 按 Agent 过滤（可选）
  string session_id = 3;                     // 按会话过滤（可选）
  repeated string tags = 4;                  // 标签过滤（交集语义由服务端定义）
}

message ListMemoriesResponse {
  repeated Memory memories = 1;              // 记忆列表
  PaginationResponse pagination = 2;         // 分页信息
}

message GetMemoryRequest {
  string memory_id = 1;                      // 记忆 ID
}

message GetMemoryResponse {
  Memory memory = 1;                         // 记忆详情
}

message UpdateMemoryRequest {
  string memory_id = 1;                      // 记忆 ID
  optional string content = 2;               // 新内容
  repeated string tags = 3;                  // 新标签（整列表替换）
  optional float importance = 4;             // 新重要度
}

message UpdateMemoryResponse {
  Memory memory = 1;                         // 更新后的记忆
}

message DeleteMemoryRequest {
  string memory_id = 1;                      // 记忆 ID
}

message DeleteMemoryResponse {
  bool success = 1;                          // 删除是否成功
}

// =========================
// UserService
// =========================

service UserService {
  rpc GetProfile(GetProfileRequest) returns (GetProfileResponse);
  rpc UpdateProfile(UpdateProfileRequest) returns (UpdateProfileResponse);
  rpc ListDeviceSessions(ListDeviceSessionsRequest) returns (ListDeviceSessionsResponse);
  rpc RevokeSession(RevokeSessionRequest) returns (RevokeSessionResponse);

  // 预留演进：
  // rpc DeleteAccount(DeleteAccountRequest) returns (DeleteAccountResponse);
}

message GetProfileRequest {}

message GetProfileResponse {
  UserProfile profile = 1;                   // 当前用户资料
}

message UpdateProfileRequest {
  optional string display_name = 1;          // 显示名
  optional string avatar_url = 2;            // 头像 URL
  optional string locale = 3;                // 语言区域
  optional string timezone = 4;              // 时区
}

message UpdateProfileResponse {
  UserProfile profile = 1;                   // 更新后资料
}

message ListDeviceSessionsRequest {
  PaginationRequest pagination = 1;          // 分页参数
}

message ListDeviceSessionsResponse {
  repeated DeviceSession sessions = 1;       // 设备会话列表
  PaginationResponse pagination = 2;         // 分页信息
}

message RevokeSessionRequest {
  string device_session_id = 1;              // 待撤销会话 ID
}

message RevokeSessionResponse {
  bool success = 1;                          // 撤销是否成功
}
```

---

## 3. 各服务设计说明（MVP 与演进）

### 3.1 AuthService

- **MVP**：`Register / Login / RefreshToken / Logout`
- 登录成功返回 `TokenPair`，用于客户端续期与鉴权。
- `Logout` 支持单设备与全设备登出。
- 演进可加邮箱验证、密码重置、MFA。

### 3.2 ChatService

MVP 产品形态类似 IM 聊天软件，按 boundary 分块返回，不做逐字符流式。采用三接口设计：

- `SendMessage`（unary）：用户发消息，服务端异步处理，立即返回确认
- `Subscribe`（server streaming）：客户端订阅会话事件，服务端推送 `ChatEvent`
  - `TextChunk`：按 boundary 分块的文字（工具调用前/后各成一块）
  - `ToolRequest`：请求客户端执行 client-side tool
  - `RoundComplete`：一轮 agent 循环结束
  - `ChatError`：错误事件
- `SubmitToolResult`（unary）：客户端工具执行完提交结果；服务端通过 `pending map`（key=`tool_call_id`）协调等待，无需双向流

`last_event_id` 字段支持断线重连后的事件续传（服务端从断点后推）。

### 3.3 AgentService

- 在 ADR-006 基础上补全 `DeleteAgent`，形成完整 CRUD。
- `Persona` 作为独立消息对象，避免重复定义。
- 软删优先，硬删需要显式 `hard_delete=true`。

### 3.4 SessionService

- 在 ADR-006 基础上补全 `UpdateSession` 与 `ListSessionMessages`。
- 支持会话元信息管理（title、archived）与消息历史分页。

### 3.5 MemoryService

- MVP 覆盖：`List/Get/Update/Delete`。
- `Create/Search` 为演进接口（可由系统自动写入，后续对用户开放）。

### 3.6 UserService

- 资料：`GetProfile/UpdateProfile`
- 安全：`ListDeviceSessions/RevokeSession`
- 用于支持多端登录态治理与账户自助安全能力。

---

## 4. 通用类型定义约束

1. **时间统一**：全部使用 `google.protobuf.Timestamp`。
2. **分页统一**：全部使用 `PaginationRequest/PaginationResponse`（游标式分页）。
3. **内容统一**：聊天内容统一封装 `ContentBlock`，避免文本/多模态/工具结构分裂。
4. **错误统一**：业务错误用 `ErrorCode`，并与 gRPC status code 映射。

---

## 5. 错误处理约定（gRPC status + 业务错误码）

### 5.1 双层错误模型

- **传输/协议层**：使用 gRPC `status code`（如 `INVALID_ARGUMENT`、`UNAUTHENTICATED`）。
- **业务层**：使用 `ErrorCode`（建议放入 status details 的 `ErrorDetail`）。

### 5.2 推荐映射

| gRPC Status Code | ErrorCode（示例） | 典型场景 |
|---|---|---|
| `INVALID_ARGUMENT` | `ERROR_CODE_INVALID_ARGUMENT` | 参数校验失败 |
| `UNAUTHENTICATED` | `ERROR_CODE_UNAUTHENTICATED` | access token 无效/过期 |
| `PERMISSION_DENIED` | `ERROR_CODE_PERMISSION_DENIED` | 越权访问他人资源 |
| `NOT_FOUND` | `ERROR_CODE_NOT_FOUND` | agent/session/memory 不存在 |
| `ALREADY_EXISTS` | `ERROR_CODE_CONFLICT` | 邮箱已注册、资源名冲突 |
| `FAILED_PRECONDITION` | `ERROR_CODE_PRECONDITION_FAILED` | 资源状态不允许当前操作 |
| `RESOURCE_EXHAUSTED` | `ERROR_CODE_RATE_LIMITED` / `ERROR_CODE_QUOTA_EXCEEDED` | 限流或配额不足 |
| `DEADLINE_EXCEEDED` | `ERROR_CODE_TIMEOUT` | 下游超时 |
| `UNAVAILABLE` | `ERROR_CODE_UNAVAILABLE` | 依赖服务不可用 |
| `INTERNAL` | `ERROR_CODE_INTERNAL` | 未分类内部错误 |

### 5.3 Chat 流式错误建议

- 可恢复错误：在流内发送 `ChatEvent.error`（`retryable=true`）并尽量保持流语义清晰。
- 不可恢复错误：返回 gRPC 非 OK 状态并结束流。

---

## 6. 协议演进策略（向后兼容）

1. **字段演进**
   - 仅追加字段，不删除已发布字段。
   - 不复用已使用 tag。
   - 废弃字段标注 `deprecated = true`，并保留至少一个次版本周期。

2. **oneof 演进**
   - 可新增 oneof 分支；客户端必须容忍未知分支。
   - 禁止改变既有分支语义。

3. **枚举演进**
   - `0` 保留为 `UNSPECIFIED`。
   - 仅新增枚举值，不重排、不改含义。

4. **服务与方法演进**
   - MVP 方法保持稳定。
   - 新能力优先新增 rpc；破坏性变更通过 `package ...v2` 发布。

5. **兼容性验证建议**
   - CI 中加入 proto breaking check（如 buf breaking）。
   - 每次发布产出变更日志：新增字段、弃用字段、行为变更说明。

---

## 7. 与既有设计一致性检查（验收映射）

- [x] 覆盖 ADR-006 四个核心服务及方法（并补全 CRUD 缺口）
- [x] `ChatEvent` oneof 包含：`TextChunk / ToolRequest / RoundComplete / ChatError`（三接口设计，无双向流）
- [x] 新增 `MemoryService` 与 `UserService` 满足模块需求
- [x] 通用类型（Timestamp/Pagination/Error/ContentBlock）统一
- [x] 明确 gRPC status 与业务错误码协作方式
- [x] 提供 v1→v2 协议演进策略

---

## 8. 非目标说明

本文档不包含：

- Rust/tonic 代码实现细节
- 传输层网关、负载均衡、重试中间件设计
- 具体鉴权 token 格式（JWT claims 结构等）

以上内容在实现与部署文档中定义。
