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

- **D03-Rev**（sub-agent: d03-revision）
  - 目标：修订 port.md，去关联类型，改为 DynEventStream
  - 输出：`docs/design/channel-system/port.md`（修改）
- **D04**（sub-agent: d04-agent-runtime-detail）
  - 目标：Agent Runtime 内部树形拆分（基于 S03 actor 结论）
  - 输出：`docs/design/core/agent-runtime-detail.md`

## 已完成

- ✅ D01 架构原则文档更新
- ✅ S01 async trait spike → dyn Trait + async-trait，Arc<dyn Port + Send + Sync>
- ✅ D02 Crate 划分（8 个 crate，DAG 无环）
- ✅ D03 Channel Port trait 设计（7 方法，4 AgentEvent 变体，fanout + 重连）
- ✅ S02 spike → 推荐 Box<dyn Stream>，D03 需去关联类型（→ D03-Rev 处理）
- ✅ S03 spike → Tokio actor 可行，bounded mailbox(8)，idle timeout，代际 ID

## 下一步（D03-Rev/D04 完成后）

1. 🔲 **D05 核心 Port trait 精确定义**（LlmProvider、MessageStore 等方法签名）
2. 🔲 **D06 错误类型体系设计**

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

*最后更新：2026-02-19 04:15*
