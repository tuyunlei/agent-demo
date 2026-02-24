# gRPC 接入层设计（D-INFRA-01）

**状态**：Draft（MVP 可实施）  
**更新时间**：2026-02-24  
**所属层**：Layer 1 Channel（接入层）  
**适用 crate**：`agent-channel`、`agent-server`  
**关联文档**：

- `docs/design/architecture.md`
- `docs/design/orchestration/turn-executor.md`
- `docs/design/decisions/003-grpc.md`
- `docs/design/decisions/006-grpc-interfaces.md`

---

## 0. 背景与目标

agent-demo 采用四层架构：

1. Layer 1：Channel（协议接入 + 认证鉴权 + DTO 映射）
2. Layer 2：Orchestration（Turn 编排）
3. Layer 3：Capability（能力抽象）
4. Layer 4：Infrastructure（外部系统适配）

本设计文档聚焦 Layer 1 的 gRPC 接入，目标是明确：

1. **边界**：接入层与编排层各做什么
2. **职责**：每个 gRPC handler 的标准处理流程
3. **映射**：proto DTO 与编排层输入输出类型的转换规则
4. **鉴权**：JWT 认证拦截器设计与上下文透传
5. **错误**：领域错误到 gRPC status 的统一映射
6. **组装**：`agent-server` 作为 composition root 的 DI 组装方式
7. **选型**：与参考框架接入层实践的对比与本方案取舍

---

## 1. 接入层职责边界

### 1.1 接入层做什么

Layer 1（gRPC Channel）允许的职责仅包括：

1. **协议解析与协议返回**
   - 接收 gRPC request（proto 生成 DTO）
   - 返回 gRPC response / stream
2. **认证与鉴权**
   - 从 metadata 解析 JWT
   - 校验 token 有效性、过期、签名
   - 提取 `user_id` / `tenant_id` / scopes
   - endpoint 级别鉴权（是否允许访问）
3. **输入验证（协议层）**
   - 字段缺失、空字符串、分页参数范围
   - DTO 语法/格式校验（例如 UUID 字符串是否合法）
4. **DTO ↔ Orchestrator Input/Output 映射**
   - proto request → 编排层输入（`TurnInput` 等）
   - 编排层输出 → proto response
5. **错误映射**
   - 领域/编排错误 → `tonic::Status`
   - 统一错误细节结构（error code + message + request_id）
6. **可观测性透传**
   - request_id、trace_id 读取与补全
   - 写入 tracing span / metrics tag

### 1.2 接入层不做什么

Layer 1 明确禁止：

1. 不做业务决策
   - 不判断“是否触发压缩”
   - 不判断“是否继续 tool loop”
   - 不拼接业务 prompt
2. 不直接依赖 Layer 4 具体实现
   - 不直接操作 SQL/Redis/文件
   - 不直接调用外部 LLM SDK
3. 不承载跨请求流程状态机
   - turn 生命周期逻辑在编排层
4. 不重复实现编排层错误恢复策略
   - 例如 LLM fallback、persist degrade 由编排层定义

### 1.3 与编排层边界

边界约束：

1. 接入层将请求归一化后，调用编排入口
2. 对聊天主路径，接入层只调用 `TurnExecutor::run_turn()`
3. 接入层不窥探/修改编排层内部状态机
4. 编排层返回什么语义，接入层只做协议化映射

边界示意：

```text
(gRPC Request DTO)
    ↓ decode + auth + dto mapping
Channel Handler
    ↓ exactly one orchestrator call
TurnExecutor::run_turn(input)
    ↓ result
Channel Handler
    ↓ output mapping + status mapping
(gRPC Response DTO)
```

### 1.4 依赖规则落实

遵循 architecture 文档依赖约束：

1. `agent-channel` 依赖 `agent-orchestrator`（或当前 `agent-app` 的编排入口）
2. `agent-channel` 可依赖 `agent-proto`（协议 DTO）
3. `agent-channel` 不依赖 `agent-storage` / `Postgres*` 等基础设施实现
4. 具体实现创建发生在 `agent-server` composition root

### 1.5 接入层检查清单（落地）

每个 handler 开发时必须检查：

1. 是否仅做协议转换、认证、映射
2. 是否仅调用编排层用例一次（或有限次，且无业务分支决策）
3. 是否没有出现 SQL/SDK 调用
4. 是否有统一错误映射
5. 是否有 request_id / trace_id 透传

---

## 2. gRPC Handler 设计

