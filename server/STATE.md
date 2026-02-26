# agent-demo/server — 状态

## Phase: idle

Docker Compose 部署完成，PR #18 已 merge 到 develop。

## 当前进度

- [x] 框架分层架构源码调研
- [x] 新架构设计文档（11 个任务全部完成）
- [x] 代码重构对齐设计（R-01 ~ R-09 全部完成）
- [x] 测试覆盖率提升：65% → 77.48%
- [x] Docker Compose 部署（PR #18 ✅ merged）
  - [x] Multi-stage Dockerfile（rust:1.88 → debian:bookworm-slim，~36MB）
  - [x] docker-compose.yml（postgres + server，可配置端口）
  - [x] PG_* 独立环境变量 + percent-encoding（Codex review 修复）
  - [x] .dockerignore（排除 server/target/ 等，build context 从 20GB 降到 ~7MB）
  - [x] 空环境变量处理（DATABASE_URL + PG_*）
  - [x] pre-push hook（fmt + clippy + test）
  - [x] Preview 环境运行中：`preview-agent.xclz.org`
  - [x] Production 环境运行中：`agent.xclz.org`
  - [x] TLS 证书自动签发（Let's Encrypt via Caddy）

## 部署架构

```
Internet → :443 TLS → Caddy (host, shared) → localhost:50051/50052 h2c → server → postgres
```

- Preview: `preview-agent.xclz.org` → localhost:50051
- Production: `agent.xclz.org` → localhost:50052
- Caddy 独立于项目，作为共享基础设施运行

## 阻塞点

- develop → main merge 需要涂涂确认

---

*最后更新：2026-02-26*
