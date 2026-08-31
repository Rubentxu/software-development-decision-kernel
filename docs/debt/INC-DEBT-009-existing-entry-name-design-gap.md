---
id: INC-DEBT-009-existing-entry-name-design-gap
title: "ExistingEntry.name design gap — diff_existing_target didn't compare names"
status: closed
severity: low
priority: P3
cluster_id: CL-03
created: 2026-08-25
created_by: sddk-debt-verify (cycle-34, A-min smoke)
owner: orchestrator
cycle_source: p-52b95ef55999f9de/kernel-cycle-35-inc-debt-009-existing-entry-name-design-gap
attribution: pre_existing
base_sha_when_discovered: 860bf3a (cycle-34 archive)
---

# INC-DEBT-009 — ExistingEntry.name design gap

> Closed at cycle-35.

## Context

`ExistingEntry.name` field captured by `read_existing()` but never compared in `diff_existing_target()`.
As a result, renames in IDE configs passed undetected by the reconciler.
Also, `#[allow(dead_code)]` annotation on the field was technically incorrect (field IS read by callers, just compared to itself in current flows).

## Resolution

Cycle-35:
- Added `if existing.name != target.name { diffs.push(FieldDiff { field_name: "name", ... }) }` at the top of `diff_existing_target`
- Removed `#[allow(dead_code)]` annotation from `ExistingEntry.name`
- Removed the C3 follow-up doc-comment (now obsolete)
- Added RED test `diff_existing_target_emits_name_diff_when_names_differ`

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|---------|
| 2026-08-25 | sddk-debt-verify (cycle-34) | created | C3 design gap noted in INC-DEBT-008 §Open Follow-ups |
| 2026-08-25 | sddk-apply (cycle-35) | status: open → closed | name comparison wired; RED test passes |
