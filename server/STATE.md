# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: idle

---

## 当前阶段

**Step 4 持久化全部完成。** 🎉 等涂涂确认下一步方向（Step 5 韧性 or Step 6 会话管理）。

## 当前执行中

无。

## 阻塞点

暂无。

## 已完成

- ✅ T0.1~T0.2 工程脚手架
- ✅ T1.1~T1.3 Echo 闭环
- ✅ T2.1~T2.3 真实认证
- ✅ T3.1~T3.4 真实 AI 回复 + 部署
- ✅ 安全加固
- ✅ QG1-4 代码质量 + 测试
- ✅ T4.1 PostgreSQL 接入（PR #1）
- ✅ T4.2 真实用户注册（PR #2）
- ✅ T4.3 消息持久化（PR #3）
- ✅ QG5 测试分离 + 文件限制强化（PR #4）
- ✅ QG6 覆盖率 CI 门禁（PR #5，阈值 54%）
- ✅ QG7 集成测试（PR #6，5 个 sqlx::test）

## 已知待修

- user_message_id 返回固定 "msg-001"，应返回真实 message_id

## 基础设施

- PostgreSQL 16.11：users + sessions + messages 表（含 sequence_num BIGSERIAL）
- 仓库 public，CI 免费
- 测试：38 个（33 单元 + 5 集成），覆盖率 54%
- CI 门禁：fmt + clippy + arch deps + file size + tests + coverage ≥54%
- 部署已验证：注册 → 登录 → 发消息 → AI 回复 → DB 持久化 ✅

---

*最后更新：2026-02-22*
