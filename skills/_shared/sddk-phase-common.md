# SDDK Phase — Common Protocol

Shared protocol for every SDDK phase executor. Load it alongside the phase skill
and phase prompt.

CLI ownership and invocation semantics are defined only in
`skills/_shared/cli-usage-contract.md`. Load that contract before invoking any
SDDK lifecycle command.

Executor boundary: perform the assigned phase and return its envelope. Leaf
executors do the work directly. A coordinator may dispatch workers only when
its canonical phase prompt declares the worker set, fan-out inputs, join
condition, and synthesis owner. `sddk-verify` and `sddk-debt-verify` are the
current coordinator exceptions; their workers remain leaf executors.

## A. Skill Loading

1. Check if the orchestrator injected a `## Skills to load before work` block in your launch prompt. If yes, read those exact `SKILL.md` files before task-specific work.
2. If no skills block was provided, check for `SKILL: Load` instructions. If present, load those exact skill files.
3. If neither was provided, read `{project-data-dir}/skill-registry.md` when
   the orchestrator supplied that XDG path, then match triggers and load exact
   `SKILL.md` paths.
4. If no registry exists, proceed with your phase skill only.

NOTE: the preferred path is (1) — exact skill paths selected by the orchestrator. Paths (2) and (3) are fallbacks. Searching the registry is SKILL LOADING, not delegation. If `## Skills to load before work` is present, IGNORE redundant `SKILL: Load` instructions.

## B. Artifact Retrieval

Read dependencies directly from `{cycle-artifacts-dir}` and durable knowledge
from `{vault}`. These filesystem authorities always take precedence.

When the resolved knowledge profile enables Engram, it may be queried for
recovery context. `mem_search` returns previews, so call `mem_get_observation`
before using any result. Engram never replaces a missing authoritative
artifact.

```
mem_search(query: "sddk/{change-name}/{artifact-type}", project: "{project}") → save ID
```

Then **run all retrievals in parallel**:

```
mem_get_observation(id: {saved_id}) → full content (REQUIRED)
```

Do NOT use search previews as source material.

## C. Artifact Persistence

Every phase that produces an artifact MUST persist it. Skipping this BREAKS the pipeline — downstream phases will not find your output.

### File System (always)

Write to `{cycle-artifacts-dir}/{artifact}.md`.

### Engram Memory (optional)

If `sddk knowledge status` reports `engram_enabled: true`, also call:

```
mem_save(
  title: "sddk/{change-name}/{artifact-type}",
  topic_key: "sddk/{change-name}/{artifact-type}",
  type: "architecture",
  project: "{project}",
  capture_prompt: false,
  content: "{your full artifact markdown}"
)
```

`topic_key` enables upserts — saving again updates, not duplicates.
`capture_prompt: false` is mandatory for SDDK artifacts because they are automated pipeline outputs, not human/proactive memory saves.

## D. Return Envelope

> **CRITICAL — Response ordering**: Your FINAL output MUST be text (the return envelope), NOT a tool call. If you need to save to Engram (`mem_save`), do it BEFORE your final text response. Do NOT call `mem_session_summary` — that's for top-level agents only. **Why**: When a sub-agent's last action is a tool call, the parent agent receives only the tool result — your text response (the actual analysis) is lost.

Every phase MUST return a structured envelope to the orchestrator:

- `status`: `success`, `partial`, or `blocked`
- `executive_summary`: 1-3 sentence summary of what was done
- `detailed_report`: (optional) full phase output, or omit if already inline
- `artifacts`: list of artifact keys/paths written
- `next_recommended`: the next SDDK phase to run, or "none"
- `risks`: risks discovered, or "None"
- `skill_resolution`: how skills were loaded — `paths-injected` (received exact skill paths from orchestrator), `fallback-registry` (self-loaded paths from registry), `fallback-path` (loaded via SKILL: Load path), or `none` (no skills loaded)
- `human_summary`: (cycle-close phases only — archive, release, blocked returns during later phases) novice-friendly 2-3 sentence prose rendering for the developer reading the chat. See `prompts/sddk/orchestrator.md` § "Cycle Close: Human-Facing Output". Phase executors do not need to populate this; the orchestrator renders it from `executive_summary` + `risks` + `next_recommended` at cycle close.
- `report_locale_requested`: explicit requested BCP 47 tag or `null`
- `report_locale`: resolved BCP 47 tag from the launch plan
- `report_locale_fallback`: `none | project | parent-language | es`
- `report_audience`: `novice | standard | expert`

Example:

```markdown
**Status**: success
**Summary**: Design created for `{change-name}`. Defined architecture, interfaces, and data flows.
**Artifacts**: `{cycle-artifacts-dir}/design.md`
**Next**: sddk-tasks
**Risks**: None
**Skill Resolution**: paths-injected
```

## E. Context Quality Gates

SDDK uses context quality levels to adapt effort:

| Level | Meaning | Action |
|-------|---------|--------|
| C0 | Unknown context | Full investigation required |
| C1 | Conversational only | Minimal, heuristic approach |
| C2 | Some durable knowledge | Targeted deepening |
| C3 | Full durable knowledge | Direct, minimal verification |

Report context quality in your return envelope under `context_quality`.

## F. Knowledge Pipeline Preflight (Optional)

When the launch plan includes `with_knowledge: true`, the owning phase MAY run the
knowledge pipeline as a preflight check. The pipeline is:

```
scan  →  review plan  →  import  →  verify
```

### Approval Authority

Import consumes the exact scan `plan_id`. Pass `--approve` only with the
comma-separated changed-entry IDs explicitly present in the reviewed launch
plan's `knowledge_approved_entry_ids`. A boolean approval or bare `--approve`
is invalid.

### Quarantine Rule (MANDATORY)

An empty approval list does not skip import. New candidates follow their plan
disposition. Unapproved changed candidates are persisted with `NeedsReview`
authority and may create incidences; they are never silently promoted to
trusted. Only explicitly approved compatible changes become trusted.

## G. CLI Usage

Consume the orchestrator's validated `cli_context`; do not repeat immutable
bootstrap queries. A phase coordinator may refresh cycle status under the
shared freshness policy and owns its gate, transition, and post-transition
ledger verification. Workers and lenses return evidence to their coordinator
and MUST NOT invoke cycle, lock, gate, transition, or ledger lifecycle commands.

Every automated CLI invocation uses structured output as required by
`cli-usage-contract.md`. Preserve authoritative failures and return their error
classification; never convert a failed command into empty state.

## H. Presentation

Follow `prompts/sddk/phase-contracts.md` Report Presentation. Literal headings
in phase templates describe structure, not a forced output language. Localize
presentation prose to `report_locale` while preserving every machine value and
all evidence. Audience changes initial disclosure only, never content.
