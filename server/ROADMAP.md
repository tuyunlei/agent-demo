# agent-demo/server — ROADMAP

> 架构设计初稿已完成（14 模块）。当前阶段：深化设计 + 可行性验证，双轨并行。
> 详细历史设计见 `docs/design/`

---

## 当前阶段：Phase 0 — 架构深化 + 可行性验证

### 轨道 A：可行性验证（Spikes）

每个 spike 只验证一个技术问题，以 `cargo check` 通过 + 明确结论为成功标准。

| # | 任务 | 状态 | 位置 | 验证目标 |
|---|------|------|------|----------|
| S01 | async trait：dyn vs 泛型 | 🔲 | `spikes/s01-async-trait/` | 7-8 个 Port 注入时，两种方案的代码形态、编译结果、权衡 |
| S02 | gRPC stream 类型链路 | 🔲 | `spikes/s02-grpc-stream/` | LLM SSE → ChatEvent → gRPC server streaming 类型链路能否编译 |
| S03 | Tokio actor/mailbox（session 单写者） | 🔲 | `spikes/s03-tokio-actor/` | session 单写者模型的 actor pattern 在 Rust 里怎么实现 |
| S04 | Arc vs 生命周期注入 | 🔲 | `spikes/s04-di-pattern/` | Port 依赖注入用 Arc<dyn Trait> 还是 &dyn Trait，所有权影响 |

### 轨道 B：深化设计（Design Docs）

把模块内部拆到 3-4 层树形结构，接口精确到方法签名。

| # | 任务 | 状态 | 位置 | 内容 |
|---|------|------|------|------|
| D01 | 架构原则文档更新 | 🔲 | `docs/design/principles.md` | 补充树形模块化要求、Rust 特有约束 |
| D02 | Crate 划分设计 | 🔲 | `docs/design/crate-structure.md` | workspace crate 结构，每个 crate 职责和依赖方向 |
| D03 | Agent Runtime 树形拆分 | 🔲 | `docs/design/core/agent-runtime-detail.md` | 内部拆到 3-4 层：RequestDispatcher、ContextAssembler、ExecutionLoop、StreamCoordinator、PostProcessor |
| D04 | Port trait 接口定义 | 🔲 | `docs/design/ports/` | 所有 Port 的精确方法签名、入参类型、错误类型 |
| D05 | 错误类型体系设计 | 🔲 | `docs/design/error-types.md` | 跨模块错误传播策略，thiserror vs anyhow 选型 |

### 依赖关系

```
S01 → D02（crate 划分依赖 trait 方案选型）
S02 → D04（Port 签名依赖 stream 类型确认）
S03 → D03（Runtime 拆分依赖 actor 方案）
D01 → D02 → D03 → D04（设计文档有顺序依赖）
```

---

## Phase 1：Walking Skeleton（待 Phase 0 完成后开始）

最小可运行版本：SendMessage → 简化 Agent Runtime → LLM 调用 → 流式返回 → 存 PG

具体任务待 Phase 0 完成后拆分。

---

## Phase 2：逐步完善（待 Phase 1 跑通后）

- 加认证完整流程
- 加 Persona 系统
- 加工具调用
- 加记忆系统
- 加真正的 iOS 客户端

---

*最后更新：2026-02-17*
