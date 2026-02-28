# Anthropic vs OpenClaw Conversation Compaction: Mechanism, Cost, and Design Trade-offs

_Last updated: 2026-02-28 (CST)_

## TL;DR

- **Anthropic now has a built-in compaction mechanism** in the Messages API (beta): `context_management.edits: [{ type: "compact_20260112" }]` with beta header `compact-2026-01-12`.
- Anthropic compaction is **server-side**, **opt-in**, and **trigger-based** (default trigger 150k input tokens, minimum 50k). It emits a `compaction` content block and then automatically ignores earlier blocks on subsequent turns.
- **OpenClaw compaction is hybrid**:
  - It leverages the underlying Pi session compaction flow (`session.compact(...)`) and wraps it with OpenClaw-specific guardrails.
  - In `safeguard` mode (default in OpenClaw config), it performs custom summarization/pruning logic in an extension before compaction finalization.
  - It adds operational layers: pre-compaction memory flush, tool-result sanitization/repair, overflow retries, post-compaction context refresh/audits.
- **Cost model differs**:
  - Anthropic explicitly charges compaction as additional sampling iterations (`usage.iterations` includes a `compaction` iteration).
  - OpenClaw may incur additional model calls via summarization stages (`generateSummary(...)` loops/retries), plus optional memory flush turns and overflow-recovery attempts.

---

## 1) Anthropic built-in compaction

## 1.1 Does Anthropic provide built-in compaction/summarization?

**Yes.** Anthropic documents a server-side compaction feature in Claude API docs:
- `Compaction` doc page (beta feature):
  - URL: https://platform.claude.com/docs/en/build-with-claude/compaction
  - Uses beta header: `compact-2026-01-12`
  - API edit type: `compact_20260112`

This is not just a cookbook trick; it is an API-level context management strategy.

## 1.2 Mechanism and internal behavior (as documented)

From Anthropic docs:
1. API detects token threshold crossing.
2. Generates a summary.
3. Returns a `compaction` block.
4. Continues with compacted context.

On later requests, Anthropic states it **automatically drops blocks before the `compaction` block** when processing prompt context.

### Structure
- Compaction output appears as a `content` block of type `compaction` (plus normal `text` output blocks).
- For streaming, compaction uses dedicated compaction deltas (`compaction_delta`) rather than regular text chunk streaming.

### Triggering
- Configurable trigger via `trigger: { type: "input_tokens", value: ... }`
- Default trigger: **150,000 input tokens**
- Minimum allowed trigger: **50,000 tokens**

### Control knobs
- `instructions`: replaces default summarization prompt entirely.
- `pause_after_compaction`: return after compaction block so caller can inject/preserve extra content before continuing.

## 1.3 Compression strategy design

Anthropic compaction strategy is **summary replacement** rather than plain truncation:
- Older context is summarized into a compaction block.
- Earlier blocks are logically superseded by that block.
- Optional workflow allows preserving recent messages manually (with `pause_after_compaction`).

Related Anthropic context management options (separate feature set):
- Context editing page:
  - URL: https://platform.claude.com/docs/en/build-with-claude/context-editing
  - Includes server-side clearing strategies (tool result clearing, thinking block clearing), and discusses client-side SDK compaction alternative.

## 1.4 Cost and token accounting

Anthropic explicitly documents that compaction introduces an **additional sampling step** and affects billing/rate limits.

Key points from docs:
- `usage.iterations` includes per-iteration usage, with compaction iteration(s) + message iteration(s).
- Top-level `usage.input_tokens`/`usage.output_tokens` **exclude compaction iterations**.
- To compute total billed tokens with compaction enabled, sum all `usage.iterations` entries.
- Re-using an already produced compaction block in later requests does not itself incur a new compaction step unless a new compaction is triggered.

## 1.5 Automatic vs opt-in

- **Opt-in** via beta header + context edit config.
- Once enabled, **automatic at runtime** when trigger threshold is crossed.

## 1.6 What is preserved vs lost

Preserved:
- The generated compaction summary block.
- Any content after latest compaction block.
- Optionally preserved exact turns if app uses `pause_after_compaction` and appends selected turns.

Lost/reduced:
- Detailed pre-compaction raw history content (replaced by summary abstraction).
- Potentially nuanced low-salience details not retained in summary prompt/objective.

---

## 2) OpenClaw compaction logic (source-based)

Scope analyzed under: `~/code/references/openclaw/`

Primary files:
- `src/agents/compaction.ts`
- `src/agents/pi-embedded-runner/compact.ts`
- `src/agents/pi-extensions/compaction-safeguard.ts`
- `src/agents/pi-embedded-runner/run.ts`
- `src/auto-reply/reply/agent-runner-memory.ts`
- `src/auto-reply/reply/memory-flush.ts`
- `src/config/defaults.ts`
- `src/agents/pi-settings.ts`
- `src/auto-reply/reply/post-compaction-context.ts`
- `src/auto-reply/reply/post-compaction-audit.ts`
- `src/gateway/server-methods/sessions.ts` (separate transcript line trimming endpoint)

