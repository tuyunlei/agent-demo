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

- **S01 async trait spike**（sub-agent: s01-async-trait-spike）
  - 状态：running
  - 内容：验证 dyn Trait vs 泛型参数两种方案，给出架构选型推荐
  - brief：`docs/design/briefs/s01-async-trait.md`
  - 输出：`spikes/s01-async-trait/`

## 已完成

- ✅ **D01 架构原则文档更新**（2026-02-19）
  - 补充了 2.7 模块化结构、2.8 Walking Skeleton、4.4 async trait 策略、4.5 所有权即设计
  - review 通过

## 下一步（按优先级）

1. 🔲 **D02 Crate 划分设计**（依赖 S01 结论）
   - workspace crate 结构，每个 crate 的职责和依赖方向
2. 🔲 **D03 Agent Runtime 内部树形拆分**
   - 拆到 3-4 层，覆盖 ContextAssembler、ExecutionLoop、StreamCoordinator、PostProcessor 等子模块
3. 🔲 **S02 Spike：gRPC stream 类型链路**
   - 验证 LLM SSE → internal ChatEvent stream → gRPC server streaming 的 Rust 类型链路

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

*最后更新：2026-02-19 01:15*
