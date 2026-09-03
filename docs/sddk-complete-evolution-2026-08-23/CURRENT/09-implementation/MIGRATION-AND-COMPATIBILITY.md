# Migration & Compatibility

## Goal

Introduce the evolution without breaking canonical SDD workflows or prematurely replacing existing verification.

## Phase 1 — additive only

New docs/schemas/skills; no change to A-full/A-lite/A-min/B-direct; no new mandatory gates.

## Phase 2 — evidence bridge

Current verify/debt results may be normalized:

```text
existing evidence → adapter/normalizer → assurance evidence
```

Do not rerun identical checks solely to satisfy a new model.

## Phase 3 — optional SDD Adaptive integration

SDD Adaptive may request assurance capabilities conditionally. Current workflows remain baseline/reference.

## Phase 4 — improvement laboratory

Existing Workflow Laboratory receives candidate lifecycle extensions. No existing routing/workflow policy changes automatically.

## Historical data

Do not backfill every cycle eagerly. Optional import must mark provenance as `imported` and cannot invent missing metadata.

## Skill migration

Do not replace `rust-patterns`. Add compact `systems-reasoning` and `rust-systems-reasoning`. Consolidation of overlapping skills uses candidate/evaluation rather than arbitrary deletion.

## Configuration versioning prerequisite

Before GCI experimentation, all mutable harness artifacts need stable content hashes/version refs.

## Rollback

Every promoted configuration version keeps parent version + promotion receipt so rollback is deterministic.

## Kernel compatibility

No new language-specific dependency and no new WorkflowIR primitive required for EA/GCI v1.

# Agent-interface migration

## Principle

Dual-surface migration:

```text
low-level CLI remains
+
high-level Goal Surface introduced
```

Do not perform a flag day.

## Step 0 — Inventory legacy behavior

For each candidate command sequence capture:

```text
preconditions
state transitions
gates
reports
artifacts
receipts
events
metrics
failure/blocker behavior
recovery behavior
```

## Step 1 — Shadow plan

`goal plan` computes without applying.

Compare planned obligation graph with legacy workflow requirements.

## Step 2 — Shadow execution in isolated fixtures

Run:

```text
LegacySequence
vs
GoalRun
```

from the same starting state.

## Step 3 — Behavioral parity

Compare:

```text
final state
invariants
report kinds + schema versions
receipts
evidence
events
metrics
blockers
```

A concise new GoalResult does not count as a replacement for detailed reports.

## Step 4 — Agent opt-in

Prompts/agents may choose goal interface.

Low-level path remains fallback.

## Step 5 — Default semantic path

Only after measured parity and lower interaction overhead.

## Step 6 — Prompt simplification

Remove mechanical sequences from prompts only after the runtime owns them.

Keep:

```text
purpose
goal selection guidance
cognitive responsibilities
hard semantic rules
```

Remove:

```text
stable shell recipes
manual internal plumbing
```

## Step 7 — Low-level deprecation

Optional and much later.

Only deprecate an operation if it no longer has unique debugging/recovery value.

## Reports

Existing report formats remain authoritative until a separate versioned spec explicitly supersedes them.

A high-level API indexes existing reports; it does not silently rename/drop them.
