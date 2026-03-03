# Server Roadmap

## Progress

```
Phase 1: Walking Skeleton        ████████████ DONE  (PRs #1-#4)
Phase 2: Refactoring & Quality   ████████████ DONE  (PRs #5-#22)
Phase 3: Tech Debt Cleanup       ████████████ DONE  (PRs #23-#30)
Phase 4: Core Features           ░░░░░░░░░░░░ ← HERE
Phase 5: MVP Product             ░░░░░░░░░░░░
Phase 6: Deploy & Polish         ░░░░░░░░░░░░
```

**Coverage**: 85.96% (CI threshold 85%, long-term target 90%)

## Current Phase: Core Features

### Remaining Tech Debt (non-blocking)
See [KNOWN_ISSUES.md](KNOWN_ISSUES.md) — 2 open issues (KI-03, KI-04). Fix opportunistically during feature work.

### Feature TODO
| Priority | Feature | Status | Notes |
|----------|---------|--------|-------|
| P0 | Memory / Compaction | Framework exists | CompactionService only has Noop impl |
| P0 | Session management | Basic exists | Need create/restore/list |
| P0 | User Auth | Design done | JWT + password, migration ready |
| P1 | Tool expansion | Framework exists | Only web_search built-in |
| P1 | Provider fallback | Design done | LLM provider fallback chain |
| P2 | Skill system | Research done | Three-layer progressive discovery |

### Blockers
- `develop → main` merge requires 涂涂's confirmation

## Deployment

```
Internet → :443 TLS → Caddy → localhost:50051/50052 h2c → server → postgres
```

- Preview: `preview-agent.xclz.org` → :50051
- Production: `agent.xclz.org` → :50052

## Completed

<details>
<summary>Phase 1: Walking Skeleton (PRs #1-#4)</summary>

- [x] Architecture design (11 design documents)
- [x] 4-layer crate structure (13 crates)
- [x] gRPC service skeleton
- [x] PostgreSQL + sqlx setup
- [x] Basic LLM integration (OpenAI-compatible)
- [x] Docker Compose deployment (PR #18)
- [x] Provider Capabilities (PR #19, ADR-009)
</details>

<details>
<summary>Phase 2: Refactoring & Quality (PRs #5-#22)</summary>

- [x] R-01 ~ R-09: Full code refactoring (PRs #5-#14)
- [x] Cleanup stale files (PR #15)
- [x] T-01 ~ T-05: Test coverage 65% → 85.96% (PRs #16-#17)
- [x] L-01 ~ L-05: Lint hardening (PRs #20-#22)
  - deny: cast_possible_truncation, cast_sign_loss, unwrap_used, too_many_lines, cognitive_complexity
  - Thresholds: function ≤30 lines, complexity ≤10
</details>

<details>
<summary>Phase 3: Tech Debt Cleanup (PRs #23-#30)</summary>

- [x] Health check endpoint (PR #23)
- [x] Remove dead AgentError + shadow ToolRuntime (PR #24)
- [x] Extract ChatRuntime trait (PR #25)
- [x] Unify shadow LLM types — KI-02 (PR #27, +54/-527)
- [x] Flow CLI + role guides — INFRA-01 (PR #28)
- [x] Flow merge gate validation — INFRA-01b (PR #29)
- [x] Integrate ContextBuilder into TurnExecutor — KI-01+KI-05 (PR #30, +320/-201)
</details>
