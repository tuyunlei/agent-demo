# agent-demo/server — 状态

## Phase: idle

代码重构完成。代码已对齐新架构设计文档。等待涂涂确认下一步方向。

## 当前进度

- [x] 框架分层架构源码调研
- [x] 涂涂确认架构方向
- [x] 自动推进机制重新设计
- [x] 旧设计文档清理
- [x] 新架构设计文档（11 个任务全部完成）
- [x] 代码重构对齐设计（R-01 ~ R-09 全部完成）

## 阻塞点

- develop → main merge 需要涂涂确认

---

## 已完成的里程碑

<details>
<summary>代码重构阶段（R-01 ~ R-09，PR #5-14）</summary>

- R-01: agent-domain 重构 + agent-types 合并 (PR #5)
- R-02: agent-context crate (PR #6)
- R-03: agent-tools crate (PR #7)
- R-04: agent-llm 重构 (PR #8)
- R-05: agent-memory crate (PR #9)
- R-06a: agent-orchestrator rename (PR #10)
- R-06b: TurnExecutor 实现 (PR #11)
- R-07: EventStore 实现 (PR #12)
- R-08: agent-channel + agent-server 重构 (PR #13)
- R-09: arch 测试更新 + 覆盖率修复 (PR #14)

最终架构：4 层 13 crate，93+ 测试，CI 全绿
</summary>
</details>

<details>
<summary>架构设计阶段（11 个设计文档，~9500 行）</summary>

- D-CLEAN：旧文档清理（18 archived + 11 deleted）
- D-ARCH-01：整体分层架构（architecture.md）
- D-ARCH-02：事件流数据模型（core/event-model.md）
- D-CAP-01~04：能力层（llm-provider / tool-system / context-builder / session-lifecycle）
- D-ORCH-01：编排层（orchestration/turn-executor.md）
- D-INFRA-01~02：基础设施层（grpc-layer / postgres-adapter）
- D-ADR：ADR 004-008 更新/新增
</details>

<details>
<summary>Phase 1 + Phase 2（6 tasks，PR #22-27）</summary>

- ✅ 工具调用 + web_search + 上下文时间戳 + 多会话 + Token 刷新 + 统一错误
</details>

<details>
<summary>Walking Skeleton + 质量体系（PR #1-21）</summary>

- 91 个测试，覆盖率 65%+，mutation catch rate 100%
</details>

## 基础设施

- PostgreSQL 16.11：users + sessions + messages + events
- 仓库 public，CI 免费
- CI 门禁：fmt + clippy + arch tests + tests + coverage ≥65%

---

*最后更新：2026-02-25*
