# ADR-006: gRPC 接口定义

**日期**：2026-02-16
**状态**：已决定（初稿，后续会迭代）

## 四个 Service

### AuthService — 认证
- `Register` — 邀请码验证 + 设备绑定
- `Authenticate` — 设备密钥认证

### ChatService — 聊天（核心）
- `SendMessage` → `stream ChatEvent` — 发消息，服务端流式返回
- `GetHistory` — 获取历史消息（分页）

### AgentService — Agent 管理
- `GetAgent` — 获取 agent 信息
- `UpdateAgent` — 更新 agent（名字、人格等）

### SessionService — 会话管理
- `ListSessions` — 获取会话列表
- `CreateSession` — 创建新会话

## ChatEvent 统一流式事件

```protobuf
message ChatEvent {
  oneof event {
    TextDelta text_delta = 1;              // 文字回复（流式分块）
    ToolCallStart tool_call_start = 2;     // 工具调用开始
    ToolCallResult tool_call_result = 3;   // 工具调用结果
    ClientToolRequest client_tool_request = 4; // 客户端工具下发
    Done done = 5;                         // 完成
    Error error = 6;                       // 错误
  }
}
```

## 设计要点

- SendMessage 用 server streaming，MVP 可以只发一条完整回复 + Done（非流式），接口不用改
- ChatEvent 统一了流式回复、工具状态、客户端工具下发
- 未来新增事件类型只需在 oneof 里加字段，向后兼容
