# Known Issues

Read this before modifying code to avoid building on top of problematic foundations.

---

## 1. agent-context is a dead crate (Resolved 2026-03-04)

**Status**: Resolved

`agent-orchestrator` now depends on `agent-context`, `TurnExecutor` receives an injected `ContextBuilder`, and system prompt composition is built through `ContextBuilder` + `PromptSection` implementations.

---

## 2. agent-domain shadow trait (Resolved in PR #27)

**Status**: Resolved in PR #27

agent-domain and agent-llm each define a set of traits with overlapping concepts:

| Concept | agent-domain | agent-llm |
|---------|-------------|-----------|
| LLM call | `LlmProvider::generate` | `LlmProvider::complete` |
| Tool execution | `ToolRuntime` | `ToolRuntime` (agent-tools) |
| Tool definition | `ToolSpec`, `ToolCall` | `ToolSpec`, `ToolCall` (agent-tools) |

`OpenAiProvider` implements both `LlmProvider` traits simultaneously, with a `provider_compat` conversion layer bridging them.

**Impact**:
- New developers don't know which trait to use
- Conversion layer is pure glue code that shouldn't exist
- agent-domain's version is a "shadow API" — implemented but not actually driving the system

**Fix direction**: Unify to one definition, eliminate the provider_compat conversion layer.

---

## 3. agent-channel fake dev-dependencies

**Severity**: Medium — Test dependency leakage

agent-channel's `Cargo.toml` dev-dependencies include `agent-llm`, `agent-tools`, `agent-memory`.

Channel layer (outermost layer of hexagon) tests should not directly depend on concrete adapter crates. This means tests are not isolated through trait boundaries, but directly construct inner layer concrete implementations.

**Impact**:
- Violates hexagonal architecture dependency rules (outer layer should not depend on inner layer concrete implementations)
- Tests are coupled with concrete implementations, changing adapters requires changing tests
- Masks whether trait interfaces are truly mockable

**Fix direction**: Channel tests should use mock implementations of traits, not reference concrete adapters. If mocking is difficult, the trait design has issues, fix the trait first.

---

## 4. TurnExecutor constructor exposes too many internal dependencies (Resolved 2026-03-06)

**Status**: Resolved

`TurnExecutor::new` now takes a single `TurnExecutorDeps` struct bundling `llm`, `tools`,
`message_store`, `compaction`, `context_builder`, and `config`. This removes internal dependency
leakage from the constructor signature and simplifies call sites.

---

## 5. system prompt hardcoded in turn_compat (Resolved 2026-03-04)

**Status**: Resolved

Hardcoded `turn_compat.rs::build_system_prompt` was removed. Prompt content now lives in concrete `PromptSection` implementations and is composed by `ContextBuilder`.

---

## 6. AgentError dead code (Resolved in PR #27)

**Status**: Resolved in PR #27

`AgentError` is defined in `agent-domain/src/ports.rs`, exported through `lib.rs`, but no external crate references it. Only tested in `ports_tests.rs`.

**Fix direction**: Confirm if there are plans to use it. If not, delete.

---

*Last updated: 2026-03-06*
