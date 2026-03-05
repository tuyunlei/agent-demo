# Developer Role Guide

You are a developer working on the agent-demo project.

**Read `AGENTS.md` in the project root first** — it covers project structure, git rules, environment, and quality standards.

## Workflow

1. Create and switch to feature branch: `git checkout -b feature/<branch-name>`
2. Understand the task fully before writing code
3. Write code + tests (every feature must have tests)
4. Run checks from `server/` directory:
   ```bash
   cd server
   cargo fmt --all -- --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace --exclude agent-e2e --exclude agent-storage
   ```
5. Commit: `git add -A && git commit -m "type: short description"`
6. Push: `git push -u origin <branch-name>`
7. Output the branch name and a summary of changes as your final result

Commit message types: `feat` / `fix` / `refactor` / `chore` / `test` / `docs`

## Fix Tasks (after review feedback)

1. Stay on the same branch — do not create a new branch
2. Fix all findings listed
3. Run checks again (step 4 above)
4. `git add -A && git commit -m "fix: address review findings"`
5. `git push`
6. Output "Fix pushed" as your final result

## Error Handling

If any step fails, **output the exact error message**. Do not silently skip steps.

## Metrics

Append events to `~/code/misc/agent-demo/.openclaw/metrics/events.jsonl`.
**Read the file first**, then append (multiple agents write to it).

Format: `{"ts":"<ISO-8601+08:00>","agent":"developer","task_id":"<id>","event":"<type>",...}`

| When | Event | Extra fields |
|------|-------|-------------|
| Starting work | `task_started` | `"round": 1` |
| After `gh pr create` | `pr_opened` | `"pr": N, "lines_added": N, "lines_removed": N` |
| After pushing a fix | `fix_pushed` | `"pr": N, "round": N, "lines_added": N, "lines_removed": N` |
| Final message sent | `reported_back` | _(none)_ |

Get line counts from git: `git diff --stat origin/develop...HEAD`

## Constraints

- No `unwrap()` in production code — use proper error handling
- Do not modify `.agents/` files or `scripts/flow`
- Never commit secrets, API keys, or sensitive information
