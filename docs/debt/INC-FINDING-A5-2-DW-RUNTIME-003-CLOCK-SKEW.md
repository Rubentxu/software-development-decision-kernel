---
id: INC-FINDING-A5-2-DW-RUNTIME-003-CLOCK-SKEW
title: "DW-RUNTIME-003 anchor comment in restart_survival.rs drifted from what the test actually does"
status: closed
severity: low
priority: P3
fingerprint: "9304e0e450505577"
fingerprint_aliases: []
cluster_id: CL-DOC-QUALITY
created: 2026-09-17
created_by: cycle-a5-2-durability-rebuild-recovery
owner: cycle-a5-2-durability-rebuild-recovery
resolved_by: cycle-a5-2-durability-rebuild-recovery
resolved_at: 2026-09-17
closed_at: 2026-09-17
closed_by: cycle-a5-2-durability-rebuild-recovery
---

# INC-FINDING-A5-2-DW-RUNTIME-003-CLOCK-SKEW — anchor comment did not match the test it annotated

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`crates/sddk-storage/tests/workflow_run_restart_survival.rs` was
marked `#[ignore]` since v1.89.1 with a long comment block referring to
"DW-RUNTIME-003" and a wall-clock-skew theory (hardcoded
`"2026-09-07T12:00:00.000Z"` sorting BEFORE the record_run event
under `ORDER BY occurred_at DESC LIMIT 1`).

A5-2 re-read the test (cycle 2026-09-17): the comment does not describe
the test. The test inserts a `pending->running` event *directly* into
`workflow_run_events_v1`, then asserts `loaded.state == Pending` —
which is the bug, not the diagnostic. The query the comment complains
about (`ORDER BY occurred_at DESC`) was never the loader's query; the
loader uses `ORDER BY rowid DESC`, which is durable.

So the test was passing for the wrong reason (asserting the stale
invariant), with a comment claiming a clock skew that was not the
failure mode.

## Rationale

- **Severity low.** No correctness issue; the test asserted the wrong
  thing.
- **Priority P3.** Doc-quality finding closed in the same cycle that
  found it.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-17 | cycle-a5-2 | created + closed | `crates/sddk-storage/tests/workflow_run_restart_survival.rs` rewritten; `#[ignore]` removed; `loaded.state == Running` added at phase 1 and phase 2; clock-skew comment removed. |

## References

- `crates/sddk-storage/tests/workflow_run_restart_survival.rs` (post-A5-2)
- `docs/history/legacy-packages/architecture-a5-a6/architecture-a5/A5-2-RECEIPT.md` §3 finding 1