### 2.1 统一 Handler 流程模板

所有 unary handler 统一采用如下流程：

1. `decode request`
2. `auth`（若 endpoint 需要）
3. `validate`
4. `map DTO -> orchestrator input`
5. `call orchestrator`
6. `map orchestrator output -> DTO`
7. `map error -> tonic::Status`

统一伪代码模板：

```rust
async fn handler(req: Request<ProtoReq>) -> Result<Response<ProtoResp>, Status> {
    let meta = extract_transport_meta(&req)?;
    let identity = maybe_authenticate(&req, EndpointPolicy::Required)?;

    let dto = req.into_inner();
    validate_proto(&dto)?;

    let input = mapper::to_usecase_input(dto, identity, meta)?;
    let output = orchestrator.execute(input).await.map_err(error::to_status)?;

    let resp = mapper::to_proto_response(output);
    Ok(Response::new(resp))
}
```

### 2.2 AuthService handler 设计

#### 2.2.1 Register

职责：账号注册（公开端点，不要求 access token）。

流程：

1. decode `RegisterRequest`
2. 不做 JWT 校验（`EndpointPolicy::Public`）
3. 校验 email/password/invite_code 基本格式
4. map 到 `RegisterInput`
5. 调用 `AuthOrchestrator::register()`
6. map `RegisterOutput` -> `RegisterResponse`

#### 2.2.2 Login

职责：账号登录（公开端点，不要求 access token）。

流程：

1. decode `LoginRequest`
2. public endpoint
3. 校验 email/password 非空
4. map `LoginInput`
5. 调用 `AuthOrchestrator::login()`
6. map `LoginResponse`（token pair）

#### 2.2.3 RefreshToken

职责：使用 refresh token 换取新 access token（公开端点，凭 body token 校验）。

流程：

1. decode `RefreshTokenRequest`
2. public endpoint（不依赖 bearer access token）
3. 校验 refresh_token 非空
4. map `RefreshTokenInput`
5. 调用 `AuthOrchestrator::refresh_token()`
6. map 输出 token pair

### 2.3 ChatService handler 设计

#### 2.3.1 SendMessage（核心）

职责：处理一次用户消息 turn，返回 assistant 回复（MVP unary）。

流程：

1. decode `SendMessageRequest`
2. JWT auth required
3. 解析 request text blocks
4. map 为 `TurnInput`
5. 调用 `TurnExecutor::run_turn(input)`
6. map `TurnOutput` -> `SendMessageResponse`

#### 2.3.2 Subscribe

职责：订阅会话事件流（server stream）。

流程：

1. decode `SubscribeRequest`
2. JWT auth required
3. 校验 session 访问权限
4. 订阅编排层/事件层流（抽象接口）
5. `DomainEvent` -> `ChatEvent` 按需映射输出

> MVP 如暂未实现，可返回 `UNIMPLEMENTED`，但设计上保留完整映射路径。

### 2.4 SessionService handler 设计

#### 2.4.1 CreateSession

流程：

1. decode `CreateSessionRequest`
2. JWT auth required
3. map `CreateSessionInput`
4. 调用 `SessionOrchestrator::create_session()`
5. map `Session` -> proto `Session`

#### 2.4.2 ListSessions

流程：

1. decode `ListSessionsRequest`
2. JWT auth required
3. map pagination/filter
4. 调用 `SessionOrchestrator::list_sessions()`
5. map page result -> `ListSessionsResponse`

#### 2.4.3 ListSessionMessages

流程：

1. decode `ListSessionMessagesRequest`
2. JWT auth required
3. 鉴权：用户是否可读该 session
4. map 为 `ListSessionMessagesInput`
5. 调用 `SessionOrchestrator::list_session_messages()`
6. map message list + pagination

### 2.5 SendMessage 完整 Rust 伪代码

> 以下伪代码展示接入层完整职责闭环：decode → auth → mapping → run_turn → response。

