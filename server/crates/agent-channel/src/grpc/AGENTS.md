# grpc/ — Contributor Guide

## What this is
- Treat this directory as the concrete gRPC adapter for `agent-channel`.
- Keep module wiring in `mod.rs`; export only `AuthServiceHandler`, `ChatServiceHandler`, `HealthServiceHandler`, `SessionServiceHandler`, `UserId`, and `auth_interceptor`.
- Implement transport-facing `tonic` service traits here (`auth_service_server::AuthService`, `chat_service_server::ChatService`, `session_service_server::SessionService`).
- Keep handlers focused on request extraction, DTO mapping, and delegation.

## Key constraints
- Construct handlers with explicit dependencies (`AuthServiceHandler::new`, `ChatServiceHandler::new`, `SessionServiceHandler::new`).
- Do extract authenticated identity from request extensions via `UserId` (or reject with `Status::unauthenticated` where required).
- Do parse/validate transport payloads early (`extract_text`, `non_empty`, `normalize_page_size`, trimmed `session_id`).
- Do keep defaulting behavior intentional and visible (for example, `send_message` currently falls back to `"unknown"` user id).
- Convert domain/orchestrator errors only through `into_status` and `IntoGrpcStatus` implementations in `error.rs`.
- Preserve current unimplemented RPC behavior (`logout`, `subscribe`, `submit_tool_result`) unless you are explicitly delivering those endpoints.
- Keep timestamp conversion consistent via local helpers like `to_timestamp`.

## How to add a new handler
- Add `<name>_handler.rs` and `<name>_handler_tests.rs` in this directory.
- Declare the module in `mod.rs` and re-export the handler type there.
- Implement the generated gRPC trait for your handler type and accept `tonic::Request<...>` / return `tonic::Response<...>`.
- Read `UserId` from `request.extensions()` if the RPC is authenticated.
- Map incoming proto messages into orchestrator/store inputs; keep mapping code local and explicit.
- Return proto DTOs by dedicated mapping helpers (follow `to_proto_session` and `to_proto_message` patterns).
- Keep helper functions small and deterministic so they are unit-testable without a running server.

## Error handling
- Add new error mappings by implementing `IntoGrpcStatus` for the error type in `error.rs`.
- Map client-caused issues to precise status codes (`invalid_argument`, `unauthenticated`, `not_found`, `already_exists`).
- Hide sensitive backend details for infrastructure/provider failures (follow `TurnError::LlmError(_) -> "AI service error"`).
- Use stable, actionable status messages; tests should assert both code and message for critical mappings.

## Testing
- Keep tests colocated via `#[cfg(test)] #[path = "..._tests.rs"]` in each handler module.
- Mock ports/interfaces directly (`AuthPort`, `MessageStore`) to isolate handler behavior.
- Assert both successful payload shape and failure status codes.
- Cover boundary validation paths: empty content, missing/invalid auth header, blank `session_id`, not-found sessions.
- Verify interceptor behavior end-to-end: `auth_interceptor` must inject `UserId` for valid Bearer tokens.
- Add focused tests when changing mapping helpers or pagination normalization to avoid regressions.
