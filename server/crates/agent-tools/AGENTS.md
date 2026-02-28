# agent-tools — Tool Capability Guide

## What this is
- Treat this crate as the Layer 3 capability boundary for tool execution.
- Keep tool contracts stable for orchestrator and provider integration.
- Expose one unified model for built-in and non-built-in tools.

## Layer
- Stay inside Capability-layer responsibilities.
- Define interfaces and default behavior; do not couple to Channel or Infrastructure concerns.
- Let upper layers orchestrate turn loops and event persistence.
- Let lower layers provide external clients behind concrete tool implementations.

## Key abstractions
- Implement `Tool` for each executable tool unit.
- Return tool metadata via `Tool::spec() -> ToolSpec`.
- Execute one call via `Tool::execute(input: ToolInput) -> Result<ToolOutput, ToolError>`.
- Model call metadata with `ToolInput` (`request_id`, `tool_name`, `arguments_json`, `timeout_ms`).
- Model successful payload with `ToolOutput { content_json }`.
- Use `ToolSpec` as the LLM-visible contract (`name`, `description`, `parameters_schema`, `strict`, `execution_class`).
- Use `ToolCall` for provider-emitted call intent (`id`, `name`, `arguments`).
- Use `ToolResult` for normalized result payload (`request_id`, `output_json`, `error_message`).
- Keep `ToolError` variants explicit (`InvalidArguments`, `ExecutionFailed`, `Timeout`, `NotFound`).

## How `ToolRuntime` works
- Depend on `ToolRuntime` trait, not concrete runtime internals.
- Register tools with `register(Box<dyn Tool>)`.
- Discover available tools with `list_specs()`.
- Execute a batch with `execute_calls(Vec<ToolInput>) -> Vec<ToolCallResult>`.
- Preserve input order when returning `ToolCallResult` values.
- Isolate failures per call; do not let one tool failure break batch execution.
- Use `DefaultToolRuntime` as the baseline registry implementation (`HashMap<String, Arc<dyn Tool>>`).
- Use `DefaultToolRuntime::execute_call` for single-call convenience and timeout wrapping.
- Return `ToolError::NotFound` for unknown tool names instead of panicking.
- Wrap timeout using `tokio::time::timeout` and return `ToolError::Timeout`.

## Constraints
- Keep `Tool` implementations `Send + Sync`.
- Keep argument parsing explicit and validate early.
- Return machine-readable JSON strings in `ToolOutput.content_json`.
- Convert runtime and business failures into typed errors, not crashes.
- Preserve compatibility with `agent-llm` tool schema semantics.
- Add unit tests for registration, batch execution, and error paths.

## Notes
- Keep ADR-009 semantics: `ExecutionClass` (`Local`, `Remote`, `ProviderBuiltin`, `Client`) is metadata.
- Do not use `ExecutionClass` as control-flow branching inside runtime execution.
- Keep runtime flow uniform; attach execution-class meaning to policy, audit, or observability.
- Prefer extending contracts incrementally; avoid parallel type systems for tool metadata.
