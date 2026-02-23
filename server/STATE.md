# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: idle

---

## 当前阶段

**质量体系全部完成（QG1-14 + E2E-1~5）。** 等涂涂确认下一步功能开发方向。

## 当前执行中

无。

## 阻塞点

等涂涂确认功能开发优先级：T5.1 统一错误处理 → T6.2+ListSessions → RefreshToken → T5.2

## 已完成

- ✅ T0.1~T0.2 工程脚手架
- ✅ T1.1~T1.3 Echo 闭环
- ✅ T2.1~T2.3 真实认证
- ✅ T3.1~T3.4 真实 AI 回复 + 部署
- ✅ 安全加固
- ✅ QG1-14 质量体系（fmt/clippy/arch/filesize/tests/coverage/mutation/proptest/e2e/Rust arch tests）
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
