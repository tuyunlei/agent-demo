# agent-server/AGENTS.md

## What this is
- Treat this crate as the process entrypoint and composition root for the backend server.
- Build and wire concrete dependencies here; keep business flow logic out of this crate.
- Keep top-level startup in `main()` and runtime wiring in `run_server()`.
- Keep environment parsing in `ServerConfig::from_env()` and related helpers.
- For global architecture/security constraints, see `server/AGENTS.md`.

## Layer
- Keep this crate in Layer 1 (Channel / composition root).
- Assemble Layer 4 adapters in `build_stores()` (`PostgresUserStore`, `PostgresMessageStore`, `PostgresEventStore`).
- Assemble Layer 3 capabilities in `build_capabilities()` (`OpenAiProvider`, `DefaultToolRuntime`, `NoopCompactionService`).
- Assemble Layer 2 orchestrator services in `run_server()` (`TurnExecutor`, `AuthService`).
- Expose protocol handlers only through `serve_handlers()` (`ChatServiceHandler`, `SessionServiceHandler`, `AuthServiceHandler`).

## Dependencies
- Depend on `agent_channel` for handlers and `auth_interceptor()` only.
- Depend on `agent_orchestrator` for orchestration entrypoints (`ChatRuntime`, `TurnExecutor`, `AuthService`, `TurnExecutorConfig`).
- Depend on `agent_llm` via `LlmProvider`; default to `OpenAiProvider::with_config(...)`.
- Depend on `agent_tools` via `ToolRuntime`; register built-ins in `build_tool_runtime()`.
- Depend on `agent_storage::pg` for Postgres adapters and run SQLx migrations in this crate.
- Depend on `agent_proto` service servers for gRPC service registration.

## Key constraints
- Do all DI in this crate; don’t instantiate infra/capability implementations inside channel handlers.
- Keep handler creation centralized in `serve_handlers()`; don’t duplicate interceptor wiring.
- Keep `TurnExecutor::new(...)` assembly in one place and pass chat runtime as `Arc<dyn ChatRuntime>`.
- Initialize admin bootstrap through `ensure_admin_user_from_env()` only; don’t add hidden bootstrap paths.
- Resolve DB URL via `ServerConfig::resolve_database_url()`; prefer `DATABASE_URL`, otherwise compose from `PG_*`.
- Keep URL credential safety by using `build_database_url()` + `percent_encode()`.
- Validate required environment variables with `required_env()`; don’t silently default secrets.
- Register built-in tools explicitly (`GetCurrentTimeTool`, `WebSearchTool::from_env()`).
- Keep `ServerBuilder` test-focused and injectable; use it for dependency-substituted server tests.

## Notes
- Follow code reality over historical design text.
- The design doc’s Appendix F.1 mentions readiness/health endpoints and limiter wiring, but current code does not implement them; do not document them as current behavior.
- Keep `main.rs` minimal: load config, log listen address, call `run_server(config)`.
- Add new wiring points by extending existing assembly functions before creating new startup paths.
