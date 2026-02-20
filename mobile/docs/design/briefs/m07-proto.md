# M07 Brief: Proto 定义

## 任务目标

编写 .proto 文件，定义客户端与服务端之间的 gRPC 接口协议。需与服务端已有设计对齐。

## 输出位置

`proto/` 目录下，按服务拆分文件：
- `proto/auth.proto`
- `proto/chat.proto`
- `proto/session.proto`
- `proto/common.proto`（共享类型）

## 成功标准

1. 覆盖 MVP 所有 gRPC 接口（Auth + Chat + Session）
2. 与服务端 `grpc-services.md` 定义完全对齐
3. 与客户端 `services.md` 中的接口签名对齐
4. 消息体字段完整（包括 pagination、error、metadata）
5. 包含必要的注释说明
6. proto3 语法，package 命名规范

## 必要上下文

### 服务端 gRPC 接口设计

读取 `server/docs/design/protocols/grpc-services.md` 获取完整的服务定义、消息体、枚举。

### 客户端服务层接口

读取 `mobile/docs/design/services.md` section 3，了解客户端侧的方法签名和 DTO 结构。

### 核心服务

1. **AuthService**
   - Register(email, password, display_name?) → TokenPair
   - Login(email, password, device_name, platform) → TokenPair
   - RefreshToken(refresh_token) → TokenPair
   - Logout(logout_all_devices) → void

2. **ChatService**
   - SendMessage(request_id, session_id?, agent_id, content_blocks, metadata) → ack(message_id, session_id)
   - Subscribe(session_id, last_event_id?) → stream ChatEvent
   - SubmitToolResult(session_id, tool_call_id, result_json, is_error, error_message?) → ack

3. **SessionService**
   - ListSessions(page_size, page_token?) → SessionPage
   - GetSession(session_id) → Session
   - ListSessionMessages(session_id, page_size, page_token?) → MessagePage

### ChatEvent 类型（stream 推送）
- TextChunk(message_id, session_id, text, sequence_number)
- ToolRequest(message_id, session_id, tool_call_id, tool_name, arguments_json)
- RoundComplete(message_id, session_id, usage?)
- ChatError(session_id, error_code, message, retryable)

## 参考文档

- 服务端 gRPC 设计：`server/docs/design/protocols/grpc-services.md`
- 客户端服务层：`mobile/docs/design/services.md`

## 约束

- proto3 语法
- 不要自创服务端没有的接口
- 字段命名用 snake_case（proto 惯例）
- 枚举值命名用 SCREAMING_SNAKE_CASE
- 服务端设计文档中如有模糊的地方标注 [待确认]
