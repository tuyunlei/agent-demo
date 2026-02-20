# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: idle

---

## 当前阶段

**Phase 1：Walking Skeleton**

Phase 0 架构设计已完成（D01-D06 + S01-S03），进入实现阶段。
按 ROADMAP 的 Task 逐个推进，Server 先行（Step 0-2），Mobile 从 Step 3 加入。

## 当前执行中

（无）

## 阻塞点

暂无。

## 已完成（Phase 1 Walking Skeleton）

- ✅ T0.1 Cargo workspace 初始化（8 crate，DAG 依赖，cargo check/test 通过）
- ✅ T1.1 Proto 编译（tonic-build，4 proto → Rust，AuthService/ChatService/SessionService 生成）
- ✅ T1.2 最小 gRPC 服务（ChatServiceHandler echo，agent-server 监听 [::1]:50051，单元测试通过）
- ✅ T1.3 grpcurl 端到端验证（SendMessage echo 通过，Step 1 完成）

## 已完成（Phase 0 架构设计）

- ✅ D01-D06 设计文档 + S01-S03 可行性验证（全部完成）

## 关键决策

- **MVP 不做流式回复**：AI 回复整块返回（unary），不用 server streaming
- **ChatService MVP 简化**：SendMessage 同步返回 AI 回复，暂不需要 Subscribe 和 SubmitToolResult
- **协议**：先 gRPC（tonic），遇到问题再评估

---

*最后更新：2026-02-20*
