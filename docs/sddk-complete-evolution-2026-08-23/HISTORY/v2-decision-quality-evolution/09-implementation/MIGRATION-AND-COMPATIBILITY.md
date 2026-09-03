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
