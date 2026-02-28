# agent-memory/ — Session compaction capability

## What this is

Define the compaction capability boundary for session history.
Expose policy and outcome types that orchestration can use without coupling to storage details.
Keep this crate focused on compaction abstractions, not turn orchestration or SQL.

## Layer

Treat this crate as **Capability layer (Layer 3)**.
Provide traits and default implementations that higher layers compose.
Depend only on domain-level types and generic async/error patterns.
Do not depend on channel, orchestration, or infrastructure crates.

## Key types

Use `CompactionService` as the primary trait (`compact_if_needed`, `policy`).
Use `CompactionPolicy` for thresholds and strategy.
Use `CompactionStrategy` for strategy selection (`KeepRecent`, `LlmSummary`).
Use `CompactionOutcome` for result signaling (`Skipped` or `Compacted`).
Use `CompactionError` for in-progress, storage, and internal failures.

## Current state (MVP noop)

Treat current implementation as an MVP scaffold.
Use `NoopCompactionService` as the default implementation.
Expect `compact_if_needed` to always return `Skipped("noop compaction service")`.
Rely on `CompactionPolicy::default()` for baseline values:
- `event_count_threshold = 100`
- `keep_recent_events = 50`
- `strategy = KeepRecent`

## Future direction

Implement real compaction behind `CompactionService` without changing callers.
Follow ADR-009 guidance: support provider-native compaction and self-managed compaction.
Preserve event-stream semantics (summary + marker) when integrated with lifecycle flows.
Keep policy-driven triggering and deterministic outcomes for testability.

## Constraints

Keep this crate implementation-agnostic.
Do not embed SQL, transport DTOs, or channel-specific behavior.
Do not move lifecycle state-machine logic into this crate.
Keep public types explicit and stable for downstream crates.
Add tests for policy defaults and noop guarantees when interfaces change.
