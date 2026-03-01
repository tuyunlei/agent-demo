# server/ — Architecture Constraints

This file is an architecture guide for all developers (including sub-agents) working in the `server/` directory.
Read this file before modifying code. If there is an AGENTS.md in a subdirectory, read that too.

⚠️ **Also check `KNOWN_ISSUES.md` before modifying code** — Records known architecture issues, avoid building on top of problematic foundations.

## Task Management

- `tasks/QUEUE.md` — Current task queue, execute in order
- `tasks/<task-id>/brief.md` — Detailed description of each task
- Archive to `tasks/done/` after completion

---

## Project Overview

Multi-tenant AI Agent platform backend. Rust monolith, trait boundaries ensure future split-ability.

## Four-Layer Architecture

```
┌─ Channel (Access Layer)───────────────────────────┐
│  Protocol conversion · Auth · DTO ↔ Domain mapping │
│  crates: agent-server, agent-channel               │
└────────────────────────┬───────────────────────────┘
                         ▼
┌─ Orchestration (Orchestration Layer)──────────────┐
│  TurnExecutor · SessionLifecycle · turn state machine  │
│  crates: agent-orchestrator                         │
└────────────────────────┬───────────────────────────┘
                         ▼
┌─ Capability (Capability Layer)────────────────────┐
│  Business abstraction trait + default implementation │
│  crates: agent-domain, agent-context, agent-llm,    │
│          agent-tools, agent-memory                   │
└────────────────────────▲───────────────────────────┘
                         │ implements traits
┌─ Infrastructure (Infrastructure Layer)──────────────┐
│  External system adapters, implement Capability port trait │
│  crates: agent-storage                              │
└────────────────────────────────────────────────────┘

Shared: agent-proto (protocol generated code), agent-e2e (tests)
```

## Dependency Rules (Hard constraints, enforced by CI architecture tests)

**Allowed:**
- Channel → Orchestration → Capability
- Infrastructure → Capability

**Prohibited:**
- Orchestration → Infrastructure (Orchestration layer cannot depend on concrete implementations)
- Channel → Capability or Channel → Infrastructure (Access layer cannot bypass orchestration layer)
- Capability → Channel / Orchestration / Infrastructure (Capability layer cannot have reverse dependencies)
- Infrastructure → Channel / Orchestration (Infrastructure is not aware of upper layers)

Violating dependency direction = CI red = cannot merge.

## Crate Summary Table

| Crate | Layer | One-sentence responsibility |
|---|---|---|
| agent-server | Channel | Process entry + Composition Root (DI is here) |
| agent-channel | Channel | Protocol handler + auth + DTO conversion |
| agent-orchestrator | Orchestration | TurnExecutor + AuthService + turn flow orchestration |
| agent-domain | Capability | Domain models: Event, Session, User, ports(trait) |
| agent-context | Capability | ContextBuilder + PromptSection composition |
| agent-llm | Capability | LlmProvider trait + provider adapter |
| agent-tools | Capability | Tool trait + ToolRuntime + built-in tools |
| agent-memory | Capability | CompactionService trait + strategy |
| agent-storage | Infrastructure | PostgreSQL adapter, implements Capability port trait |
| agent-proto | Shared | protobuf generated code |
| agent-e2e | Test | End-to-end + architecture dependency tests |

## Key Design Decisions (ADR Summary)

Below is a summary of key design decisions.

1. **Monolithic deployment** (ADR-001) — One binary, trait boundaries preserve split-ability
2. **PostgreSQL only storage** (ADR-002) — Structured + JSONB + future pgvector; all queries must include user_id
3. **Protocol is implementation detail** (ADR-003) — Don't abstract transport layer, abstract business layer; add a handler for new protocol
4. **Append-only event stream** (ADR-004/007) — Event stream is source of truth, message list is projection
5. **TurnExecutor + ContextBuilder separation** (ADR-005) — Orchestration is orchestration, context is context
6. **Access layer only does protocol conversion** (ADR-006) — Handler cannot directly connect to DB/LLM/tools
7. **PromptSection pluggable** (ADR-008) — system prompt = section composition, not big string template
8. **Provider capability unified interface** (ADR-009) — stateful/builtin tools/compaction modeled through optional fields and metadata, no trait bloat

## Event Types (8 types)

UserMessage · AssistantMessage · ToolCallRequest · ToolCallResult · SystemEvent · ConfigChange · CompactionMarker · Summary

Events are append-only writes, immutable. Compaction is expressed through Summary + CompactionMarker, history is not rewritten.

## Quality Gates

- `cargo fmt` + `cargo clippy -- -D warnings` + `cargo test` (pre-push hook)
- Coverage ≥ 85% (CI tarpaulin)
- Function ≤ 30 lines, cognitive complexity ≤ 10
- deny: `cast_possible_truncation`, `cast_sign_loss`, `unwrap_used`, `too_many_lines`, `cognitive_complexity`
- warn: `cast_lossless`, `must_use_candidate`
- Production code prohibits `unwrap()`/`expect()` for operations that may fail
- New features must have corresponding tests

## Security Constraints

- JWT secret + passwords = environment variables, hardcoding prohibited
- Repository is public — committing IP, passwords, API keys, internal domain names is prohibited
- Multi-tenant queries must include tenant/user dimension filtering

## AGENTS.md Maintenance

Each crate and key subdirectory has its own AGENTS.md. This is living memory, not a one-time document.

**Read**: Read the AGENTS.md of the corresponding directory before modifying code.

**Write**: After modifying code, if you encounter the following situations, update the corresponding AGENTS.md:
- Added, deleted, or renamed public interfaces
- Stepped on pitfalls or discovered non-obvious constraints
- Made design decisions (why choose A over B)
- Fixed a bug where the root cause involves architectural understanding

Not sure whether to record it? Record it. Better to record an extra entry and delete it later than to miss something important.

**Which level to update?**
- Modified a file → Update the AGENTS.md in that file's directory (if it exists), otherwise look upward for the nearest one
- Issue found affects the entire crate → Update AGENTS.md at the crate root directory
- Issue found affects cross-crate architectural constraints → Update `server/AGENTS.md`