```rust
use std::sync::Arc;
use tonic::{Request, Response, Status};
use agent_proto::{
    SendMessageRequest, SendMessageResponse,
    ContentBlock, TextBlock, content_block,
};
use agent_orchestrator::{TurnExecutor, TurnInput, UserMessageInput, TurnMetadata};

pub struct ChatServiceHandler {
    turn_executor: Arc<TurnExecutor>,
    auth_ctx_extractor: Arc<AuthContextExtractor>,
}

impl ChatServiceHandler {
    pub fn new(
        turn_executor: Arc<TurnExecutor>,
        auth_ctx_extractor: Arc<AuthContextExtractor>,
    ) -> Self {
        Self { turn_executor, auth_ctx_extractor }
    }
}

#[tonic::async_trait]
impl agent_proto::chat_service_server::ChatService for ChatServiceHandler {
    async fn send_message(
        &self,
        req: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        // 1) decode transport metadata
        let transport = grpc_transport::extract(&req)
            .map_err(error_mapper::transport_to_status)?;

        // 2) auth (required)
        let auth_ctx = self
            .auth_ctx_extractor
            .required(&req)
            .map_err(error_mapper::auth_to_status)?;

        // 3) parse proto DTO
        let dto = req.into_inner();

        // 4) validate request-level fields
        if dto.request_id.trim().is_empty() {
            return Err(Status::invalid_argument("request_id is required"));
        }
        if dto.agent_id.trim().is_empty() {
            return Err(Status::invalid_argument("agent_id is required"));
        }

        // 5) extract user text from repeated ContentBlock
        let user_text = dto
            .content
            .iter()
            .filter_map(|block| match &block.kind {
                Some(content_block::Kind::Text(t)) => Some(t.text.trim().to_string()),
                _ => None,
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if user_text.is_empty() {
            return Err(Status::invalid_argument(
                "content must include at least one non-empty text block"
            ));
        }

        // 6) map DTO -> TurnInput
        let turn_input = TurnInput {
            session_key: mapper::to_session_key(
                auth_ctx.tenant_id.clone(),
                auth_ctx.user_id.clone(),
                dto.session_id.clone(),
            ).map_err(error_mapper::validation_to_status)?,
            user_message: UserMessageInput {
                text: user_text,
                attachments: vec![], // MVP: no attachment mapping yet
                client_message_id: Some(dto.request_id.clone()),
            },
            metadata: TurnMetadata {
                tenant_id: Some(auth_ctx.tenant_id.clone()),
                user_id: Some(auth_ctx.user_id.clone()),
                agent_id: Some(dto.agent_id.clone()),
                trace_id: transport.trace_id.clone(),
                channel: Some("grpc".to_string()),
                request_id: Some(dto.request_id.clone()),
                route_key: Some("ChatService.SendMessage".to_string()),
            },
        };

        // 7) call orchestrator (single business entry)
        let out = self
            .turn_executor
            .run_turn(turn_input)
            .await
            .map_err(error_mapper::orchestrator_to_status)?;

        // 8) map TurnOutput -> proto response
        let response = SendMessageResponse {
            request_id: dto.request_id,
            session_id: out.session_id.to_string(),
            user_message_id: out
                .client_message_id
                .unwrap_or_else(|| "".to_string()),
            assistant_content: vec![ContentBlock {
                kind: Some(content_block::Kind::Text(TextBlock {
                    text: out.assistant_text,
                })),
            }],
        };

        Ok(Response::new(response))
    }

    type SubscribeStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<agent_proto::ChatEvent, Status>> + Send>>;

    async fn subscribe(
        &self,
        _req: Request<agent_proto::SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        Err(Status::unimplemented("Subscribe is not implemented in MVP"))
    }

    async fn submit_tool_result(
        &self,
        _req: Request<agent_proto::SubmitToolResultRequest>,
    ) -> Result<Response<agent_proto::SubmitToolResultResponse>, Status> {
        Err(Status::unimplemented("SubmitToolResult is not implemented in MVP"))
    }
}
```

### 2.6 Handler 文件组织建议（agent-channel）

建议目录：

```text
agent-channel/src/grpc/
  mod.rs
  auth_handler.rs
  chat_handler.rs
  session_handler.rs
  auth_interceptor.rs
  mapper/
    mod.rs
    auth_mapper.rs
    chat_mapper.rs
    session_mapper.rs
  error/
    mod.rs
    code.rs
    status_mapper.rs
```

---

## 3. DTO 与领域类型映射

### 3.1 映射原则

1. **单向明确**：入方向和出方向分别定义
2. **纯函数**：映射函数不访问 IO
3. **无副作用**：仅结构转换 + 校验
4. **类型收敛**：channel 层把字符串 ID 尽早收敛为领域 ID 类型
5. **错误可定位**：映射失败返回精确字段路径

### 3.2 入方向映射（Proto Request -> UseCase Input）

#### 3.2.1 Chat SendMessage

