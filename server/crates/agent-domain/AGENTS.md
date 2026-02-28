# agent-domain AGENTS.md

## What this is
- Treat this crate as the stable domain contract for the whole server.
- Keep this crate focused on shared language: events, sessions, tokens, and ports.
- Use this crate to define *what* the system means, not *how* it is implemented.
- Assume all other crates depend on these types; change here only with explicit cross-crate impact review.

## Layer
- Keep `agent-domain` in the Capability layer as pure domain/port definitions.
- Define canonical data contracts in `events.rs`, `session.rs`, `token.rs`, and `ports.rs`.
- Re-export public API from `lib.rs` so downstream crates consume one stable surface.
- Avoid embedding infrastructure details, protocol DTO specifics, or orchestration flow code here.

## Key types
- Use `EventEnvelope { meta: EventMeta, payload: EventPayload }` as the canonical event container.
- Preserve `EventMeta` fields (`event_id`, `tenant_id`, `user_id`, `agent_id`, `session_id`, `sequence_number`, `timestamp_ms`, `causation_id`, `correlation_id`, `producer`, `schema_version`) as stable cross-layer contract.
- Keep `EventPayload` variants aligned with the 8 event types: `UserMessage`, `AssistantMessage`, `ToolCallRequest`, `ToolCallResult`, `SystemEvent`, `ConfigChange`, `CompactionMarker`, `Summary`.
- Keep event payload structs explicit (`UserMessageEvent`, `AssistantMessageEvent`, `ToolCallRequestEvent`, `ToolCallResultEvent`, `SystemEvent`, `ConfigChangeEvent`, `CompactionMarkerEvent`, `SummaryEvent`).
- Keep session state in `Session` + `SessionStatus` (`Active`, `Compacting`, `Archived`) and preserve sequence/token tracking fields.
- Keep auth/token shape stable through `TokenPair` and auth result/error types.

## Port traits
- Use `AuthPort` for user auth boundary; keep `AuthResult`/`AuthError` semantics explicit.
- Use `LlmProvider` for model generation boundary with `LlmRequest`, `LlmResponse`, `LlmUsage`, `LlmError`, and `FinishReason`.
- Use `ToolRuntime` for tool listing/execution via `ToolSpec`, `ToolCall`, and `ToolResult`.
- Use `EventStore` as the primary event-stream persistence contract (`append_events`, `read_events`, `read_recent_events`, `get_session`, `create_session`, `list_sessions`).
- Keep `NewEvent`, `AppendResult`, `EventRange`, `CreateSessionParams`, and `SessionListFilter` consistent with `EventStore` semantics.
- Keep `MessageStore` for compatibility/read-model scenarios without promoting it over `EventStore` as source-of-truth.

## Stability constraints
- Treat all public structs/enums/traits in this crate as versioned contracts, not local implementation details.
- Do not rename/remove fields or enum variants lightly; prefer additive changes for backward compatibility.
- Keep event streams append-only: write new events, do not mutate historical events in place.
- Preserve per-session monotonic sequencing assumptions (`sequence_number`, `last_sequence`, `event_count`).
- Keep compaction semantics event-based: represent replacement via `SummaryEvent` + `CompactionMarkerEvent`, not history rewrite.
- Keep serde compatibility in mind for persisted/replayed events and cross-crate boundaries.

## Notes
- Prefer introducing new event variants/fields over overloading existing payload meaning.
- Keep tests in `events.rs`, `session.rs`, `token.rs`, and `ports_tests.rs` updated when contracts evolve.
- Document any domain-surface change with corresponding ADR/design updates before broad refactors.
- If a change can break orchestrator, storage, channel, or protocol mapping, stop and align architecture owners first.
