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

- **S02 spike**（sub-agent: s02-channel-adapter-dyn）
  - 状态：running
  - 目标：验证 ChannelAdapter 关联类型 vs Box<dyn Stream>，确认 D03 trait 是否需要修改
  - 输出：`spikes/s02-channel-adapter-dyn/`
- **S03 spike**（sub-agent: s03-tokio-actor）
  - 状态：running
  - 目标：验证 session 单写者 actor pattern（Tokio mpsc + oneshot）
  - 输出：`spikes/s03-tokio-actor/`

## 已完成

- ✅ **D01 架构原则文档更新**（2026-02-19）
  - review 通过
- ✅ **S01 async trait spike**（2026-02-19）
  - 结论：dyn Trait + async-trait，Arc<dyn Port + Send + Sync>
- ✅ **D02 Crate 划分设计**（2026-02-19）
  - 8 个 crate，DAG 无环，产出：`docs/design/crate-structure.md`
  - review 通过
- ✅ **D03 Channel Port trait 设计**（2026-02-19）
  - ChannelAdapter 7 个方法，AgentEvent 4 个变体，fanout + 重连语义设计完整
  - review 通过（关联类型问题待 S02 验证后可能微调）

## 下一步（S02/S03 完成后）

1. 🔲 **D04 Agent Runtime 内部树形拆分**（依赖 S03 结论）
2. 🔲 **D05 核心 Port trait 精确定义**（LlmProvider、MessageStore 等方法签名）
3. 🔲 **D06 错误类型体系设计**

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

*最后更新：2026-02-19 03:15*
