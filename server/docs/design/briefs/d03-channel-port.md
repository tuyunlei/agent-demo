# Brief: D03 — Channel Port Trait 精确定义

## 任务目标

为 `ChannelAdapter` Port 写出精确的 Rust trait 设计文档：方法签名、关联类型、
AgentEvent 枚举定义、错误类型。Channel Port 必须协议无关，gRPC/WebSocket 都能实现它。

## 输出位置

`server/docs/design/channel-system/port.md`

新建该文件，markdown 格式。

## 成功标准

1. `ChannelAdapter` trait 有精确的方法签名（含 async、参数类型、返回类型、错误类型）
2. `AgentEvent` 枚举完整定义（TextChunk、ToolRequest、RoundComplete、ChatError 等变体）
3. `InboundMessage` 结构体有精确字段定义
4. 说明 gRPC Channel 如何实现这个 trait（只需要说明映射关系，不写实际 Rust 代码）
5. 说明多个并发订阅者场景（同一用户多设备）如何处理

## 必要上下文

**Channel 的核心职责**（来自 channel-system/overview.md）：
- 接收用户的入站消息（InboundMessage），标准化后交给 Application Layer
- 将 Agent Runtime 产生的 AgentEvent 下发给对应的客户端连接
- 协议无关：gRPC、WebSocket、Push 都是实现，不是接口

**现有 gRPC 协议文档中的事件定义**（作为 AgentEvent 设计参考）：
```
TextChunk     - 流式文本片段（内容 + 是否最后一片）
ToolRequest   - 请求客户端执行工具（tool_call_id + 工具名 + 参数 + 超时）
RoundComplete - 一轮对话完成（包含 token 用量 Usage）
ChatError     - 错误事件（error_code + message + request_id）
```

**gRPC 协议中的三个接口**（作为 Channel 实现的参考）：
- `SendMessage`（unary）：客户端发消息，服务端接收 → InboundMessage
- `Subscribe`（server streaming）：服务端推送 AgentEvent 流给客户端
- `SubmitToolResult`（unary）：客户端提交工具执行结果

**S01 结论**：
- trait 使用 `#[async_trait]`
- 持有方式：`Arc<dyn ChannelAdapter + Send + Sync>`

**关键设计问题需要在文档中给出答案**：
- Channel 如何把 AgentEvent 推给特定的连接（session_id + 连接 handle）？
- 多设备订阅同一 session 时，如何 fanout？
- gRPC Subscribe 流断开重连时，Channel 如何处理？

**参考文件**：
- `server/docs/design/channel-system/overview.md`
- `server/docs/design/protocols/grpc-services.md`

## 约束

- 不写实际 Rust 代码（用 Rust 伪代码/类型签名描述即可）
- 只设计 Channel Port 本身，不展开 gRPC Adapter 的内部实现细节
- AgentEvent 只定义 MVP 需要的变体，不要展开未来可能的类型
- 不重复 channel-system/overview.md 已有的内容，只补充精确接口定义
