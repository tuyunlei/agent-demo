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

- **D05**（sub-agent: d05-port-traits）
  - 目标：5 个核心 Port 精确方法签名（LlmProvider/MessageStore/SessionStore/MemoryStore/PersonaStore）
  - 输出：`docs/design/ports/core-ports.md`
- **D06**（sub-agent: d06-error-types）
  - 目标：错误类型体系（三层错误、thiserror vs anyhow、转换规则）
  - 输出：`docs/design/error-types.md`

## 已完成

- ✅ D01 架构原则文档更新
- ✅ S01 async trait spike → dyn Trait + async-trait，Arc<dyn Port + Send + Sync>
- ✅ D02 Crate 划分（8 个 crate，DAG 无环）
- ✅ D03 Channel Port trait 设计 → 修订（DynEventStream，对象安全）
- ✅ S02 spike → Box<dyn Stream> 方案，D03 关联类型问题已修复
- ✅ S03 spike → Tokio actor 可行，bounded mailbox(8)，idle timeout，代际 ID
- ✅ D04 Agent Runtime 树形拆分（3 层 9 子模块，EventPublisher → ChannelAdapter）

## 下一步（D05/D06 完成后）

1. 🔲 **整体 review**：拿涂涂来审阅 Phase 0 的全部产出，决定是否进入 Phase 1

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

*最后更新：2026-02-19 05:15*
