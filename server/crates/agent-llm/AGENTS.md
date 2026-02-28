# agent-llm / AGENTS.md

## What this is
- Treat this crate as the provider-agnostic LLM capability layer for the server.
- Keep orchestration code unaware of vendor-specific APIs, payloads, and error formats.
- Preserve a stable domain-facing contract while allowing provider adapters to evolve.

## Layer
- Keep this crate in Capability layer (Layer 3) terms and boundaries.
- Expose traits and normalized types from `lib.rs`; hide wire/protocol details behind adapters.
- Keep Infrastructure concerns (HTTP schema, status-code mapping, vendor wire structs) inside adapter modules.

## Key abstractions
- Implement and depend on the `LlmProvider` trait as the primary boundary.
- Pass `LlmRequest` in and return `LlmResponse` out; keep both provider-neutral.
- Declare feature support through `LlmProviderCapabilities` instead of branching in callers.
- Use `LlmError` classification (`is_retryable`, `is_fallbackable`) to drive upper-layer policy.
- Reuse `MockLlmProvider` for deterministic tests and contract verification.

## Adapter pattern
- Treat `OpenAiProvider` as one concrete adapter, not the architecture.
- Keep OpenAI-compatible request/response wire structs in `provider_wire` only.
- Keep mapping glue in `provider_compat` when bridging to/from `agent_domain` legacy surfaces.
- Convert vendor payloads into normalized `LlmResponse` fields (`content`, `tool_calls`, `finish_reason`, `usage`, `response_id`).
- Map transport/HTTP/vendor failures into `LlmError` without leaking vendor codes upward.

## How to add a new provider
- Create a new adapter type implementing `LlmProvider` (parallel to `OpenAiProvider`).
- Add provider-specific wire DTOs in a dedicated module; do not pollute shared types.
- Map provider message/tool formats into shared `LlmRequest`/`LlmResponse` consistently.
- Implement `provider_id()` with a stable identifier and fill `capabilities()` honestly.
- Keep `stream()` unsupported unless fully implemented; rely on default unsupported behavior otherwise.
- Add/extend compat mapping only when domain compatibility requires it.
- Add focused unit tests for request mapping, response parsing, finish-reason conversion, and error mapping.

## Error handling
- Classify retry/fallback behavior via `LlmError` variants, not ad-hoc string matching.
- Treat `RateLimit`, `Timeout`, `ProviderDown`, and `Transport` as retry/fallback candidates.
- Treat `AuthError` and `InvalidRequest` as non-retryable contract/config failures.
- Return `UnsupportedCapability` for unimplemented features (for example default `stream()`).

## Notes
- Keep provider-agnostic design as the primary invariant; adapters are replaceable implementations.
- Keep `provider_compat` as migration glue, not the long-term center of crate design.
- Use `MockLlmProvider` to test orchestration-facing behavior without network coupling.
- Note ADR-009 fields already exist in shared request/capability types: `previous_response_id`, `builtin_tools`, `supports_stateful`.
- Treat ADR-009 stateful/builtin-tools behavior as not yet fully wired in current adapter flow.
- Keep test files (`provider_tests.rs` and module tests) aligned with contract changes.