## 2.1 When does OpenClaw decide to compact?

There are several entry paths:

1) **Automatic overflow recovery during run loop**
- In `pi-embedded-runner/run.ts`, likely context-overflow errors trigger explicit compaction attempts (`compactEmbeddedPiSessionDirect(...)`) with bounded retries.
- It avoids immediate double-compaction if in-attempt compaction already happened.
- Includes fallback: oversized tool-result truncation if compaction isn’t enough.

2) **Manual `/compact` command**
- `auto-reply/reply/commands-compact.ts` invokes `compactEmbeddedPiSession(...)`, supports optional custom instructions.

3) **Pre-compaction memory flush phase (proactive)**
- `agent-runner-memory.ts` + `memory-flush.ts` run a dedicated memory-writing turn when session token usage nears compaction threshold.
- Trigger logic: `shouldRunMemoryFlush(...)` compares current token load vs `contextWindow - reserveTokensFloor - softThresholdTokens`.

4) **Gateway `sessions.compact` RPC** (different thing)
- `gateway/server-methods/sessions.ts` has `sessions.compact` that trims transcript file line count (`maxLines`, default 400).
- This is **file-level retention trimming**, not semantic LLM summarization compaction.

## 2.2 Compression strategy (summarize vs truncate vs hybrid)

OpenClaw is **hybrid** and multi-layered:

### A) Safeguard summarization extension
- `compaction-safeguard.ts` hooks `session_before_compact`.
- Uses custom summarization pipeline from `agents/compaction.ts`:
  - token estimation
  - adaptive chunking (`computeAdaptiveChunkRatio`)
  - chunk-by-max-tokens with safety margin
  - multi-stage summarization (`summarizeInStages`) with fallback paths
  - retry logic around `generateSummary(...)`

### B) History-share pruning before summarization
- `pruneHistoryForContextShare(...)` drops oldest chunks when history exceeds `maxHistoryShare` of context budget.
- Then tries to summarize dropped chunks to avoid hard-loss.

### C) Oversized/tool-heavy safeguards
- Strips tool-result details from summarization input (`stripToolResultDetails(...)`) to reduce bloat and avoid unsafe/untrusted payload inclusion.
- Repairs orphan tool-result/tool-use pairing (`repairToolUseResultPairing` / `sanitizeToolUseResultPairing`) to keep transcript API-valid.

### D) Overflow fallback truncation
- In runtime overflow path (`run.ts`), if compaction fails to recover, OpenClaw can truncate oversized stored tool results.

So practical behavior is: **summarize-first with pruning and structural repair; truncate as fallback in specific overload cases**.

## 2.3 How system prompts / tool calls / special context are handled

### System prompt handling
- `compact.ts` rebuilds and reapplies full embedded system prompt (`buildEmbeddedSystemPrompt(...)`, `createSystemPromptOverride(...)`, `applySystemPromptOverrideToSession(...)`) for compaction runs.
- After auto-compaction completion, OpenClaw can inject post-compaction startup reminder context from AGENTS.md sections (`readPostCompactionContext(...)`).
- It also audits whether required startup files were re-read (`post-compaction-audit.ts`).

### Tool calls/results handling
- Tool/result pairing is explicitly validated/repaired before compaction and after turn-limiting.
- Tool result details are stripped in compaction summarization token path for safety and context control.
- Tool failures can be summarized into a dedicated section in safeguard summary.

### “Pinned messages”
- In the analyzed compaction path, there is **no explicit pinned-message preservation mechanism** analogous to “always keep pinned turns”.
- Preservation is instead policy-based (recent-window retention, max history share, optional manual preservation patterns, and post-compaction context refresh).

## 2.4 Config and defaults

- `agents.defaults.compaction.mode` exists with values `default | safeguard`.
- `applyCompactionDefaults(...)` in `config/defaults.ts` sets default mode to **`safeguard`** if unset.
- Compaction reserve floor support: `reserveTokensFloor` (default constant in `pi-settings.ts`: `20_000`).
- Memory flush defaults:
  - enabled by default
  - soft threshold default 4000 tokens
  - customizable prompt/system prompt

## 2.5 Cost model in OpenClaw

OpenClaw can incur extra cost from multiple sources:

1) **Compaction summarization calls**
- `agents/compaction.ts` calls `generateSummary(...)` per chunk with retries (up to 3 attempts in retry wrapper), potentially multiple chunks/stages.

2) **Memory flush turn**
- `agent-runner-memory.ts` may execute a full additional model turn near threshold.

3) **Overflow retries**
- `run.ts` may retry prompt after compaction, and can loop across multiple attempts when overflows persist.

4) **Main response still runs**
- Compaction overhead is in addition to normal assistant generation.

Net: OpenClaw’s effective context survivability is strong, but operational token spend can rise due to proactive and reactive safeguards.

---

## 3) Side-by-side comparison

