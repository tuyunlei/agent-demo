# agent-context/src/sections / AGENTS.md

## What this is
- Treat this directory as the concrete `PromptSection` implementation set.
- Add behavior here when you need to change system prompt content granularity.
- Keep section logic focused on rendering text from `PromptSectionContext`.

## How to add a new section
- Create a new file in this directory (for example `policy.rs`).
- Define a section struct (for example `pub struct PolicySection;`).
- Implement `PromptSection` for that struct.
- Implement `name()` with a stable, unique string.
- Implement `build(&PromptSectionContext)` and return deterministic text.
- Set explicit `order()` and treat it as `SectionOrder` (`u16`) for readability and review.
- Override `is_stable()` only when output changes frequently (time, nonce, per-turn signals).
- Override `enabled()` only for context-dependent gating.
- Export the section in `mod.rs` (`pub mod ...` + `pub use ...`).
- Register the section in `DefaultContextBuilder::with_default_sections()` in `builder.rs`.
- Add tests in `tests.rs` for content, ordering, and stability expectations.

## Existing sections
- `IdentitySection` (`identity.rs`)
  - `order()` = 10
  - `is_stable()` = true (default)
- `SafetySection` (`safety.rs`)
  - `order()` = 20
  - `is_stable()` = true (default)
- `ToolsSection` (`tools.rs`)
  - `order()` = 30
  - `is_stable()` = true (default)
- `RuntimeSection` (`runtime.rs`)
  - `order()` = 90
  - `is_stable()` = true (default)
- `DateTimeSection` (`datetime.rs`)
  - `order()` = 100
  - `is_stable()` = false (explicit override)

## Order and stability constraints
- Preserve semantic order: identity/safety first, capability description next, runtime/time last.
- Keep `SectionOrder` values sparse so you can insert future sections without renumbering all entries.
- Avoid order collisions unless output adjacency is intentionally equivalent.
- Keep unstable sections at the tail to reduce cache-prefix churn.
- Keep stable sections deterministic across turns for identical context input.
- Keep section text concise; avoid dumping large blobs into system prompt.
- Never perform I/O, storage reads, or network calls inside `build()`.
- Return empty strings only when intentional; composer will drop trimmed empty output.
- Use `enabled()` for runtime gating, not `build()` side effects.
- Validate insertion impact with ordering tests (`SystemPromptComposer`) before merging.

## Practical review checklist
- Confirm section is exported in `mod.rs`.
- Confirm section is registered in default section list when intended.
- Confirm order value and `is_stable()` choice are justified in code review.
- Confirm tests assert relative ordering and expected prompt fragments.
