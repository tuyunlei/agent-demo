# agent-context / AGENTS.md

## What this is
- Treat this crate as the ContextBuilder capability for Layer 3 (Capability).
- Build LLM-ready context from session data without leaking orchestration or infrastructure concerns.
- Keep the core split explicit: sections define prompt fragments, composer assembles them, builder emits final output.

## Layer
- Keep this crate inside the Capability layer contract from `server/AGENTS.md`.
- Depend on domain data types (for example event envelopes), not concrete storage or channel adapters.
- Expose stable interfaces to orchestration: let callers provide input; do not fetch external state here.

## Key types
- Use `ContextBuilder` trait as the crate’s primary contract.
- Use `DefaultContextBuilder` as the default implementation.
- Use `PromptSection` trait as the extension point for system prompt content.
- Use `SystemPromptComposer` (re-exported as `PromptComposer` conceptually) to order and concatenate enabled sections.
- Use `ContextInput` as full build input.
- Use `ContextOutput` as final build output (`system_prompt`, `messages`, `diagnostics`).
- Use `ContextError` for all build failures and validation errors.
- Use `ModelMessage`, `TokenBudget`, `HistoryPolicy`, and `ContextDiagnostics` to keep policies explicit.

## How ContextBuilder works
- Construct a `DefaultContextBuilder` with either explicit sections (`new`) or defaults (`with_default_sections`).
- Call `build_system_prompt(&PromptSectionContext)` to render section-based prompt text.
- Let `SystemPromptComposer` filter by `enabled(ctx)`, sort by `order()`, call `build(ctx)`, and skip empty text.
- Call `build_messages(&ContextInput)` to map selected history into `Vec<ModelMessage>`.
- Call `build(&ContextInput)` when you need both prompt and messages in one pass.
- Return `ContextOutput` only; never return ad-hoc tuples from public APIs.

## Constraints
- Keep `PromptSection` implementations pure and fast; avoid I/O and side effects.
- Keep ordering deterministic; `order()` values must produce stable output.
- Keep dynamic prompt fragments minimal to preserve cacheability (`is_stable()` semantics).
- Keep `build_messages` policy-driven; do not embed hidden heuristics.
- Preserve error boundaries: wrap section failures using `ContextError` variants.
- Add tests for every behavior change in prompt composition or message selection.

## Notes
- Treat `PromptSection` as the long-term extension seam for tenant/domain customization.
- Treat `SystemPromptComposer` as the only assembler for section output ordering.
- Treat `ContextBuilder` as the only producer of externally consumed `ContextOutput`.
- When adding features (token estimation, compaction-aware history, cache hooks), extend existing types before adding parallel abstractions.
- Keep public names and docs aligned with the current exports in `src/lib.rs`.
