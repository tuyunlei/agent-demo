# Brief: S01 — Spike: async trait dyn vs 泛型参数

## 任务目标

验证在 Rust 中为 Port trait（包含 async 方法）实现依赖注入时，两种方案的代码形态、
编译结果与权衡，给出一个明确的架构选型建议。

**要验证的核心问题**：
在 6-8 个 Port 的六边形架构里，用 `dyn Trait`（动态分发）还是泛型参数（静态分发）
来注入 Port？哪个方案在 Rust 的类型系统下更实用？

## 输出位置

```
server/spikes/s01-async-trait/
├── Cargo.toml
├── src/
│   └── lib.rs      (或 main.rs)
└── RESULT.md       (结论文档，必须有)
```

## 成功标准

1. `cargo check` 通过（不要求 `cargo run`，不要求连接任何外部服务）
2. `RESULT.md` 包含：
   - 两种方案各自的代码示例（可以直接引用 src/ 里的代码）
   - 编译层面的限制（哪些写法无法通过，为什么）
   - 明确推荐：选哪种方案，理由是什么
   - 如果需要权衡：写清楚什么场景用哪种

## 必要上下文

**我们的架构**：六边形架构（Ports & Adapters），核心域定义 Port（trait），
适配器层实现。Application Service 接收多个 Port 作为依赖注入。

**典型的 Port 列表**（需要注入到 Application Service 的）：
- `LlmProvider`：调用 LLM，返回流式 token（async，返回 Stream）
- `MessageStore`：存取消息历史（async CRUD）
- `SessionStore`：存取会话状态（async CRUD）
- `MemoryStore`：存取长期记忆（async，可能涉及向量检索）
- `ToolExecutor`：执行工具调用（async，等待工具结果）
- `PersonaStore`：读取人格配置（async 读）
- `EventPublisher`：发布 ChatEvent 给 Subscribe 流（async，可能是 channel sender）

**关键难点**：
- async trait 在 Rust 中不能直接 `dyn Trait`（因为 async fn 返回 impl Future，
  不是固定大小类型）
- 常见解法：`async_trait` crate（会 box future）或手动 Pin + Box
- 泛型方案：`struct AgentRuntime<L: LlmProvider, M: MessageStore, ...>`，
  但 6-8 个泛型参数时类型签名会很长
- Send + Sync 约束：tokio 多线程运行时下 trait object 需要 `dyn Trait + Send + Sync`

**最终用法场景**（需要能支持）：
```rust
// Application Service 接收多个 Port
struct SendMessageService {
    llm: ???,          // 如何持有 LlmProvider？
    messages: ???,     // 如何持有 MessageStore？
    // ... 其他 Port
}
```

## 约束

- 不要实现任何真实业务逻辑，只做结构验证
- 不要引入 sqlx、tonic、reqwest 等真实适配器依赖（用空实现模拟）
- 可以引入 `async-trait` crate、`tokio` crate（仅用于 async runtime）
- 每种方案都要有可以 cargo check 的完整代码
- RESULT.md 用中文写，代码注释可以中英混用
