# S03 Tokio Actor Spike — RESULT

## 结论

**可行。**

用原生 Tokio（`mpsc + oneshot + tokio::spawn`）可以稳定实现“每个 session 单写者、跨 session 并发”的模型：
- 同一 `session_id` 的请求进入同一个 mailbox，actor 串行处理，天然避免并发写冲突。
- 不同 `session_id` 对应不同 actor task，可并发执行。
- 通过 idle timeout + graceful shutdown 可以做生命周期收敛。

## 完成内容

已在 `server/spikes/s03-tokio-actor/` 创建：
- `Cargo.toml`
- `src/main.rs`
- `RESULT.md`

并验证：
- `~/.cargo/bin/cargo check` ✅
- `~/.cargo/bin/cargo run` ✅

运行结果显示：
1. `session-A` 的 3 个并发调用被同一个 actor 按 turn=1/2/3 串行处理。
2. `session-B` 独立 actor 并发存在。
3. `session-B` 可 graceful shutdown。
4. `session-A` 在 idle timeout 后自动退出。
5. registry 在 actor 退出后回收条目，最终 active actors=0。

## 代码形态总结

- `SessionCommand`：
  - `ProcessMessage { input, reply: oneshot::Sender<String> }`
  - `Shutdown { reply: oneshot::Sender<()> }`
- `SessionHandle`：对外 API（`process_message` / `shutdown`），内部只暴露 `mpsc::Sender`。
- `SessionRegistry`：
  - `get_or_create(session_id)`：返回存活 handle，不存在则创建 actor。
  - `remove_if_match(session_id, actor_id)`：actor 退出后按代际删除，防止误删新 actor。
- `run_session_actor`：
  - mailbox 循环 + `timeout(IDLE_TIMEOUT, rx.recv())`
  - 串行模拟异步推理（`sleep`）
  - 处理 shutdown / channel close / idle stop。

## 关键设计决策点

1. **mailbox 容量（bounded channel）**
   - 采用 `mpsc::channel(MAILBOX_CAPACITY=8)`，而不是 unbounded。
   - 作用：在 session 堵塞时把压力回传给调用方，避免无限排队导致内存膨胀。

2. **back pressure 策略**
   - 当前使用 `send().await`：当队列满时上游 await。
   - 在 D04 落地时建议加可配置策略：
     - `await`（默认，保序）
     - `try_send + fast-fail`（返回 429/Busy）
     - `send + timeout`（超时后返回可重试错误）

3. **生命周期管理**
   - idle timeout 自动退出，避免大量冷 session 常驻。
   - 显式 `Shutdown` 支持优雅停机。
   - actor 退出后通知 registry 清理条目，避免 handle 泄漏。

4. **并发安全与代际一致性**
   - registry 清理时按 `actor_id` 匹配删除，避免“旧 actor 退出把新 actor entry 删掉”的竞态。

5. **回复语义**
   - 每次命令使用 `oneshot` 回复，调用方可拿到一次性结果。
   - 适合 request/response；后续可叠加 EventStream 做增量事件推送。

## 在 D04（Agent Runtime 拆分）中的落地建议

推荐把 session actor 作为 Runtime Core 的并发边界：

1. gRPC/HTTP handler 收到请求后：
   - 从 `SessionRegistry` 取 handle；
   - 投递 `ProcessMessage`；
   - 等待 oneshot 初始响应（或立即返回 request accepted）。

2. actor 内部串行执行：
   - 状态读取/更新（会话上下文、turn 计数、锁定态）
   - LLM 调用
   - 存储写入
   - EventStream 发布（token、tool、final）

3. idle session 自动回收：
   - 降低常驻资源
   - 活跃 session 自动热启动

## 是否推荐

**推荐。**

相比引入完整 actor 框架（actix/xtra），Tokio 原生实现更轻、可控、依赖少，且已经满足当前“单 session 单写者 + 多 session 并发 + 生命周期回收”的核心需求。

## 可选替代方案（更简单但能力弱）

- `HashMap<session_id, Mutex<SessionState>>`：
  - 优点：实现最简单。
  - 缺点：长耗时 async 工作持锁问题明显，生命周期与背压策略难做精细化。

- “每请求临时 task + 外部分布式锁”：
  - 优点：跨进程可扩展。
  - 缺点：复杂度高、延迟/失败语义更复杂，不适合当前本地 runtime 核心并发控制。

综上，当前阶段采用 Tokio actor/mailbox 是更平衡的方案。
