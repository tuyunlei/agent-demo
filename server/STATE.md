# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: dev-pending

---

## 当前阶段

**Phase 1：Walking Skeleton**

## 当前执行中

- **T3.2 Agent Runtime 最小路径** — gpt-5.3-codex sub-agent 开发中
  - 分支：`feature/t3.2-agent-runtime`
  - 验收：SendMessage → LLM(Kimi K2.5) → 返回 AI 回复

## 阻塞点

暂无。

## 已完成（Phase 1 Walking Skeleton）

- ✅ T0.1 Cargo workspace 初始化
- ✅ T0.2 GitHub Actions CI
- ✅ T1.1 Proto 编译（tonic-build）
- ✅ T1.2 最小 gRPC 服务（Echo）
- ✅ T1.3 grpcurl 端到端验证（Step 1 完成）
- ✅ T2.1 JWT 认证（六边形四层贯通）
- ✅ T2.2 Auth 拦截器（ChatService 受保护）
- ✅ T2.3 grpcurl 完整认证验证（Step 2 完成）
- ✅ T3.1 LlmProvider Port + OpenAI 兼容适配器（review passed）
- ✅ T3.3 Caddy + TLS 部署（REDACTED_HOST:8443）
- ✅ 安全加固（JWT secret + 密码环境变量化）

## 关键决策

- MVP 不做流式回复，AI 回复整块返回（unary）
- 协议：gRPC（tonic）
- LLM: Kimi K2.5 via volcengine OpenAI 兼容 API
- 部署：Caddy 反代 gRPC over TLS，systemd user service
- 安全：JWT secret + 登录凭证从环境变量读取

---

*最后更新：2026-02-21*
