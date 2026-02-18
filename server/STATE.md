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

（暂无，刚迁移完，准备开始）

## 下一步（按优先级）

1. 🔲 **更新架构原则文档**（`docs/design/principles.md`）
   - 补充今天讨论的内容：模块化要求树形拆分而非平铺、Rust 特有约束等
2. 🔲 **Crate 划分设计**（新任务）
   - 物理模块边界：workspace crate 结构，每个 crate 的职责和依赖方向
3. 🔲 **Agent Runtime 内部树形拆分**（新任务）
   - 拆到 3-4 层，覆盖 ContextAssembler、ExecutionLoop、StreamCoordinator、PostProcessor 等子模块
4. 🔲 **Spike：async trait dyn vs 泛型**（spike 任务）
   - 验证 7-8 个 Port 注入时，dyn Trait + async 和泛型参数两种方案的代码形态和编译结果
5. 🔲 **Spike：stream 类型链路**
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

*最后更新：2026-02-17*
