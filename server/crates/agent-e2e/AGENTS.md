# agent-e2e/ — End-to-end and architecture guard tests

## What this is

Validate cross-crate behavior at system level and enforce architecture boundaries.
Use this crate for integration-style verification that unit tests in individual crates do not cover.

Run this suite in CI and explicit workflows.
Do not treat it as always-on local fast test feedback.

## Architecture tests

Use `tests/arch.rs` to enforce dependency direction and maintain layering discipline.
Check `Cargo.toml` agent-* dependencies against forbidden dependency maps.
Fail fast when a crate depends on disallowed layers.

Use the same file to enforce Rust source size heuristics:
- max effective lines for normal files
- relaxed limit for `*_tests.rs`
- ignore generated `.pb.rs`

Keep architecture assertions data-driven and easy to update when layering rules intentionally change.

## E2E tests

Use `tests/e2e.rs` for full request-path verification:
- auth register/login flows
- token validation behavior
- chat roundtrip behavior
- session message retrieval
- LLM error propagation

Back tests with `MockLlmProvider` to make responses deterministic.
Assert both API-visible outputs and internal mock call expectations.

## Test infrastructure

Use `tests/common/mod.rs` `TestEnv` as the shared harness.
Build server instances with `ServerBuilder`.
Inject:
- `PostgresUserStore`
- `PostgresMessageStore`
- `MockLlmProvider`
- test JWT secret

Bind ephemeral localhost port, spawn server with shutdown signal, and provide typed gRPC clients.
Keep startup/shutdown behavior reliable and explicit.

## How to add tests

Add architecture assertions in `arch.rs` when introducing new layering constraints.
Add behavior scenarios in `e2e.rs` for externally observable workflows.
Reuse `TestEnv` helpers instead of duplicating setup code.
Use `#[sqlx::test(migrations = "../agent-storage/migrations")]` for DB-backed scenarios.
Prefer deterministic fixtures and explicit assertions over snapshot-style ambiguity.

Remember this crate is excluded from normal default `cargo test` workflows because it requires database-backed integration setup; keep it CI-ready.
