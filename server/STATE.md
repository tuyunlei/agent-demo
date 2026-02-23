# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: idle

---

## 当前阶段

**Agent 核心能力开发。** 按 ROADMAP 顺序推进：工具调用 → web_search → 上下文时间戳 → 多会话 → Token 刷新 → 统一错误处理。

## 任务队列

1. 🔲 工具调用链路 + get_current_time（PR #22）
2. 🔲 web_search 工具（PR #23）
3. 🔲 上下文时间戳 + System Prompt 增强（PR #24）
4. 🔲 多会话管理（PR #25）
5. 🔲 Token 自动刷新（PR #26）
6. 🔲 统一错误处理（PR #27）

## 当前执行中

无。准备派第一个任务。

## 阻塞点

无。

## 已完成

- ✅ T0.1~T0.2 工程脚手架
- ✅ T1.1~T1.3 Echo 闭环
- ✅ T2.1~T2.3 真实认证
- ✅ T3.1~T3.4 真实 AI 回复 + 部署
- ✅ 安全加固
- ✅ QG1-14 质量体系
- ✅ E2E-1~5 专业 e2e 测试体系
- ✅ T4.1~T4.3 持久化
- ✅ T6.1 ListSessionMessages

## 已知待修

- user_message_id 返回固定 "msg-001"，应返回真实 message_id

## 基础设施

- PostgreSQL 16.11：users + sessions + messages 表（含 sequence_num BIGSERIAL）
- 仓库 public，CI 免费
- 测试：72+ 个（单元 + 集成 + e2e + proptest + arch），覆盖率 65%+
- Mutation catch rate：100%
- CI 门禁：fmt + clippy(cognitive ≤10, too_many_lines ≤50) + Rust arch tests + tests + coverage ≥65%

---

*最后更新：2026-02-23*
