# agent-demo/server — 当前状态

每次唤醒时首先读这个文件。

---

## Phase: idle

---

## 当前阶段

**Phase 1：Walking Skeleton — Step 3 完成 🎉**

## 当前执行中

无。Server 端 Walking Skeleton e2e 已跑通。

## 阻塞点

暂无。

## 已完成（Phase 1 Walking Skeleton）

- ✅ T0.1 Cargo workspace 初始化
- ✅ T0.2 GitHub Actions CI
- ✅ T1.1~T1.3 Echo 闭环（Step 1 完成）
- ✅ T2.1~T2.3 真实认证（Step 2 完成）
- ✅ T3.1 LlmProvider Port + OpenAI 兼容适配器
- ✅ T3.2 Agent Runtime 最小路径（Kimi K2.5）
- ✅ T3.3 Caddy + TLS 部署（REDACTED_HOST:8443）
- ✅ T3.4 公网 e2e 验证：Login → SendMessage → AI 回复 ✅
- ✅ 安全加固（JWT secret + 密码环境变量化）

## 关键决策

- MVP 不做流式回复，AI 回复整块返回（unary）
- SendMessageResponse 增加 assistant_content 字段（MVP inline 返回）
- 协议：gRPC（tonic）+ TLS（Caddy 反代）
- LLM: Kimi K2.5 via volcengine OpenAI 兼容 API
- 部署：systemd user service（server + caddy）

---

*最后更新：2026-02-21*
