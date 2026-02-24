# agent-demo/server — ROADMAP

当前阶段：**架构设计**（Phase: design）

---

## 第一步：旧设计文档清理

在写新设计之前，先盘点现有 40+ 个设计文档，逐个判断保留/更新/归档/删除。

- [ ] **D-CLEAN：设计文档审计与清理**
  - 列出所有现有设计文档，逐个标注处置方式（keep / update / archive / delete）
  - 仍然有效的（如 ADR-001 单体架构、ADR-002 PostgreSQL）标记 keep
  - 需要大幅修改的归档到 `docs/design/archive/`，新版在后续任务中重写
  - 完全过时的删除
  - 产出：`docs/design/archive/` 目录 + 清理后的 `docs/design/` 目录

## 第二步：宏观架构（自顶向下）

- [ ] **D-ARCH-01：整体分层架构**
  - 定义 4 层：接入层 / 编排层 / 能力层 / 基础设施层
  - 每层的职责一句话说清
  - 层间依赖方向（只能由外向内）
  - 与 ZeroClaw/PicoClaw/OpenClaw 的对比论证
  - crate 结构映射（每层对应哪些 crate）
  - 产出：`docs/design/architecture.md`

- [ ] **D-ARCH-02：事件流数据模型**
  - 定义 append-only 事件流核心概念
  - 事件类型枚举（UserMessage / AssistantMessage / ToolCallStart / ToolResult / SystemEvent / ConfigChange 等）
  - 与现有 messages 表的差异
  - System Prompt 稳定性原则：首轮构建后不变，变更通过 ConfigChange 事件追加
  - 哪些场景需要"重置"事件流，如何设计
  - 产出：`docs/design/core/event-model.md`

## 第三步：能力层设计（各域独立）

- [ ] **D-CAP-01：LLM Provider 抽象**
  - Provider trait 定义（chat / stream_chat）
  - 请求/响应类型（含 tool_calls、finish_reason）
  - 多 provider 切换 + fallback 策略
  - 参考 ZeroClaw `providers/traits.rs`
  - 产出：`docs/design/capabilities/llm-provider.md`

- [ ] **D-CAP-02：工具系统**
  - Tool trait 定义（spec / execute）
  - 工具注册与发现
  - 内置工具 vs 扩展工具
  - 工具失败处理（错误封装回 LLM）
  - 参考 ZeroClaw `tools/traits.rs`
  - 产出：`docs/design/capabilities/tool-system.md`

- [ ] **D-CAP-03：上下文编排（ContextBuilder）**
  - ContextBuilder 接口设计
  - PromptSection trait（可插拔 section）
  - System Prompt 构建（参考 ZeroClaw `agent/prompt.rs`）
  - 历史消息窗口选取（从事件流中）
  - Token 预算管理
  - 产出：`docs/design/capabilities/context-builder.md`

- [ ] **D-CAP-04：会话生命周期**
  - Session 创建 / 读取 / 更新 / 压缩 / 归档
  - 事件流持久化（EventStore trait）
  - 压缩策略（摘要替代历史事件）
  - 参考 OpenClaw session store + PicoClaw SessionManager
  - 产出：`docs/design/capabilities/session-lifecycle.md`

## 第四步：编排层设计

- [ ] **D-ORCH-01：TurnExecutor 详细设计**
  - 单次 turn 的完整流程（接收消息 → 上下文组装 → LLM 调用 → 工具循环 → 持久化 → 返回）
  - TurnExecutor 依赖哪些能力层模块（ContextBuilder、LlmProvider、ToolRuntime、EventStore）
  - 工具循环终止条件
  - 错误处理（各环节失败如何处理）
  - 参考 ZeroClaw `Agent::turn()` + PicoClaw `runAgentLoop()`
  - 产出：`docs/design/orchestration/turn-executor.md`

## 第五步：接入层 + 基础设施层

- [ ] **D-INFRA-01：gRPC 接口与接入层**
  - gRPC handler 职责边界（只做协议转换，不含业务逻辑）
  - Proto 定义与代码的映射
  - 认证拦截器
  - 产出：`docs/design/infrastructure/grpc-layer.md`

- [ ] **D-INFRA-02：PostgreSQL 适配器**
  - EventStore 的 PostgreSQL 实现
  - 数据库 schema（events 表设计）
  - 迁移策略（从现有 messages 表到 events 表）
  - 产出：`docs/design/infrastructure/postgres-adapter.md`

## 第六步：收尾

- [ ] **D-ADR：更新 ADR（架构决策记录）**
  - 审查现有 6 个 ADR，更新或新增
  - 新增 ADR-007：事件流数据模型
  - 新增 ADR-008：ContextBuilder 可插拔 Section
  - 产出：`docs/design/decisions/` 下更新/新增

---

## 已完成

<details>
<summary>Phase 1 + Phase 2（功能开发，PR #22-27）</summary>

| 功能 | PR |
|------|-----|
| 工具调用链路 + get_current_time | #22 |
| web_search 工具 | #23 |
| 上下文时间戳 + System Prompt 增强 | #24 |
| 多会话管理 | #25 |
| Token 自动刷新 | #26 |
| 统一错误处理 | #27 |

</details>

<details>
<summary>Walking Skeleton + 质量体系（PR #1-21）</summary>

工程脚手架、Echo、认证、AI 回复、部署、持久化、QG1-14、E2E-1~5

</details>

---

*最后更新：2026-02-24*
