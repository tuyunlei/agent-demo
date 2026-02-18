# Brief: S03 — Spike: Tokio Actor/Mailbox（Session 单写者模型）

## 任务目标

验证在 Rust + Tokio 下，用 actor pattern 实现"每个 session 只有一个 writer"的
并发模型。每个 session 对应一个 actor task，外部通过 mailbox（channel）发消息。

## 背景

Agent Runtime 里每个 session 同时只能有一个推理过程在跑（防止并发写冲突），
但 HTTP 请求是并发的（多个请求可能命中同一个 session）。

设计目标：session actor 持有 session 的所有可变状态，外部只能通过消息与它通信。

## 输出位置

```
server/spikes/s03-tokio-actor/
├── Cargo.toml
├── src/
│   └── main.rs
└── RESULT.md
```

## 成功标准

1. `cargo check` 通过（最好能 `cargo run` 跑通基本流程）
2. 实现一个最小 session actor，包含：
   - 接收 `ProcessMessage` 命令，模拟推理（async sleep）
   - 返回处理结果给调用方（oneshot channel 回复）
   - 处理 actor 停止（graceful shutdown）
3. 实现 actor registry：根据 session_id 查找或创建对应的 actor handle
4. `RESULT.md` 包含：
   - 代码形态总结
   - 关键决策点（mailbox 容量、back pressure、actor 生命周期管理）
   - 是否推荐这个模式，或者有更简单的替代方案

## 必要上下文

**场景**：
```
用户发消息 → gRPC handler → 查 session actor registry → 发消息给对应 actor
                                                          ↓
                                                    actor 串行处理
                                                    （LLM 调用、存储）
                                                          ↓
                                                    通过 EventStream
                                                    推送结果给订阅者
```

**关键约束**：
- 同一 session 的请求必须串行处理（不能并发推理）
- 不同 session 可以并发（各自独立的 actor）
- actor 长时间无活动后应该能自动停止（idle timeout）

**可以引入的依赖**：
- `tokio`（full features）
- 可以参考 `tokio::sync::mpsc`（mailbox）+ `tokio::sync::oneshot`（reply）
- 不需要 actix、xtra 等 actor 框架，用原生 tokio 实现

## 约束

- 不需要接真实 LLM 或数据库，用 `tokio::time::sleep` 模拟异步工作
- 不需要完整的 session 状态，用一个简单的计数器或字符串模拟即可
- RESULT.md 中明确：这个 actor 模式在 D04（Agent Runtime 拆分）里如何落地
