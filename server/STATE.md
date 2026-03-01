# agent-demo/server — Status

Read this file first every time you wake up.

---

## Phase: idle

---

## Current Progress

**Lint strengthening all completed (L-01~L-05 + threshold tightening). PRs #20, #21, #22 merged.**

## Current Progress

- [x] Framework layered architecture research
- [x] New architecture design documents (all 11 tasks completed)
- [x] Code refactoring aligned with design (R-01 ~ R-09 all completed)
- [x] Test coverage improvement: 65% → 85.96% (CI threshold 85%)
- [x] Docker Compose deployment (PR #18)
- [x] Provider Capabilities (PR #19)
- [x] **Lint strengthening all completed** (PRs #20, #21, #22)
  - deny: cast_possible_truncation, cast_sign_loss, unwrap_used, too_many_lines, cognitive_complexity
  - warn: cast_lossless, must_use_candidate
  - Thresholds: function ≤30 lines, complexity ≤10

## Deployment Architecture

```
Internet → :443 TLS → Caddy (host, shared) → localhost:50051/50052 h2c → server → postgres
```

- Preview: `preview-agent.xclz.org` → localhost:50051
- Production: `agent.xclz.org` → localhost:50052

## Blockers

- develop → main merge requires confirmation from TuTu

---

*Last updated: 2026-02-28 22:25 CST*
