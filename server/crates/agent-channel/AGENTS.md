# agent-channel — Contributor Guide

## What this is
- Treat this crate as the Channel-layer entry adapter for external protocols.
- Expose protocol handlers from `src/grpc` via `src/lib.rs`.
- Keep `lib.rs` as a narrow public surface that re-exports `AuthServiceHandler`, `ChatServiceHandler`, `HealthServiceHandler`, `SessionServiceHandler`, `UserId`, and `auth_interceptor`.
- Keep non-gRPC channel experiments isolated in `channel.rs` (currently `PlaceholderChannel`).

## Layer
- Follow Channel-layer responsibilities only; see `server/AGENTS.md` for global architecture constraints.
- Do protocol translation, auth extraction, and boundary validation here.
- Delegate business execution to orchestration/capability interfaces instead of implementing flow logic here.
- Keep gRPC as one handler implementation, not a privileged architecture layer.

## Dependencies
- Depend only on abstractions/use-cases that preserve Channel → Orchestration boundaries.
- Use `ChatRuntime` (not concrete `TurnExecutor`) as the chat execution dependency.
- Re-export handlers through `lib.rs` instead of leaking internal module layout.
- Keep `grpc/mod.rs` as the module wiring point (`auth_handler`, `chat_handler`, `session_handler`, `auth_interceptor`, `error`).
- Keep crate APIs small and explicit; remove dead exports quickly.

## Key constraints
- Do keep handlers thin: map transport DTOs in/out and stop there.
- Do use `auth_interceptor` + `UserId` to carry authenticated identity at transport boundaries.
- Do centralize transport-level error mapping in gRPC error helpers (`grpc/error.rs`).
- Don’t put DB/LLM/tool execution logic in handlers.
- Don’t bypass orchestrator-facing interfaces from Channel code.
- Don’t treat historical ADR text as source of truth when code differs; prefer current code behavior.
- Don’t duplicate root-level rules; apply `server/AGENTS.md` for repo-wide standards.

## Notes
- Keep `channel.rs` intentionally minimal until a concrete non-gRPC channel is introduced.
- Add new protocol handlers by mirroring the existing gRPC export pattern in `lib.rs`.
- Add tests next to handlers (as done in `src/grpc/*_tests.rs`) when extending handler behavior.
- Keep naming aligned with existing handler types (`AuthServiceHandler`, `ChatServiceHandler`, `SessionServiceHandler`).
- Keep authentication plumbing explicit and injectable; avoid hidden global state.
- Keep protocol evolution backward-aware, but let current implementation define operational semantics.
