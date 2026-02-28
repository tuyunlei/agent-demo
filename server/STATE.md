# agent-demo/server — 状态

## Phase: dev-pending

Lint 强化进行中。PR #22（阈值收紧）等 CI。

## 当前进度

- [x] 框架分层架构源码调研
- [x] 新架构设计文档（11 个任务全部完成）
- [x] 代码重构对齐设计（R-01 ~ R-09 全部完成）
- [x] 测试覆盖率提升：65% → 85.96%（CI 阈值 85%）
- [x] Docker Compose 部署（PR #18）
- [x] Provider Capabilities（PR #19）
- [x] **L-01~L-04：Lint 配置 + cast 审计 + unwrap 禁令 + 函数拆分**（PR #20 ✅ merged）
- [x] **L-05：must_use 审计**（PR #21 ✅ merged）
- [ ] **阈值收紧：30 行 / 复杂度 10**（PR #22 等 CI）

## 部署架构

```
Internet → :443 TLS → Caddy (host, shared) → localhost:50051/50052 h2c → server → postgres
```

- Preview: `preview-agent.xclz.org` → localhost:50051
- Production: `agent.xclz.org` → localhost:50052

## 阻塞点

- develop → main merge 需要涂涂确认

---

*最后更新：2026-02-28 22:15 CST*
