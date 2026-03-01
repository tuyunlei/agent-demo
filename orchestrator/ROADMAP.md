# orchestrator — Roadmap

## Architecture Decisions (Confirmed)

- **Roles**: Planner (task breakdown), Developer (write code), Reviewer (review) — No Architect
- **Models**: Planner = Claude Sonnet 4.6, Developer = gpt-5.3-codex, Reviewer = gpt-5.3-codex
- **Communication**: Event-driven, `send_message` function tool + asyncio.Queue message bus
- **Topology**: Planner is central node, Developer/Reviewer don't communicate directly
- **Three layers**: TuTu → XiaoXiaoTu (mechanism layer, manages orchestrator itself) → Planner/Developer/Reviewer (project layer)
- **Language**: Python (LiteLLM works out of the box)
- **Run mode**: Currently one-time script, daemon mode to be considered later

---

## Phase 1 — It Runs

Minimum viable: three agents can collaborate to complete a real task.

| Step | Content | Status |
|------|---------|--------|
| S1 | Environment setup: venv + openai-agents + Codex MCP | ✅ |
| S2 | Verification: API connectivity, Agents SDK, Codex MCP file operations | ✅ |
| S3 | Three role instructions | 🔜 |
| S4 | Message orchestration improvement: from field, loop limit, error handling, Human recovery | |
| S5 | Real task verification: pick a small task from ROADMAP to run full flow | |

### S3 Details

Planner instructions are most complex, need to:
- Read server/ROADMAP.md to understand TODO
- Read server/AGENTS.md to understand architecture
- Read server/KNOWN_ISSUES.md to understand known issues
- Break down tasks to Developer-executable granularity
- Judge whether Developer output needs Reviewer
- Judge whether task is complete, report to Human

Developer instructions:
- Codex MCP operates project files
- Write code + write tests
- After completion send_message to Planner

Reviewer instructions:
- Read-only review (no code changes)
- Check against architecture constraints
- Report issues to Planner

---

## Phase 2 — Usable

Add guardrails to make the process reliable.

| Step | Content |
|------|---------|
| S6 | Guardrails: output validation, sensitive info filtering |
| S7 | Git workflow integration: auto-create feature branches, commit, open PRs |
| S8 | CI checks: wait for CI results after PR push, auto-fix if red |
| S9 | Human recovery mechanism: manual intervention entry when process is stuck |

---

## Phase 3 — Observable

See what's happening.

| Step | Content |
|------|---------|
| S10 | Event logs: all messages, tool calls, agent decisions persisted |
| S11 | Efficiency metrics: task duration, token consumption, round count |
| S12 | Quality metrics: review rejection rate, CI failure rate |
| S13 | Output metrics: file/line change statistics |
| S14 | Dashboard: aggregated display |

---

## Phase 4 — Generalization

Not just for agent-demo.

| Step | Content |
|------|---------|
| S15 | Project configuration decoupling: Planner instructions parameterized, switching projects only changes config |
| S16 | Multi-project support: manage multiple projects simultaneously |

---

*Last updated: 2026-03-02*
