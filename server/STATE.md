# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## 当前 Phase

**Phase 0：架构深化 + 可行性验证**

架构设计（14 个模块）已完成初稿，现在进入深化阶段：
- 把模块内部拆到 3-4 层（树形，不是平铺）
- 每遇到技术不确定点 → 拆出 spike 任务，sub-agent 用 Rust 实际验证
- 两条轨道并行：**深化设计** + **可行性验证（spikes/）**

## 当前执行中

（无，等待涂涂进行 Phase 0 整体 review）

## ⏸ 阻塞点

**等待涂涂 review Phase 0 全部产出，决定是否进入 Phase 1（Walking Skeleton 实现）。**

## 已完成

- ✅ D01 架构原则文档更新（分层模块化、Walking Skeleton、Rust 约束）
- ✅ S01 async trait spike → **dyn Trait + async-trait，Arc<dyn Port + Send + Sync>**
- ✅ D02 Crate 划分（8 个 crate，DAG 无环）
- ✅ D03 Channel Port trait 设计 + D03-Rev（DynEventStream，对象安全）
- ✅ S02 spike → **Box<dyn Stream> 方案**，关联类型不可用于 dyn
- ✅ S03 spike → **Tokio actor 可行**，bounded mailbox(8)，idle timeout，代际 ID
- ✅ D04 Agent Runtime 树形拆分（3 层 9 子模块）
- ✅ D05 核心 Port 精确定义（5 个 Port，19 个方法）
- ✅ D06 错误类型体系（DomainError 8 变体，三层转换，thiserror 为主）

## 关键决策（已定）

- **ChatService 协议**：三接口（SendMessage unary + Subscribe server streaming + SubmitToolResult unary），不用双向流
- **按 boundary 分块**：MVP 不做逐字符流式，每个 TextChunk 是完整的一段文字
- **Client-side tools**：通过 SubmitToolResult 回传，服务端 pending map 协调
- **协议**：先 gRPC 试水，遇到问题再评估是否加 WebSocket

## 阻塞点

暂无。

## 待澄清

- user_id 是否冗余存储到 sessions/messages 表？（倾向于加，但未定）

---

*最后更新：2026-02-19 06:15*
