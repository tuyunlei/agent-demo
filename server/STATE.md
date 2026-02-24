# agent-demo/server — 状态

## Phase: design

架构重新设计中。不写代码，只产出设计文档。

## 当前进度

- [x] 框架分层架构源码调研（docs/research/framework-architecture-comparison.md）
- [x] 涂涂确认架构方向（4 层 + trait 按域分布 + ContextBuilder 独立 + 事件流）
- [x] 自动推进机制重新设计（system prompt / STATE / WORKFLOW / ticker）
- [ ] 旧设计文档清理（删/更新/归档）
- [ ] 新架构设计文档（见 ROADMAP.md）

## 阻塞点

无。

---

## 已完成的里程碑

<details>
<summary>Phase 1 + Phase 2（6 tasks，38 min，PR #22-27）</summary>

- ✅ 工具调用链路 + get_current_time（PR #22）
- ✅ web_search 工具（PR #23）
- ✅ 上下文时间戳 + System Prompt 增强（PR #24）
- ✅ 多会话管理（PR #25）
- ✅ Token 自动刷新（PR #26）
- ✅ 统一错误处理（PR #27）
</details>

<details>
<summary>Walking Skeleton + 质量体系（PR #1-21）</summary>

- ✅ 工程脚手架 + CI
- ✅ Echo 闭环 + 真实认证 + AI 回复 + 部署
- ✅ PostgreSQL 持久化 + 分页查询
- ✅ QG1-14 质量门禁 + E2E-1~5 端到端测试
- 91 个测试，覆盖率 65%+，mutation catch rate 100%
</details>

## 基础设施

- PostgreSQL 16.11：users + sessions + messages
- 仓库 public，CI 免费（macOS runner 可用）
- CI 门禁：fmt + clippy + arch tests + tests + coverage ≥65%

---

*最后更新：2026-02-24*
