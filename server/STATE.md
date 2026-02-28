# agent-demo/server — 状态

## Phase: idle

PR #19 (Provider Capabilities) merged。Lint 强化 ROADMAP 已规划（L-01 ~ L-05）。

## 当前进度

- [x] 框架分层架构源码调研
- [x] 新架构设计文档（11 个任务全部完成）
- [x] 代码重构对齐设计（R-01 ~ R-09 全部完成）
- [x] 测试覆盖率提升：65% → 85.96%（CI 阈值 85%）
- [x] Docker Compose 部署（PR #18）
- [x] Provider Capabilities（PR #19）— stateful API、builtin tools、compaction layering、ADR-009
- [ ] **Lint 强化**（L-01 ~ L-05）— 待开始

## 下一步

L-01（lint 配置骨架）→ L-02（cast 审计）→ L-03（unwrap 禁令）→ L-04（拆分大函数）→ L-05（must_use 审计）

## 部署架构

```
Internet → :443 TLS → Caddy (host, shared) → localhost:50051/50052 h2c → server → postgres
```

- Preview: `preview-agent.xclz.org` → localhost:50051
- Production: `agent.xclz.org` → localhost:50052

## 阻塞点

- develop → main merge 需要涂涂确认

---

*最后更新：2026-02-28*