| Proto 字段 | 目标类型 | 规则 |
|---|---|---|
| `request_id` | `TurnMetadata.request_id` | 必填，空串报 `invalid_argument` |
| `session_id` | `TurnInput.session_key` | 空值表示新会话，非空需 parse 为 SessionId |
| `agent_id` | `TurnMetadata.agent_id` | 必填 |
| `content[]` | `UserMessageInput.text` | 仅抽取 TextBlock，按换行合并 |
| metadata | `TurnMetadata.route_key/trace_id` | 透传必要键 |
| JWT claims | `TurnMetadata.user_id/tenant_id` | 来自 interceptor，不从 body 读 |

#### 3.2.2 Session CreateSession

| Proto 字段 | 目标类型 | 规则 |
|---|---|---|
| `title` | `CreateSessionInput.title` | 可空；空则编排层生成默认标题 |
| JWT user_id | `CreateSessionInput.owner_user_id` | 必填 |
| JWT tenant_id | `CreateSessionInput.tenant_id` | 必填 |

#### 3.2.3 ListSessions / ListSessionMessages

| Proto 字段 | 目标类型 | 规则 |
|---|---|---|
| `pagination.page_size` | `PageQuery.limit` | clamp 到 `[1, 100]` |
| `pagination.page_token` | `PageQuery.cursor` | 可空 |
| `updated_at_order` | `SortOrder` | proto enum -> domain enum |
| `session_id` | `SessionId` | parse 失败返回 `invalid_argument` |

#### 3.2.4 Auth 请求

| Proto 字段 | 目标类型 | 规则 |
|---|---|---|
| `RegisterRequest` | `RegisterInput` | email/password/invite_code 必填 |
| `LoginRequest` | `LoginInput` | email/password 必填 |
| `RefreshTokenRequest.refresh_token` | `RefreshTokenInput` | 必填，格式基本校验 |

### 3.3 出方向映射（UseCase Output -> Proto Response）

#### 3.3.1 TurnOutput -> SendMessageResponse

| TurnOutput 字段 | Proto 字段 | 规则 |
|---|---|---|
| `session_id` | `session_id` | `to_string()` |
| `assistant_text` | `assistant_content[text]` | 组装单个 TextBlock（MVP） |
| `metadata.request_id` | `request_id` | 原样回显 |
| `client_message_id` | `user_message_id` | 若无则空串 |

#### 3.3.2 Session -> Session DTO

| Domain 字段 | Proto 字段 | 规则 |
|---|---|---|
| `Session.id` | `session.id` | string |
| `Session.title` | `session.title` | 为空时给默认展示名 |
| `Session.created_at` | `session.created_at` | 转 protobuf timestamp |
| `Session.updated_at` | `session.updated_at` | 转 protobuf timestamp |

#### 3.3.3 Message -> ChatMessage DTO

| Domain 字段 | Proto 字段 | 规则 |
|---|---|---|
| `Message.id` | `message.id` | string |
| `Message.role` | `message.role` | enum mapping |
| `Message.blocks` | `message.content[]` | 当前先映射 TextBlock |
| `Message.created_at` | `message.created_at` | timestamp |

### 3.4 映射层位置

映射层放在 `agent-channel` 内，不下沉到 orchestrator：

1. 协议 DTO 只存在于 channel
2. 编排层只依赖 domain/usecase 输入输出
3. 未来新增 HTTP/WebSocket 时可复用同一 usecase 输入，不耦合 proto

建议包结构：

```text
agent-channel::grpc::mapper
  - auth_mapper
  - chat_mapper
  - session_mapper
  - common (timestamp, pagination, id parsing)
```

### 3.5 版本演进策略

当 proto 演进时：

1. 优先新增字段，避免破坏旧字段
2. mapper 对新增字段设默认值
3. domain 输入不随 proto 每次变动而抖动
4. channel 层可并存 v1/v2 mapper，编排层接口保持稳定

---

## 4. 认证与鉴权

### 4.1 Auth 拦截器总体设计（tonic middleware）

使用 `tonic::service::Interceptor` 或 tower layer 实现统一认证中间件：

1. 从 gRPC metadata 读取 `authorization: Bearer <token>`
2. 解析 JWT（签名、过期、issuer、audience）
3. 提取 claim：`sub(user_id)`、`tenant_id`、`scope`
4. 构造 `AuthContext` 写入 `request.extensions()`
5. handler 内通过扩展读取，不重复解析 token

`AuthContext` 伪代码：

```rust
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: String,
    pub scopes: Vec<String>,
    pub token_id: Option<String>,
}
```

