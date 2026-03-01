# Known Issues

Read this before modifying code to avoid building on top of problematic foundations.

---

## 1. agent-context is a dead crate

**Severity**: High — Design and implementation are disconnected

agent-context defines a complete system prompt composition system (`ContextBuilder`, `PromptSection`, `SystemPromptComposer`), including identity, timezone, security, tools, and other sections.

**But no crate depends on it.** Zero external references in `Cargo.toml`.

The actual system prompt build is done through hardcoded string concatenation in the `build_system_prompt` function in `agent-orchestrator/src/turn_compat.rs`, completely bypassing agent-context's design.

**Impact**:
- ~500 lines of carefully designed code are unused
- New developers seeing agent-context will assume system prompt goes through here, but it doesn't
- Design intent (pluggable sections, token budget management) never landed

**Fix direction**: Either make TurnExecutor actually use ContextBuilder, or delete agent-context and do it well in turn_compat. Pick one, cannot have both.

---

## 2. agent-domain shadow trait

**Severity**: High — Architecture consistency issue

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

## 4. TurnExecutor constructor exposes too many internal dependencies

**Severity**: Medium — Interface design issue

`TurnExecutor::new` accepts 5 `Arc<dyn Trait>` parameters: `LlmProvider`, `ToolRuntime`, `MessageStore`, `CompactionService`, `TurnExecutorConfig`.

Callers must know which components TurnExecutor needs internally to construct it, this is implementation detail leakage.

**Impact**:
- Every time a new internal dependency is added, all places constructing TurnExecutor need to change
- Composition Root (agent-server) takes on too much assembly knowledge
- Tests need to construct all dependencies to test a behavior

**Fix direction**: Consider Builder pattern or Context/Config struct to encapsulate dependencies. Or reconsider: does TurnExecutor have too many responsibilities that need to be split.

---

## 5. system prompt hardcoded in turn_compat

**Severity**: Medium — Directly related to Issue 1

The `build_system_prompt` function in `turn_compat.rs` builds system prompt through direct string concatenation, bypassing the entire composition system designed by agent-context.

```
turn_compat.rs::build_system_prompt  ←  Actually used
agent-context::ContextBuilder        ←  Designed but not connected
```

**Impact**:
- system prompt content and format are not configurable
- Cannot dynamically compose different sections by user/scenario
- Adding prompt content requires modifying Rust code, recompiling

**Fix direction**: Solve together with Issue 1. If keeping agent-context, migrate turn_compat's logic there; if deleting agent-context, at least extract hardcoding into configurable templates.

---

## 6. AgentError dead code

**Severity**: Low — Only internal to agent-domain

`AgentError` is defined in `agent-domain/src/ports.rs`, exported through `lib.rs`, but no external crate references it. Only tested in `ports_tests.rs`.

**Fix direction**: Confirm if there are plans to use it. If not, delete.

---

*Last updated: 2026-03-02*
