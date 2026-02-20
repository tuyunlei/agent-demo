# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: dev-pending

---

## 当前阶段

**Phase 1：Walking Skeleton**

Phase 0 架构设计已完成（D01-D06 + S01-S03），进入实现阶段。
按 ROADMAP 的 Task 逐个推进，Server 先行（Step 0-2），Mobile 从 Step 3 加入。

## 当前执行中

- **T0.1 Cargo workspace 初始化** — gpt-5.3-codex sub-agent 执行中
  - 分支：`feature/t0.1-cargo-workspace`
  - 验收：`cargo check --workspace` + `cargo test --workspace` + `cargo run -p agent-server`

## 阻塞点

暂无。

## 已完成（Phase 0 架构设计）

- ✅ D01 架构原则文档更新（分层模块化、Walking Skeleton、Rust 约束）
- ✅ S01 async trait spike → dyn Trait + async-trait，Arc<dyn Port + Send + Sync>
- ✅ D02 Crate 划分（8 个 crate，DAG 无环）
- ✅ D03 Channel Port trait 设计 + D03-Rev（DynEventStream，对象安全）
- ✅ S02 spike → Box<dyn Stream> 方案，关联类型不可用于 dyn
- ✅ S03 spike → Tokio actor 可行，bounded mailbox(8)，idle timeout，代际 ID
- ✅ D04 Agent Runtime 树形拆分（3 层 9 子模块）
- ✅ D05 核心 Port 精确定义（5 个 Port，19 个方法）
- ✅ D06 错误类型体系（DomainError 8 变体，三层转换，thiserror 为主）

## 关键决策

- **MVP 不做流式回复**：AI 回复整块返回（unary），不用 server streaming
- **ChatService MVP 简化**：SendMessage 同步返回 AI 回复，暂不需要 Subscribe 和 SubmitToolResult
- **按 boundary 分块**：后续加流式时按 boundary 分块，不逐字符
- **协议**：先 gRPC（tonic），遇到问题再评估

---

*最后更新：2026-02-20*
