# agent-demo — AI Agent Platform

Multi-tenant AI agent platform (SaaS) for emotional companionship and life assistance for general users.

## Project Structure

```
agent-demo/
├── scripts/flow              # Task lifecycle CLI (operations + metrics + merge gates)
├── proto/                    # gRPC Proto definitions
├── server/                   # Rust server (Cargo workspace)
│   ├── crates/               # Hexagonal architecture
│   ├── crates/agent-e2e/tests/arch.rs  # Architecture gate tests
│   └── KNOWN_ISSUES.md       # Known architecture issues
├── mobile/                   # iOS client (Swift + SwiftUI)
├── deploy/                   # Deployment config
└── .openclaw/metrics/        # Task state + event log (gitignored)
```

## Git

- Branch: `feature/<TASK_ID>-<description>` → `develop` (PR, merge commit, no squash) → `main` (manual)
- **No direct commits to develop or main** — always use PRs
- Delete feature branch after PR merge
- Pre-push hook: `git config core.hooksPath .githooks` (fmt + clippy + test)

## Environment

- Git remote and SSH keys are configured — `git push` works directly
- `gh` CLI is authenticated — `gh pr create`, `gh pr merge` work directly
- Rust toolchain: `~/.cargo/bin/` must be in PATH
- Commit identity is pre-configured in git

## Quality Standards

- **CI red = cannot merge**, no exceptions
- New features must include corresponding tests
- Code must comply with architecture constraints (dependencies flow inward)
- Function limits: 50 lines max, cognitive complexity ≤ 25
- No `unwrap()` in production code

## Flow CLI

All task lifecycle operations go through `./scripts/flow`:

```bash
./scripts/flow init <TASK_ID> "<description>"       # Create task (orchestrator)
./scripts/flow branch <TASK_ID> [short-desc]         # Create branch + record start (developer)
./scripts/flow pr <TASK_ID> [title]                  # Push + open PR + record (developer)
./scripts/flow verdict <TASK_ID> <PASS|FAIL>         # Record review (reviewer)
./scripts/flow ci <TASK_ID> <green|red>              # Record CI (automated)
./scripts/flow merge <TASK_ID>                       # Gate check → merge (orchestrator)
./scripts/flow status [TASK_ID]                      # View progress
./scripts/flow check <TASK_ID>                       # Verify checkpoints (CI gate)
```

**Merge is gated**: `flow merge` refuses unless review=PASS and CI=green.
**Every command leaves a trace** — success and failure both recorded in events.jsonl.

## Constraints

⚠️ **Repository is public** — no IP addresses, passwords, API keys, or sensitive information.

## References

| What | Where |
|------|-------|
| Server roadmap | server/ROADMAP.md |
| Known issues | server/KNOWN_ISSUES.md |
| Task state | .openclaw/metrics/tasks/*.json |
| Event log | .openclaw/metrics/events.jsonl |
