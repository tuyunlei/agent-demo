# S01 Spike 结果：async trait 注入方式（dyn vs 泛型）

## 1. Spike 结论（先说结论）

- `cargo check`：✅ 通过
- 推荐方案：**`dyn Trait`（基于 `async-trait`）作为 Application Service 的默认注入方式**
- 一句话理由：**在 6~8 个 Port 的六边形架构里，`dyn` 能显著降低类型复杂度与模块耦合，工程可维护性更好，性能损耗通常可接受。**

---

## 2. 验证范围与代码位置

- 工程目录：`server/spikes/s01-async-trait/`
- 关键文件：
  - `src/lib.rs`
  - `Cargo.toml`
- 本次只做结构可行性验证，不接真实外部系统（DB/LLM/消息总线均为 fake adapter）

---

## 3. 方案 A：`dyn Trait`（动态分发）

### 代码形态（已在 `src/lib.rs`）

```rust
pub struct SendMessageServiceDyn {
    llm: Arc<dyn LlmProvider>,
    messages: Arc<dyn MessageStore>,
    sessions: Arc<dyn SessionStore>,
}
```

对应的 Port trait 用 `#[async_trait]`：

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn stream_tokens(&self, prompt: &str) -> AppResult<Vec<Token>>;
}
```

### 编译层面说明

- 直接 `async fn` 的 trait 默认对 `dyn Trait` **不 object-safe**。
- `async-trait` 会把 async 方法改写为 `Pin<Box<dyn Future<...>>>`，从而支持 trait object。
- 在 tokio 多线程场景，`dyn Trait + Send + Sync` 是常见必备约束。

### 优点

- Service 类型签名稳定、短小，构造器可读性高。
- Port 数量增长（6~8 个）时复杂度可控。
- 运行时更容易做“按配置切换 adapter”（多实现热插拔）。

### 代价

- 动态分发 + future 装箱有少量运行时开销。
- 某些极致性能路径不如静态分发。

---

## 4. 方案 B：泛型参数（静态分发）

### 代码形态（已在 `src/lib.rs`）

```rust
pub struct SendMessageServiceGeneric<L, M, S>
where
    L: LlmProvider,
    M: MessageStore,
    S: SessionStore,
{
    llm: L,
    messages: M,
    sessions: S,
}
```

### 编译层面说明

- 泛型方式可完整编译通过，且无需把依赖放进 `Arc<dyn ...>`。
- 静态分发有利于优化（可能 inline、无 vtable 调用）。

### 优点

- 理论性能更优（无动态分发）。
- 类型约束在编译期更强，某些错误更早暴露。

### 代价

- Port 数量增加时，泛型参数和 `where` 约束迅速膨胀。
- Service/Builder/测试替身/组合类型都更“重”，跨模块传播类型噪音。
- 如果上层再泛型化，会出现“泛型传染”。

---

## 5. 哪些写法会卡编译（限制点）

### 限制 1：不借助 `async-trait` 时，想直接 `dyn` async trait 会失败

下面这种“直觉写法”在稳定 Rust 下不能直接作为 trait object 使用：

```rust
trait BadPort {
    async fn call(&self);
}

struct S {
    p: Box<dyn BadPort>, // object safety 问题
}
```

原因：`async fn` 在 trait 中对应不定大小 future，不能直接满足 object-safe 的要求。

### 限制 2：泛型方案在多 Port 下可编译，但工程可读性明显下降

单个 Service 尚可；当扩展到 `L, M, S, Mem, Tool, Persona, Event...` 时，
类型签名、构造函数、测试桩都会更长，维护成本上升（这是工程层面的“限制”）。

---

## 6. 推荐落地策略

### 默认策略（推荐）

- **Application Service 层使用 `Arc<dyn Port + Send + Sync>`**
- Port trait 统一使用 `#[async_trait]`
- 在组合根（composition root）集中完成具体 adapter 装配

### 例外策略（可选）

如果某个热点路径经 profiling 证明动态分发开销显著，再对局部改为泛型/静态分发。

---

## 7. 最终推荐

**选 `dyn Trait`（配合 `async-trait`）作为主方案。**

- 对当前六边形架构（多 Port 注入）更实用；
- 代码组织和可维护性收益明显；
- 性能成本通常可接受，且可在局部热点再做静态化优化。
