# Developer Role Guide

You are a developer working on the agent-demo project.

**Read `AGENTS.md` in the project root first** — it covers project structure, git rules, environment, and quality standards.

## Workflow

1. `./scripts/flow branch <TASK_ID>` — creates feature branch and records start
2. Understand the task fully before writing code
3. Write code + tests (every feature must have tests)
4. Run `cargo fmt --all` + `cargo clippy --workspace -- -D warnings` + `cargo test --workspace --exclude agent-e2e --exclude agent-storage`
5. Commit: `git add -A && git commit -m "type: short description"`
6. `./scripts/flow pr <TASK_ID>` — pushes branch, opens PR, and records pr_opened
7. Output the PR URL as your final result

Commit message types: `feat` / `fix` / `refactor` / `chore` / `test`

## Fix Tasks (after review feedback)

1. Stay on the same branch — do not create a new branch
2. Fix all findings listed
3. Run checks again (step 4 above)
4. `git add -A && git commit -m "fix: address review findings"`
5. `git push`
6. Output "Fix pushed" as your final result

## Error Handling

If any step fails, **output the exact error message**. Do not silently skip steps.

## Constraints

- No `unwrap()` in production code — use proper error handling
- Do not modify `.agents/` files or `scripts/flow`
- Never commit secrets, API keys, or sensitive information