### 4.2 Endpoint 认证策略矩阵

| Service.Method | 是否要求认证 | 说明 |
|---|---|---|
| AuthService.Register | 否 | 公共注册端点 |
| AuthService.Login | 否 | 公共登录端点 |
| AuthService.RefreshToken | 否 | 基于 refresh token，非 access token |
| AuthService.Logout | 是 | 需要识别当前用户 |
| ChatService.SendMessage | 是 | 需用户身份和 tenant 隔离 |
| ChatService.Subscribe | 是 | 会话事件订阅受保护 |
| ChatService.SubmitToolResult | 是 | 工具回填必须认证 |
| SessionService.CreateSession | 是 | 会话归属用户 |
| SessionService.GetSession | 是 | 受会话读权限保护 |
| SessionService.ListSessions | 是 | 仅返回当前用户可见数据 |
| SessionService.ListSessionMessages | 是 | 受会话读权限保护 |

### 4.3 JWT 解析规则

最小 claim 要求：

1. `sub`：用户 ID（必需）
2. `tenant_id`：租户 ID（必需）
3. `exp`：过期时间（必需）
4. `iat`：签发时间（建议）
5. `scope`：权限列表（可选，默认最小权限）

校验失败场景：

1. 缺少 bearer token
2. token 非法格式
3. 签名错误
4. token 过期
5. claim 缺失（sub/tenant_id）

### 4.4 鉴权分层

鉴权分两层：

1. **接入层粗粒度鉴权**
   - endpoint 是否需要登录
   - scope 是否具备（如 `chat:write`、`session:read`）
2. **编排层细粒度授权**
   - 某 user 是否可访问某 session
   - 某 tenant 是否可操作某 agent

说明：接入层不做业务域判定，但可做通用 scope gate。

### 4.5 认证信息传递给编排层

传递策略：

1. interceptor 写入 `AuthContext` 到 request extensions
2. handler 把 `AuthContext` 映射到 usecase input metadata
3. 编排层统一从 input metadata 获取 `user_id/tenant_id`

示意：

```text
JWT -> Interceptor(AuthContext)
    -> Handler map
    -> TurnInput.metadata.user_id/tenant_id
    -> TurnExecutor
```

### 4.6 与当前实现差距（现状到目标）

当前 `chat_handler.rs` 中 `user_id` 缺省会退化为 `"unknown"`。目标设计要求：

1. SendMessage 必须要求认证
2. 缺失用户上下文直接返回 `UNAUTHENTICATED`
3. 禁止使用默认 `unknown` 继续执行业务

---

## 5. 错误码映射

### 5.1 设计目标

1. 对客户端返回稳定且可预测的 status code
2. 保留可追踪错误细节（error code / request_id）
3. 避免泄漏内部实现细节（SQL、SDK 错误原文）

### 5.2 统一错误响应格式

gRPC 顶层仍使用 `tonic::Status`，错误细节通过 message + details 承载。

推荐逻辑格式：

```json
{
  "error": {
    "code": "SESSION_NOT_FOUND",
    "message": "session not found",
    "request_id": "req_xxx",
    "retryable": false
  }
}
```

Rust 结构（伪代码）：

```rust
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    pub request_id: Option<String>,
    pub retryable: bool,
}
```

### 5.3 OrchestratorError -> tonic::Status 映射表

| OrchestratorError | gRPC Code | 统一 error.code | 说明 |
|---|---|---|---|
| `InvalidInput` | `INVALID_ARGUMENT` | `INVALID_INPUT` | 协议通过但业务输入非法 |
| `SessionNotFound` | `NOT_FOUND` | `SESSION_NOT_FOUND` | 会话不存在 |
| `AgentNotFound` | `NOT_FOUND` | `AGENT_NOT_FOUND` | agent 不存在 |
| `PermissionDenied` | `PERMISSION_DENIED` | `PERMISSION_DENIED` | 无资源访问权限 |
| `Conflict` | `ABORTED` | `CONFLICT` | 并发冲突/版本冲突 |
| `RateLimited` | `RESOURCE_EXHAUSTED` | `RATE_LIMITED` | 触发限流 |
| `ToolLoopExceeded` | `FAILED_PRECONDITION` | `TOOL_LOOP_EXCEEDED` | 达到工具循环上限 |
| `ContextTooLarge` | `FAILED_PRECONDITION` | `CONTEXT_TOO_LARGE` | 上下文超限制 |
| `LlmUnavailable` | `UNAVAILABLE` | `LLM_UNAVAILABLE` | 上游 LLM 不可用 |
| `UpstreamTimeout` | `DEADLINE_EXCEEDED` | `UPSTREAM_TIMEOUT` | 上游调用超时 |
| `PersistenceError` | `INTERNAL` | `PERSISTENCE_ERROR` | 存储失败 |
| `Internal` | `INTERNAL` | `INTERNAL_ERROR` | 未分类内部错误 |

