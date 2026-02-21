# agent-demo/mobile — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: review-pending

---

## 当前阶段

**iOS 质量底线建设中。** 推进顺序：MQG2 → MQG-infra（SPM plugin） → MQG3（ViewModel + 单测） → FIX-1（AI 回复显示）→ 暂停等涂涂验收。

## 当前执行中

MQG2 PR #8：CI watch 运行中 + review sub-agent 已派出。

## 阻塞点

暂无。

## 已完成

- ✅ M01-M07 架构设计
- ✅ TM3.1a Xcode 项目初始化
- ✅ TM3.1b grpc-swift v2 + proto 编译
- ✅ TM3.2 GitHub Actions iOS CI
- ✅ TM3.3 Login 页面
- ✅ TM3.4 Chat 页面
- ✅ MQG1 SwiftLint + SwiftFormat CI 门禁（PR #7）

## 已知问题

- Chat 页面显示 `[sent] request_id=...` 而非 AI 回复（FIX-1，依赖 MQG-infra 重新生成 proto）

---

*最后更新：2026-02-22*
