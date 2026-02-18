# S02 Spike 结果：ChannelAdapter dyn 兼容性验证

## 完成情况

已在 `server/spikes/s02-channel-adapter-dyn/` 创建：

- `Cargo.toml`
- `src/lib.rs`
- `RESULT.md`（本文）

并在该目录执行：

```bash
source $HOME/.cargo/env && cargo check
```

结果：`cargo check` 通过。

---

## 方案 A：去关联类型，返回 `Box<dyn Stream>`

对应代码：`src/lib.rs` 中 `ChannelAdapterDyn`。

核心形态：

```rust
pub type DynEventStream = Box<dyn Stream<Item = EventItem> + Send + Unpin + 'static>;

#[async_trait]
pub trait ChannelAdapterDyn: Send + Sync {
    async fn subscribe(&self, req: SubscriptionRequest) -> Result<DynEventStream, ChannelError>;
    async fn publish(&self, envelope: AgentEventEnvelope, policy: FanoutPolicy) -> Result<DeliveryReport, ChannelError>;
}
```

应用层可直接持有：

```rust
Arc<dyn ChannelAdapterDyn + Send + Sync>
```

这与当前需求（运行时多实现可替换）完全对齐。

---

## 方案 B：保留关联类型，使用泛型持有

对应代码：`src/lib.rs` 中 `ChannelAdapterGeneric` + `AppServiceGeneric<A>`。

核心形态：

```rust
#[async_trait]
pub trait ChannelAdapterGeneric: Send + Sync {
    type EventStream: Stream<Item = EventItem> + Send + Unpin + 'static;
    async fn subscribe(&self, req: SubscriptionRequest) -> Result<Self::EventStream, ChannelError>;
}

pub struct AppServiceGeneric<A>
where
    A: ChannelAdapterGeneric + Send + Sync + 'static,
{
    adapter: Arc<A>,
}
```

结论：可以通过编译，但服务层类型被具体 `A` 绑定，不适合“同一位置动态注入不同 adapter 实现”。

---

## 对需求的匹配度比较

你的关键约束是：

1. `agent-app` 要持有 `Arc<dyn ChannelAdapter + Send + Sync>`
2. 运行时可能切换 gRPC / WebSocket 等实现
3. tokio 多线程安全（`Send + Sync`）

对比：

- **方案 A（Box<dyn Stream>）**：
  - ✅ 支持 `Arc<dyn Trait>`
  - ✅ 运行时替换实现自然
  - ✅ `Send + Sync + 'static` 约束可完整保留
  - ⚠️ 有一层动态分发/装箱开销（通常可接受）

- **方案 B（泛型 + 关联类型）**：
  - ✅ 零/少抽象开销，静态分发
  - ❌ 不适合单一应用服务对象在运行时装配不同具体实现
  - ❌ 与“dyn 持有”目标冲突

---

## 明确建议

### 推荐方案
**推荐方案 A：`subscribe` 返回 `Box<dyn Stream<...>>`（或 `Pin<Box<dyn Stream<...>>>` 等等价 boxed stream 形式）。**

### D03 是否需要改
**需要改。**

如果 D03 的核心目标仍是应用层通过 `Arc<dyn ChannelAdapter + Send + Sync>` 做多实现可替换，那么 trait 里不应再使用 `type EventStream` 关联类型作为对外返回值；应改为 boxed stream 返回类型，确保对象安全与运行时可替换性。