# agent-demo/mobile — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: idle

---

## 当前阶段

**等待 Server Step 3 完成后加入**

Phase 0 架构设计已完成（M01-M07），Server 先行完成 Step 0-2，Mobile 从 Step 3 开始实现。

## 当前执行中

（无）

## 阻塞点

等待 Server 端完成 Step 0-2（脚手架 + Echo 闭环 + 认证），有可用的 gRPC 端点后 Mobile 开始。

## 已完成（Phase 0 架构设计）

- ✅ M01 架构原则 + 分层规范（`docs/design/principles.md`）
- ✅ M02 技术选型 + 约束（`docs/design/tech-stack.md`）
- ✅ M03 基础层设计（`docs/design/foundation.md`）
- ✅ M04 服务层设计（`docs/design/services.md`）
- ✅ M05 业务模块划分（`docs/design/business-modules.md`）
- ✅ M06 应用集成层设计（`docs/design/app-integration.md`）
- ✅ M07 Proto 定义（`proto/` 目录 4 个 .proto 文件）

## 已定的架构决策

- **四层分层**：应用集成层 → 业务层 → 服务层 → 基础层，依赖只能向下
- **页面级模式**：State + Action + Reducer + Effect
- **技术栈**：iOS 17+, Swift, SwiftUI, SPM, grpc-swift, GRDB
- **MVP 只做 Login + Chat 两个页面**
- **不做流式回复**：AI 回复整块返回

---

*最后更新：2026-02-20*
