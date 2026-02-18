# Brief: S02 — Spike: ChannelAdapter 关联类型 + dyn 兼容性

## 任务目标

验证 `ChannelAdapter` trait 有关联类型 `EventStream` 时，如何持有 `dyn ChannelAdapter`。
找到能通过 `cargo check` 的方案，并给出明确的 trait 设计建议。

## 背景

D03 设计的 trait 如下（简化）：

```rust
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    type EventStream: Stream<Item = Result<AgentEventEnvelope, ChannelError>>
        + Send + Unpin + 'static;

    async fn subscribe(&self, req: SubscriptionRequest) -> Result<Self::EventStream, ChannelError>;
    async fn publish(&self, envelope: AgentEventEnvelope, policy: FanoutPolicy) -> Result<DeliveryReport, ChannelError>;
    // ...
}
```

问题：有关联类型的 trait 通常无法直接做 trait object（`dyn ChannelAdapter`），
或者需要指定关联类型（`dyn ChannelAdapter<EventStream = ???>`），
这样就失去了"任意实现可替换"的能力。

## 输出位置

```
server/spikes/s02-channel-adapter-dyn/
├── Cargo.toml
├── src/
│   └── lib.rs
└── RESULT.md
```

## 成功标准

1. `cargo check` 通过
2. 验证至少两个方案：
   - **方案 A**：关联类型改为 `Box<dyn Stream<...>>`（去掉关联类型，改返回类型）
   - **方案 B**：保留关联类型，用泛型参数持有（`Arc<impl ChannelAdapter>`）
3. `RESULT.md` 包含：
   - 两种方案的代码形态（可以直接引用 src/）
   - 哪种方案更符合我们的需求（多实现可替换 + tokio 多线程安全）
   - 明确推荐：要不要修改 D03 的 trait 设计

## 必要上下文

**我们的使用场景**：
- `agent-app` 里的 Application Service 持有 `Arc<dyn ChannelAdapter + Send + Sync>`
- 运行时可能有多种 Channel（gRPC、WebSocket），需要动态切换
- 在 tokio 多线程环境下，需要 `Send + Sync`

**关键 Rust 限制**：
- 有关联类型的 trait 做 `dyn Trait` 时，必须指定所有关联类型
- `async fn` 需要 `#[async_trait]` 处理
- `Stream` 是 `futures::Stream`，在 trait 中返回它有额外约束

**可以引入的依赖**：
- `async-trait`
- `tokio` (仅 runtime 类型)
- `futures` (Stream trait)
- 不需要 tonic、sqlx 等真实适配器

## 约束

- 不实现任何实际网络 I/O，用 fake adapter 即可
- 只验证类型系统，不要跑真实逻辑
- RESULT.md 用中文，给出 D03 是否需要修改 trait 设计的明确建议
