# Brief: D03-Rev — 修订 Channel Port Trait（去关联类型）

## 任务目标

根据 S02 spike 结论，修订 `docs/design/channel-system/port.md`：
把 `ChannelAdapter` trait 的关联类型 `EventStream` 改为 boxed stream 返回类型，
确保 trait 对象安全，支持 `Arc<dyn ChannelAdapter + Send + Sync>` 持有方式。

## 输出位置

直接修改：`server/docs/design/channel-system/port.md`

## 成功标准

1. `ChannelAdapter` trait 中不再有 `type EventStream` 关联类型
2. `subscribe` 方法返回类型改为：
   `Pin<Box<dyn Stream<Item = Result<AgentEventEnvelope, ChannelError>> + Send + 'static>>`
   或等价的 type alias（如 `DynEventStream`）
3. 在文档中新增一行说明：为什么去掉关联类型（对象安全 + S02 验证结论）
4. 其他方法签名、类型定义、fanout/重连语义不变

## 必要上下文

**S02 结论**：
- 关联类型 `type EventStream` 使 trait 失去对象安全性
- 方案 A（boxed stream）能通过 `cargo check`，支持 `Arc<dyn Trait>`
- 应用层需要动态切换 gRPC/WebSocket 实现，必须用 dyn

**建议的 type alias 写法**（放在 trait 定义前）：
```
pub type DynEventStream =
    Pin<Box<dyn Stream<Item = Result<AgentEventEnvelope, ChannelError>> + Send + 'static>>;
```

## 约束

- 只修改 `subscribe` 方法签名和 trait 定义，不动其他内容
- 不重写整个文件，只做定点修改