### 5.4 AuthError -> tonic::Status 映射表

| AuthError | gRPC Code | 统一 error.code | 说明 |
|---|---|---|---|
| `MissingToken` | `UNAUTHENTICATED` | `AUTH_MISSING_TOKEN` | 未提供 bearer token |
| `InvalidTokenFormat` | `UNAUTHENTICATED` | `AUTH_INVALID_TOKEN` | token 格式错误 |
| `TokenExpired` | `UNAUTHENTICATED` | `AUTH_TOKEN_EXPIRED` | token 过期 |
| `InvalidSignature` | `UNAUTHENTICATED` | `AUTH_INVALID_SIGNATURE` | token 签名错误 |
| `MissingClaim("sub")` | `UNAUTHENTICATED` | `AUTH_MISSING_SUB` | 缺少用户 claim |
| `MissingClaim("tenant_id")` | `UNAUTHENTICATED` | `AUTH_MISSING_TENANT` | 缺少租户 claim |
| `InsufficientScope` | `PERMISSION_DENIED` | `AUTH_INSUFFICIENT_SCOPE` | scope 不满足 |
| `RevokedToken` | `UNAUTHENTICATED` | `AUTH_TOKEN_REVOKED` | token 已吊销 |
| `TooManyAttempts` | `RESOURCE_EXHAUSTED` | `AUTH_RATE_LIMITED` | 认证尝试过多 |
| `Internal` | `INTERNAL` | `AUTH_INTERNAL_ERROR` | 认证组件内部异常 |

### 5.5 校验错误映射（ValidationError）

| ValidationError | gRPC Code | 统一 error.code |
|---|---|---|
| required field missing | `INVALID_ARGUMENT` | `VALIDATION_REQUIRED` |
| enum parse failed | `INVALID_ARGUMENT` | `VALIDATION_ENUM` |
| id parse failed | `INVALID_ARGUMENT` | `VALIDATION_ID_FORMAT` |
| pagination out of range | `INVALID_ARGUMENT` | `VALIDATION_PAGINATION` |

### 5.6 错误映射伪代码

```rust
pub fn orchestrator_to_status(err: OrchestratorError, req_id: Option<&str>) -> Status {
    let (code, biz_code, msg, retryable) = match err {
        OrchestratorError::InvalidInput(m) => (Code::InvalidArgument, "INVALID_INPUT", m, false),
        OrchestratorError::SessionNotFound(id) => (Code::NotFound, "SESSION_NOT_FOUND", format!("session not found: {id}"), false),
        OrchestratorError::RateLimited => (Code::ResourceExhausted, "RATE_LIMITED", "rate limited".into(), true),
        OrchestratorError::LlmUnavailable => (Code::Unavailable, "LLM_UNAVAILABLE", "llm unavailable".into(), true),
        OrchestratorError::UpstreamTimeout => (Code::DeadlineExceeded, "UPSTREAM_TIMEOUT", "upstream timeout".into(), true),
        _ => (Code::Internal, "INTERNAL_ERROR", "internal error".into(), false),
    };

    let detail = ErrorBody {
        code: biz_code.to_string(),
        message: msg,
        request_id: req_id.map(|s| s.to_string()),
        retryable,
    };

    // 可选择序列化 detail 到 status details
    Status::new(code, serde_json::to_string(&detail).unwrap_or_else(|_| "internal".into()))
}
```

---

## 6. Composition Root（server 入口）

### 6.1 角色定义

`agent-server` 是进程入口和 DI 容器，职责：

1. 读取配置（env/file）
2. 初始化基础设施适配器（DB、LLM、cache...）
3. 组装能力层默认实现
4. 组装编排层 use case / executor
5. 构造 channel handlers
6. 启动 gRPC server

### 6.2 组装顺序（必须固定）

启动流程：

1. `config`
2. `db pool`
3. `adapters`（storage、jwt、llm provider、tool adapters）
4. `capabilities`（context builder、tool runtime、memory provider）
5. `orchestrator`（turn executor、session orchestrator、auth orchestrator）
6. `channel handlers`（Auth/Chat/Session + interceptors）
7. `serve`

