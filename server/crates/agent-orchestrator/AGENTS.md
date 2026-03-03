# agent-orchestrator — Orchestration Layer Guide

## What this is
- Treat this crate as Layer 2 orchestration for turn execution and auth flow.
- Use `TurnExecutor` to run one complete conversational turn.
- Use `ChatRuntime` as the channel-facing runtime boundary for turn execution.
- Use `AuthService` to handle login/register/refresh token workflows.
- Keep this crate focused on coordination, not implementation details.

## Layer
- Depend only on Capability-layer contracts.
- Keep Channel concerns (transport DTO/protocol details) out of this crate.
- Keep Infrastructure concerns (DB/provider adapters, SQL, SDK wiring) out of this crate.
- Wire concrete implementations only in composition root crates (for example `agent-server`).

## Dependencies (traits only)
- Inject `dyn LlmProvider` into `TurnExecutor`.
- Inject `dyn ToolRuntime` into `TurnExecutor`.
- Inject `dyn MessageStore` into `TurnExecutor`.
- Inject `dyn CompactionService` into `TurnExecutor`.
- Inject `dyn ContextBuilder` into `TurnExecutor`.
- Inject `dyn AuthPort` into `AuthService`.
- Do not import or reference Infrastructure concrete types in orchestration code.

## Key constraints
- Validate input early (`TurnExecutor::validate_input`, `AuthService::register`).
- Convert lower-level errors into orchestration errors (`store_err`, `map_auth_error`).
- Keep turn/tool loops bounded via `TurnExecutorConfig::max_tool_iterations`.
- Preserve deterministic flow in `TurnExecutor::run_turn`: resolve session → persist user input → build context/history → call LLM → handle terminal/tool-call branch.
- Keep compatibility/format conversion inside `turn_compat` helpers.
- Prefer explicit finish semantics via `TurnFinishReason` and `TurnError`.

## TurnExecutor specifics
- Construct with `TurnExecutor::new(...)` using trait objects and `TurnExecutorConfig`.
- Use `resolve_session_id` when `TurnInput.session_id` is missing/blank.
- Always persist user messages before LLM completion (`save_user_message`).
- Build request context with `build_messages_with_history`, including `ContextBuilder::build_system_prompt(...)` and formatted timestamps.
- Build LLM requests via `request_llm_response`; keep request metadata (`session_id`) populated.
- Treat `FinishReason::ToolCalls` as iterative tool workflow; treat `Stop/ContentFilter/Error` as terminal stop; treat `Length` as truncated output.
- Persist assistant tool-call messages with `save_assistant_tool_call_message` before executing tools.
- Execute tools through `ToolRuntime::execute_calls`; convert both success/failure to tool chat messages.
- Return `TurnError::ToolLoopExceeded` when iteration limit is reached.
- Trigger `compaction.compact_if_needed(session_id)` as best effort after terminal assistant persistence.

## AuthService specifics
- Construct with `AuthService::new(auth_port, jwt_secret)`.
- Use `login` only through `AuthPort::authenticate` and issue JWT pair via `create_token_pair`.
- Use `register` only through `AuthPort::create_user` after non-empty email/password checks.
- Use `refresh_token` only with validated refresh tokens (`claims.token_type == "refresh"`).
- Keep token signing/validation in `sign_token` and `validate_token` (HS256).
- Preserve TTL contracts: access token 1 hour, refresh token 7 days.
- Map `AuthError` to `AuthServiceError` with `map_auth_error`.

## Notes
- Re-export only stable orchestration API from `lib.rs` (`ChatRuntime`, `TurnExecutor`, `AuthService`, turn/auth types).
- Keep `turn_compat` as the conversion boundary between domain/llm/tool models.
- Update tests when behavior changes (`turn_executor_tests`, `service_tests`, `service_refresh_tests`, `service_prop_tests`).
- Keep orchestration behavior observable through typed outputs/errors, not hidden side effects.