| Dimension | Anthropic built-in compaction | OpenClaw compaction stack |
|---|---|---|
| Activation | Opt-in (`context_management.edits` + beta header) | Built into OpenClaw agent runtime; auto-overflow + manual `/compact` + memory flush |
| Where logic runs | Server-side API | Client/runtime orchestration + Pi session compaction + extension hooks |
| Trigger | Input-token threshold (default 150k, min 50k) | Overflow detection + configurable thresholds + memory flush proximity logic |
| Core strategy | Summary block (`compaction`) replacing earlier context | Hybrid: staged summarization + pruning + transcript repairs + truncation fallback |
| Preservation semantics | Keep summary + post-compaction context; prior blocks ignored | Keeps structured summary, can include tool-failure/file-op metadata, post-compaction rules injection/audit |
| Tool-heavy handling | General API behavior + related context-editing features | Explicit tool-result detail stripping, pairing repair, oversized tool-result truncation fallback |
| Cost accounting | Explicit `usage.iterations`; compaction iteration billable | Multiple potential extra model calls (summary chunks, retries, memory flush, retries) |
| Dev ergonomics | Simpler integration | More tunable, more complex operationally |

---

## 4) Pros / cons

## Anthropic built-in compaction

Pros:
- First-class API semantics; minimal implementation burden.
- Clear token accounting model (`usage.iterations`).
- Native block type and stream events make downstream handling deterministic.

Cons:
- Beta-gated and model-limited (per current docs).
- Same model must perform summarization (no cheaper summarizer model override in current limitation note).
- Summary quality/control depends on prompt and model behavior; lossiness is inevitable.

## OpenClaw approach

Pros:
- Strong operational resilience for agentic tool loops (repair + fallback layers).
- Fine-grained policy control (`maxHistoryShare`, reserve floors, memory flush, custom instructions).
- Explicit safety hardening around tool payloads and transcript integrity.

Cons:
- More moving parts and complexity.
- Potentially higher and less predictable token overhead due to retries/multi-stage summarization/extra turns.
- Harder to reason about exact marginal cost per request than a provider-native `usage.iterations` model.

---

## 5) Lessons for `agent-demo`

1. **Prefer provider-native compaction when available**
- If target model stack supports Anthropic-style server compaction, use it as baseline for reliability and observability.

2. **Track compaction cost explicitly**
- Mirror Anthropic’s `iterations` accounting concept in your own telemetry even for non-native compaction paths.
- Distinguish:
  - main generation cost
  - compaction/summarization cost
  - recovery/fallback cost

3. **Keep hybrid fallback layers**
- Even with native compaction, retain local safeguards for:
  - oversized tool results
  - broken tool-use/result pairings
  - post-compaction protocol restoration

4. **Design preservation tiers**
- Tier 1: exact recent turns
- Tier 2: semantic summary of older turns
- Tier 3: archived/raw transcript outside active context

5. **Guardrails before compaction**
- OpenClaw’s memory flush concept is useful for preserving durable facts before major compression events.

6. **Operationally separate “semantic compaction” vs “storage trimming”**
- Keep API clear like OpenClaw does: LLM compaction flow vs file-level line truncation (`sessions.compact`).

---

## 6) Cost implications summary

- **Anthropic native compaction**: predictable per-request visibility via `usage.iterations`; compaction itself billed as extra iteration.
- **OpenClaw custom stack**: potentially more costly in pathological long/tool-heavy tasks due to multiple summary passes and retries, but can reduce failure rate and maintain continuity better under noisy tool transcripts.
- Best practice: implement a budget policy with compaction counters and request-level cost attribution.

---

## Sources

### Anthropic docs
- Compaction (official): https://platform.claude.com/docs/en/build-with-claude/compaction
- Context editing (official): https://platform.claude.com/docs/en/build-with-claude/context-editing
- Token counting (official): https://platform.claude.com/docs/en/build-with-claude/token-counting
- Cookbook (SDK/client-side compaction context): https://platform.claude.com/cookbook/tool-use-automatic-context-compaction
- Context engineering article: https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents

### OpenClaw code references
- `~/code/references/openclaw/src/agents/compaction.ts`
- `~/code/references/openclaw/src/agents/pi-embedded-runner/compact.ts`
- `~/code/references/openclaw/src/agents/pi-extensions/compaction-safeguard.ts`
- `~/code/references/openclaw/src/agents/pi-embedded-runner/run.ts`
- `~/code/references/openclaw/src/auto-reply/reply/commands-compact.ts`
- `~/code/references/openclaw/src/auto-reply/reply/agent-runner-memory.ts`
- `~/code/references/openclaw/src/auto-reply/reply/memory-flush.ts`
- `~/code/references/openclaw/src/agents/pi-settings.ts`
- `~/code/references/openclaw/src/config/defaults.ts`
- `~/code/references/openclaw/src/auto-reply/reply/post-compaction-context.ts`
- `~/code/references/openclaw/src/auto-reply/reply/post-compaction-audit.ts`
- `~/code/references/openclaw/src/gateway/server-methods/sessions.ts`
