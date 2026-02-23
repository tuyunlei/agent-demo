# agent-demo/mobile — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: idle

---

## 当前阶段

**协作模式切换完成。** 客户端开发转为涂涂本地 Mac + Claude Code 模式。PM 准备任务，涂涂执行。

## 协作模式

- 涂涂在 Mac 上用 Claude Code 开发（读 CLAUDE.md + tasks/）
- PM 在 VPS 上维护 tasks/、review PR、merge
- tasks/ 只有 PM 写，Claude Code 不碰

## 任务队列

见 `tasks/` 目录：
1. `1-mqg4-test-protocol/` — 测试补全 + Protocol 化
2. `2-tm5.2-token-refresh/` — Token 自动刷新
3. `3-tm4.1-grdb-sqlite/` — GRDB + SQLite 本地缓存

## 阻塞点

无。

## 已完成

- ✅ M01-M07 架构设计
- ✅ TM3.1a~TM3.4 Walking Skeleton
- ✅ MQG1 SwiftLint + SwiftFormat CI 门禁
- ✅ MQG2 File size & complexity check
- ✅ MQG-infra SPM Build Plugin
- ✅ FIX-1 Chat 显示 AI 回复
- ✅ MQG3 ViewModel 重构 + 5 个单元测试
- ✅ 仓库迁移至 public（CI 免费）
- ✅ TM-REG 用户注册（PR #7）
- ✅ TM5.1 网络错误处理 + Sign Out（PR #8）

## 已知问题

无。

---

*最后更新：2026-02-24*
