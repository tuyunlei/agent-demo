# agent-demo/server — 状态

## Phase: idle

Lint 强化全部完成（L-01~L-05 + 阈值收紧）。PRs #20, #21, #22 已 merge。

## 当前进度

- [x] 框架分层架构源码调研
- [x] 新架构设计文档（11 个任务全部完成）
- [x] 代码重构对齐设计（R-01 ~ R-09 全部完成）
- [x] 测试覆盖率提升：65% → 85.96%（CI 阈值 85%）
- [x] Docker Compose 部署（PR #18）
- [x] Provider Capabilities（PR #19）
- [x] **Lint 强化全部完成**（PRs #20, #21, #22）
  - deny: cast_possible_truncation, cast_sign_loss, unwrap_used, too_many_lines, cognitive_complexity
  - warn: cast_lossless, must_use_candidate
  - 阈值：函数 ≤30 行，复杂度 ≤10

## 部署架构

```
Internet → :443 TLS → Caddy (host, shared) → localhost:50051/50052 h2c → server → postgres
```

- Preview: `preview-agent.xclz.org` → localhost:50051
- Production: `agent.xclz.org` → localhost:50052

## 阻塞点

- develop → main merge 需要涂涂确认

---

*最后更新：2026-02-28 22:25 CST*
