# agent-demo/server — ROADMAP

> 架构设计初稿已完成（14 模块）。当前阶段：深化设计 + 可行性验证，双轨并行。
> 详细历史设计见 `docs/design/`

---

## 架构认知基线

**Channel 是核心抽象，gRPC 只是一个 Channel Adapter 实现。**
服务端通过 `ChannelAdapter` Port 收发消息，不关心具体协议。
iOS 客户端（gRPC）、未来 Web 客户端（WebSocket）、Push 等都是这个 Port 的不同实现。

---

## 当前阶段：Phase 0 — 架构深化 + 可行性验证

### 轨道 A：可行性验证（Spikes）

每个 spike 只验证一个技术问题，以 `cargo check` 通过 + 明确结论为成功标准。

| # | 任务 | 状态 | 位置 | 验证目标 |
|---|------|------|------|----------|
| S01 | async trait：dyn vs 泛型 | ✅ | `spikes/s01-async-trait/` | **结论：dyn Trait + async-trait，Arc<dyn Port + Send + Sync>** |
| S02 | gRPC stream 类型链路 | 🔲 | `spikes/s02-grpc-stream/` | gRPC Channel Adapter 里 LLM ChatEvent → gRPC server streaming 能否编译 |
| S03 | Tokio actor/mailbox（session 单写者） | 🔲 | `spikes/s03-tokio-actor/` | session 单写者模型的 actor pattern 在 Rust 里怎么实现 |

> S04（Arc vs 生命周期注入）已被 S01 覆盖，不再单独做。

### 轨道 B：深化设计（Design Docs）

把模块内部拆到 3-4 层树形结构，接口精确到方法签名。

| # | 任务 | 状态 | 位置 | 内容 |
|---|------|------|------|------|
| D01 | 架构原则文档更新 | ✅ | `docs/design/principles.md` | 模块化结构、Walking Skeleton、Rust 约束 |
| D02 | Crate 划分设计 | 🔲 | `docs/design/crate-structure.md` | workspace crate 结构，每个 crate 职责和依赖方向（基于 S01 dyn 结论） |
| D03 | Channel Port trait 设计 | 🔲 | `docs/design/channel-system/port.md` | ChannelAdapter trait 精确定义：收消息、推 AgentEvent；协议无关；支持多 Channel 并发 |
| D04 | Agent Runtime 树形拆分 | 🔲 | `docs/design/core/agent-runtime-detail.md` | 内部拆到 3-4 层：RequestDispatcher、ContextAssembler、ExecutionLoop、StreamCoordinator、PostProcessor |
| D05 | 核心 Port trait 精确定义 | 🔲 | `docs/design/ports/` | LlmProvider、MessageStore、SessionStore、MemoryStore、ToolExecutor 的方法签名 + 错误类型 |
| D06 | 错误类型体系设计 | 🔲 | `docs/design/error-types.md` | 跨模块错误传播策略，thiserror 用法 |

### 依赖关系

```
S01 ✅ → D02（crate 划分依赖 dyn 方案确认，现在可以开始）
S02    → D03（gRPC Channel Adapter 实现细节依赖 stream 类型验证）
S03    → D04（Agent Runtime 拆分依赖 actor 方案）
D02    → D05（Port 精确签名依赖 crate 边界确认）
D03    → D05（Channel Port 需要引用 AgentEvent 类型）
```

---

## Phase 1：Walking Skeleton（待 Phase 0 完成后开始）

最小可运行版本：SendMessage → 简化 Agent Runtime → LLM 调用 → 返回

具体任务待 Phase 0 完成后拆分。

---

## Phase 2：逐步完善（待 Phase 1 跑通后）

- 加认证完整流程
- 加 Persona 系统
- 加工具调用
- 加记忆系统
- 加真正的 iOS 客户端

---

*最后更新：2026-02-19*
