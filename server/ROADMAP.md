# server ROADMAP

## Completed

<details>
<summary>Refactoring Phase R-01 ~ R-09 (PRs #5-14)</summary>

- [x] **R-01: agent-domain refactoring + agent-types merge** (PR #5)
- [x] **R-02: agent-context crate** (PR #6)
- [x] **R-03: agent-tools crate** (PR #7)
- [x] **R-04: agent-llm refactoring** (PR #8)
- [x] **R-05: agent-memory crate** (PR #9)
- [x] **R-06a: agent-orchestrator rename** (PR #10)
- [x] **R-06b: TurnExecutor implementation** (PR #11)
- [x] **R-07: agent-storage EventStore** (PR #12)
- [x] **R-08: agent-channel + agent-server refactoring** (PR #13)
- [x] **R-09: arch test update + coverage fix** (PR #14)
- [x] **Cleanup obsolete files** (PR #15)
</details>

<details>
<summary>Test Coverage T-01 ~ T-05 (PRs #16-17)</summary>

- [x] **T-01: agent-llm test completion** (PR #16)
- [x] **T-02+T-03+T-04: web_search + ports + miscellaneous coverage** (PR #17)
- [x] **T-05: CI coverage threshold increase** — 65% → 77% → 85%
</details>

<details>
<summary>Other Completed Items</summary>

- [x] **Docker Compose deployment** (PR #18) — preview + production dual environments
- [x] **Provider Capabilities** (PR #19) — stateful API, builtin tools, compaction layering, ADR-009
</details>

---

## Completed: Lint Strengthening (L-01~L-05, PRs #20-22)

<details>
<summary>Expand to view</summary>

- [x] L-01: lint configuration skeleton
- [x] L-02: cast audit (cast_possible_truncation, cast_sign_loss, cast_lossless)
- [x] L-03: unwrap ban (deny unwrap_used)
- [x] L-04: function size control (≤30 lines, complexity ≤10)
- [x] L-05: must_use audit
</details>

---

## TODO

See `tasks/QUEUE.md` for details.

### Test Coverage Improvement (Target 90%)

Current coverage: 85.96% (CI threshold 85%). Long-term target 90%.