### 6.3 DI 拆分建议

将 `main.rs` 中的组装逻辑下沉到 `ServerBuilder`：

1. `AppConfig::from_env()`
2. `InfrastructureModule::build(&config)`
3. `CapabilityModule::build(&infra)`
4. `OrchestratorModule::build(&caps)`
5. `ChannelModule::build(&orch)`

每个 module 只暴露 trait 对象，不泄漏具体类型到上层。

### 6.4 Composition Root Rust 伪代码

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) config
    let cfg = AppConfig::from_env()?;

    // 2) db pool
    let db_pool = PgPoolOptions::new()
        .max_connections(cfg.db.max_connections)
        .connect(&cfg.db.url)
        .await?;

    // 3) adapters (Layer 4)
    let user_repo = Arc::new(PostgresUserStore::new(db_pool.clone()));
    let session_repo = Arc::new(PostgresSessionStore::new(db_pool.clone()));
    let event_repo = Arc::new(PostgresEventStore::new(db_pool.clone()));

    let jwt_verifier = Arc::new(JwtVerifier::new(
        cfg.auth.jwt_public_key.clone(),
        cfg.auth.issuer.clone(),
        cfg.auth.audience.clone(),
    ));

    let llm_provider = Arc::new(OpenAiProvider::with_config(
        cfg.llm.api_key.clone(),
        cfg.llm.base_url.clone(),
        cfg.llm.model.clone(),
    ));

    // 4) capabilities (Layer 3)
    let context_builder = Arc::new(DefaultContextBuilder::new(/* prompt sections */));
    let tool_runtime = Arc::new(DefaultToolRuntime::new(/* tool registry */));
    let memory_provider = Arc::new(DefaultMemoryProvider::new(/* optional */));

    // 5) orchestrator (Layer 2)
    let session_lifecycle = Arc::new(DefaultSessionLifecycle::new(
        session_repo.clone(),
        event_repo.clone(),
    ));

    let turn_executor = Arc::new(TurnExecutor::new(
        session_lifecycle,
        event_repo.clone(),
        context_builder,
        llm_provider,
        tool_runtime,
        TurnExecutorConfig::default(),
    ));

    let auth_orchestrator = Arc::new(AuthOrchestrator::new(
        user_repo.clone(),
        cfg.auth.jwt_signing_key.clone(),
    ));

    let session_orchestrator = Arc::new(SessionOrchestrator::new(
        session_repo,
        event_repo,
    ));

    // 6) channel handlers (Layer 1)
    let auth_interceptor = GrpcAuthInterceptor::new(jwt_verifier);

    let auth_handler = AuthServiceHandler::new(auth_orchestrator);
    let chat_handler = ChatServiceHandler::new(turn_executor);
    let session_handler = SessionServiceHandler::new(session_orchestrator);

    // 7) serve
    tonic::transport::Server::builder()
        .layer(trace_layer())
        .add_service(public_auth_service(auth_handler))
        .add_service(private_chat_service(chat_handler, auth_interceptor.clone()))
        .add_service(private_session_service(session_handler, auth_interceptor))
        .serve(cfg.grpc.listen_addr)
        .await?;

    Ok(())
}
```

### 6.5 Public / Private Service 注册策略

因为 AuthService 存在公开端点（Register/Login/RefreshToken），建议分流：

1. Public 路由：不套全局 auth interceptor
2. Private 路由：统一套 auth interceptor
3. 若技术上统一挂载一套 interceptor，则 interceptor 内按 method 白名单 bypass

白名单示例：

- `/ai.agent.platform.v1.AuthService/Register`
- `/ai.agent.platform.v1.AuthService/Login`
- `/ai.agent.platform.v1.AuthService/RefreshToken`

### 6.6 当前入口代码对齐建议

当前 `agent-server/src/main.rs` 已具备：

1. 配置读取
2. DB pool 初始化与 migrate
3. store/provider 创建
4. `ServerBuilder::serve()` 启动

本设计强调后续演进方向：

1. 编排层接口显式化（TurnExecutor 等）
2. channel 与 infra 解耦通过 trait 注入
3. auth/public-private 路由策略统一

---

## 7. 与参考框架对比与本设计选择

### 7.1 对比维度

对比维度：

1. 接入层是否“薄”
2. 协议模型与业务模型是否分离
3. 认证信息是否标准化透传
4. 错误码是否统一映射
5. 组合根是否清晰

### 7.2 三框架简要对比

| 参考框架 | 接入层特点 | 优点 | 风险/不足 |
|---|---|---|---|
| **Kratos (Go)** | transport 层 + middleware 明确，biz/app 分层清晰 | middleware 体系成熟，错误映射规范化好 | 需要严格 discipline，易在 service 混入业务 |
| **ConnectRPC / Buf（Go/TS）** | proto-first，handler 轻，跨语言一致性好 | 协议演进与生成链路成熟 | 项目若无统一 error policy，客户端体验不一致 |
| **tonic + tower（Rust）** | interceptor + layer 灵活，类型安全高 | Rust trait + DI 可强约束边界 | 需要自行约定 mapper/error 结构，否则易分散 |

### 7.3 本设计选择

基于 ADR-003（gRPC）和 ADR-006（接口定义），本项目选择：

1. 继续使用 tonic/tower 作为接入技术栈
2. 明确 **Channel 薄适配**：只做协议转换 + 认证鉴权 + 错误映射
3. 业务流程统一进入编排层（TurnExecutor / SessionOrchestrator / AuthOrchestrator）
4. 在 `agent-channel` 设 mapper 与 error 子模块，保证入口一致性
5. 在 `agent-server` 保持 composition root 责任，不让组装逻辑扩散

### 7.4 为什么不在接入层做更多“捷径逻辑”

不采用“handler 直接调 repo/provider”的原因：

1. 破坏 Layer 1/2/3/4 依赖方向
2. 多协议入口时（HTTP/WebSocket）逻辑会复制
3. 测试复杂度提高（难以单测编排）
4. 日后拆服务时迁移成本更高

### 7.5 MVP 与后续演进

MVP：

1. SendMessage unary 跑通
2. Subscribe 可先 `UNIMPLEMENTED`
3. 错误码映射与 auth 拦截器先统一

后续：

1. 实现 Subscribe event stream
2. 完善 SubmitToolResult 双向工具流
3. 增加 grpc status details（protobuf typed error）
4. 多协议入口复用同一编排层接口

---

## 附录 A：接口方法到编排入口映射

| gRPC 方法 | 编排层入口 | 备注 |
|---|---|---|
| AuthService.Register | `AuthOrchestrator::register` | public |
| AuthService.Login | `AuthOrchestrator::login` | public |
| AuthService.RefreshToken | `AuthOrchestrator::refresh_token` | public |
| ChatService.SendMessage | `TurnExecutor::run_turn` | required auth |
| ChatService.Subscribe | `ChatStreamOrchestrator::subscribe` | required auth |
| SessionService.CreateSession | `SessionOrchestrator::create_session` | required auth |
| SessionService.ListSessions | `SessionOrchestrator::list_sessions` | required auth |
| SessionService.ListSessionMessages | `SessionOrchestrator::list_session_messages` | required auth |

---

## 附录 B：接入层实现约束（可用于 code review）

1. handler 文件中禁止出现 SQL 语句
2. handler 文件中禁止 new 外部 SDK client
3. handler 入口必须提取 request_id 并写入日志上下文
4. auth required 的 endpoint 不允许 fallback 为匿名用户
5. mapper 函数必须有单元测试（成功 + 失败）
6. error mapper 必须覆盖 OrchestratorError / AuthError 全分支
7. composition root 必须只在 `agent-server` crate
8. channel crate 不得依赖 storage crate

---

## 附录 C：最小测试矩阵

### C.1 Handler 层

1. SendMessage：无 token -> `UNAUTHENTICATED`
2. SendMessage：空 content -> `INVALID_ARGUMENT`
3. SendMessage：orchestrator `SessionNotFound` -> `NOT_FOUND`
4. ListSessions：分页越界 -> `INVALID_ARGUMENT`
5. Register：重复邮箱 -> `ALREADY_EXISTS`（若领域有该错误）

### C.2 Interceptor 层

1. bearer 缺失
2. bearer 前缀错误
3. token 过期
4. token claim 缺失 tenant_id
5. public endpoint bypass

### C.3 Error mapper

1. OrchestratorError 每个分支 code 断言
2. AuthError 每个分支 code 断言
3. message 不包含敏感栈信息

---

## 附录 D：实施计划（仅文档层）

1. 先固定目录与模块命名
2. 再补齐 mapper/error 单测模板
3. 最后补 Subscribe / SubmitToolResult 设计实现

> 本文档只定义设计，不包含代码改造。

