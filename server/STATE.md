# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: dev-pending

---

## 当前阶段

**Step 4 持久化。** T4.1 PostgreSQL 接入。

## 当前执行中

- T4.1 dev sub-agent（gpt-5.3-codex）：PostgreSQL + sqlx + bcrypt，分支 `feature/t4.1-postgresql`

## 阻塞点

暂无。

## 已完成

- ✅ T0.1~T0.2 工程脚手架
- ✅ T1.1~T1.3 Echo 闭环
- ✅ T2.1~T2.3 真实认证
- ✅ T3.1~T3.4 真实 AI 回复 + 部署
- ✅ 安全加固
- ✅ QG1-4 代码质量 + 测试

## 基础设施

- PostgreSQL 16.11：DB `agentdemo` + user `agentdemo`，TCP 已验证
- 仓库已迁移至 public，CI 免费

---

*最后更新：2026-02-22*
