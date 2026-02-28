# builtin tools — Implementation Guide

## What this is
- Treat this directory as the built-in tool catalog for `agent-tools`.
- Implement lightweight, reusable tools that ship with the server.
- Keep built-ins composable through the shared `Tool` trait.

## How to add a new built-in tool
- Create a new file under `src/builtin/` (for example, `my_tool.rs`).
- Define a tool struct with only required dependencies.
- Implement `Tool` for the struct.
- Return a complete `ToolSpec` in `spec()`:
  - set a stable `name`
  - write a precise `description`
  - define `parameters_schema` for LLM argument generation
  - set `strict` intentionally
  - set `execution_class` metadata (`Local` unless justified otherwise)
- Parse `ToolInput.arguments_json` with `serde` types.
- Validate arguments and return `ToolError::InvalidArguments` for bad input.
- Execute business logic without panicking.
- Return JSON text in `ToolOutput.content_json`.
- Convert external API failures into structured tool output or `ToolError`, based on UX intent.
- Add focused tests for happy path and failure path.
- Register the module in `src/builtin/mod.rs`.
- Re-export the tool type from `mod.rs`.
- Register the tool in runtime bootstrap where `DefaultToolRuntime` is assembled.

## Existing tools
- `GetCurrentTimeTool`
  - Accept optional IANA timezone.
  - Default to UTC when timezone is missing.
  - Return RFC3339 timestamp as JSON.
- `WebSearchTool`
  - Accept `query` and optional `count` (1..=10, default 5).
  - Return friendly JSON when API key is absent.
  - Call Brave Search when configured and format result text payload.

## Constraints
- Keep each built-in self-contained and dependency-light.
- Keep tool names unique across the runtime registry.
- Keep schemas and output shape stable to avoid prompt drift.
- Keep user-facing failure data readable and safe.
- Wrap tool failures for LLM consumption through `ToolOutput` where appropriate.
- Avoid panic-based control flow; surface recoverable failures explicitly.
- Preserve deterministic behavior in unit tests.
- Keep network tools defensive about timeouts and response parsing.
