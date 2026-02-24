# agent-demo/server — ROADMAP

当前阶段：**代码重构**（Phase: refactor）

目标：代码完全对齐架构设计文档（`docs/design/`）。

---

## 重构任务

按自底向上顺序：先稳定基础类型，再构建能力层，最后重构编排层和接入层。

### 第一步：基础类型整理

- [x] **R-01：agent-domain 重构 + agent-types 合并** (PR #5)
  - 将 agent-types 的内容合并到 agent-domain
  - 定义 EventEnvelope / EventMeta / EventPayload 等事件类型（对齐 event-model.md）
  - 定义 Session / SessionStatus 等类型（对齐 session-lifecycle.md）
  - 更新所有 crate 的依赖从 agent-types → agent-domain
  - 删除 agent-types crate
  - 设计文档：`core/event-model.md`、`capabilities/session-lifecycle.md`
  - 验收：编译通过 + 现有测试全绿

### 第二步：能力层 crate 创建

- [x] **R-02：新建 agent-context crate** (PR #6)
  - 定义 PromptSection trait + ContextBuilder trait
  - 实现内置 section（Identity / Safety / Tools / DateTime / Runtime）
  - 实现默认 ContextBuilder
  - 设计文档：`capabilities/context-builder.md`
  - 验收：单元测试覆盖所有内置 section + builder

- [x] **R-03：新建 agent-tools crate** (PR #7)
  - 定义 Tool trait + ToolRuntime trait
  - 从 agent-app 迁移现有工具代码（get_current_time、web_search）
  - 实现 DefaultToolRuntime
  - 设计文档：`capabilities/tool-system.md`
  - 验收：现有工具测试迁移 + ToolRuntime 单元测试

- [x] **R-04：agent-llm 重构** (PR #8)
  - LlmProvider trait 对齐 `capabilities/llm-provider.md`（complete + stream）
  - 统一 LlmRequest / LlmResponse / LlmError 类型
  - MockLlmProvider 更新
  - 设计文档：`capabilities/llm-provider.md`
  - 验收：现有 LLM 测试 + 新 trait 测试

- [x] **R-05：新建 agent-memory crate（最小骨架）** (PR #9)
  - 定义 MemoryProvider trait
  - 提供 NoopMemoryProvider 默认实现
  - 设计文档：`capabilities/session-lifecycle.md`（记忆部分）
  - 验收：trait 定义 + noop 实现 + 编译通过

### 第三步：编排层重构

- [x] **R-06：agent-app → agent-orchestrator 重命名 + 重构** (PR #10 rename, PR #11 TurnExecutor)
  - 重命名 crate
  - 实现 TurnExecutor（对齐 turn-executor.md）
  - 实现 SessionLifecycle
  - 依赖能力层 trait，不依赖具体实现
  - 设计文档：`orchestration/turn-executor.md`
  - 验收：TurnExecutor 单元测试（mock 所有 trait）+ 工具循环测试

### 第四步：基础设施层重构

- [ ] **R-07：agent-storage 重构（EventStore 实现）**
  - 实现 EventStore trait 的 PostgreSQL 适配器
  - 新建 events 表（migration）
  - 保留现有 messages 表兼容
  - 设计文档：`infrastructure/postgres-adapter.md`
  - 验收：EventStore 集成测试（sqlx::test）

### 第五步：接入层 + DI 重构

- [ ] **R-08：agent-channel + agent-server 重构**
  - Channel handler 只做协议转换（对齐 grpc-layer.md）
  - agent-server 作为 Composition Root 做 DI 组装
  - 错误码映射对齐设计
  - 设计文档：`infrastructure/grpc-layer.md`
  - 验收：e2e 测试全绿 + 新的 DI 组装测试

### 第六步：收尾

- [ ] **R-09：arch 测试更新 + 覆盖率修复**
  - 更新 arch.rs 中的 crate 依赖方向测试
  - 确保新 crate 的依赖方向符合设计
  - 覆盖率 ≥65%
  - 验收：CI 全绿

---

## ✅ 已完成：架构设计

<details>
<summary>11 个设计文档（~9500 行）</summary>

| 任务 | 产出 |
|------|------|
| D-CLEAN | 旧文档清理（18 archived + 11 deleted） |
| D-ARCH-01 | architecture.md（853 行） |
| D-ARCH-02 | core/event-model.md（1040 行） |
| D-CAP-01 | capabilities/llm-provider.md（928 行） |
| D-CAP-02 | capabilities/tool-system.md（809 行） |
| D-CAP-03 | capabilities/context-builder.md（1274 行） |
| D-CAP-04 | capabilities/session-lifecycle.md（1269 行） |
| D-ORCH-01 | orchestration/turn-executor.md（1358 行） |
| D-INFRA-01 | infrastructure/grpc-layer.md（995 行） |
| D-INFRA-02 | infrastructure/postgres-adapter.md（1005 行） |
| D-ADR | ADR 004-008 更新/新增 |

</details>

<details>
<summary>Phase 1 + Phase 2 功能开发（PR #22-27）</summary>

工具调用 + web_search + 上下文时间戳 + 多会话 + Token 刷新 + 统一错误

</details>

<details>
<summary>Walking Skeleton + 质量体系（PR #1-21）</summary>

91 个测试，覆盖率 65%+，mutation catch rate 100%

</details>

---

*最后更新：2026-02-24*
