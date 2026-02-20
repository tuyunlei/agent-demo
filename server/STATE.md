# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: review-pending

---

## 当前阶段

**Phase 1：Walking Skeleton**

## 当前执行中

- **T2.2 Auth 拦截器** — gpt-5.3-codex sub-agent 执行中
  - 分支：`feature/t2.2-auth-interceptor`
  - 验收：ChatService 受保护，AuthService 不受保护

## 阻塞点

暂无。

## 已完成（Phase 1 Walking Skeleton）

- ✅ T0.1 Cargo workspace 初始化
- ✅ T1.1 Proto 编译（tonic-build）
- ✅ T1.2 最小 gRPC 服务（Echo）
- ✅ T1.3 grpcurl 端到端验证（Step 1 完成）
- ✅ T2.1 JWT 认证（AuthPort → AuthService → AuthHandler → HardcodedProvider，4 测试）

## 关键决策

- MVP 不做流式回复，AI 回复整块返回（unary）
- 协议：gRPC（tonic）

---

*最后更新：2026-02-20*
