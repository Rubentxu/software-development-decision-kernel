---
id: INC-FINDING-A5-3-DELTA-4-NON-BLOCKING-PATH-ABANDONED
title: "Delta-4 abandoned the non-blocking Parallel path in code but left 162 lines + 3 tests behind"
status: closed
severity: medium
priority: P3
fingerprint: "tbd-on-archive"
fingerprint_aliases: []
cluster_id: CL-DEAD-CODE-IGNORED-TESTS
created: 2026-09-17
created_by: cycle-a5-3-concurrency-cas-authority-races
owner: cycle-a5-3-concurrency-cas-authority-races
resolved_by: cycle-a5-3-concurrency-cas-authority-races
resolved_at: 2026-09-17
closed_at: 2026-09-17
closed_by: cycle-a5-3-concurrency-cas-authority-races
cycle_origin: "p-63676b11dc0ef88f/a5-3-concurrency-cas-authority-races"
---

# INC-FINDING-A5-3-DELTA-4-NON-BLOCKING-PATH-ABANDONED — Delta-4 side-stepped the path but did not delete it

> Durable record for one R12 finding across cycles. See ADR-0047 §3.2.

## Context

`crates/sddk-engine/src/workflow_runtime.rs:1079-1081` contains:

```rust
// Delta-4: Parallel operators now use the blocking path (pending_sender = None).
// The non-blocking path had a sender-drop bug that prevented proper result collection.
let pending_sender: Option<std::sync::mpsc::Sender<ChildResult>> = None;
```

This is a **runtime-side decision** to abandon the non-blocking Parallel
supervisor branch. But:

1. The non-blocking code path itself (`crates/sddk-engine/src/operator.rs:1196-1356`,
   ~162 lines, including the supervisor dispatch + sender handling + drain loop)
   was never deleted.
2. Two `#[ignore]` tests in `crates/sddk-engine/tests/parallel_spec_scenarios.rs`
   (lines 864, 984) still exercise the dead path against an IR that has
   zero registered operators. The IR setup makes them unrecoverable by
   fixing only the test harness.
3. One test in `crates/sddk-engine/tests/parallel_concurrency_tests.rs`
   (`parallel_spans_three_ticks_drain`, line 670) directly exercises the
   deleted drain loop.

A5-3 (cycle 2026-09-17) read all three sites, confirmed the path is
unreachable from the runtime, and chose the honest disposition: **delete
the dead code + the orphaned tests**, and document the choice.

## Rationale

- **Severity medium.** Dead code is reachable through unit-test
  compilation but unreachable through the runtime. It inflates the
  cognitive surface area of `operator.rs` and obscures the actual
  production boundary.
- **Priority P3.** No correctness risk; cleanup-only finding closed in
  the same cycle that found it.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-17 | cycle-a5-3 | created + closed | (a) `crates/sddk-engine/src/operator.rs` non-blocking branch deleted; explanatory comment naming Delta-4 + A5-3 R12 left in place. (b) `crates/sddk-engine/src/workflow_runtime.rs:1080-1082` Delta-4 comment updated to reference A5-3 R12. (c) `parallel_spec_scenarios.rs` ignored tests par_006_a + par_006_d removed; S-PAR-007a assertion 23 → 21. (d) `parallel_concurrency_tests.rs::parallel_spans_three_ticks_drain` removed; obsolete `BTreeMap` + `RunId` imports dropped. |

## What this finding is NOT

- **NOT** a refusal to ever build a non-blocking Parallel path. If a
  future cycle needs non-blocking semantics, the path can be rebuilt
  intentionally with a real operator set + correct drain loop. The
  finding is that the abandoned path was left as ghost code.
- **NOT** a complaint about Delta-4 itself. Delta-4 was a correct
  production fix; it just did not clean up after itself.

## References

- `crates/sddk-engine/src/operator.rs` (post-A5-3): non-blocking branch deleted, comment in place
- `crates/sddk-engine/src/workflow_runtime.rs` (post-A5-3): Delta-4 comment updated
- `crates/sddk-engine/tests/parallel_spec_scenarios.rs` (post-A5-3): 2 ignored tests removed
- `crates/sddk-engine/tests/parallel_concurrency_tests.rs` (post-A5-3): `parallel_spans_three_ticks_drain` removed
- `docs/history/legacy-packages/architecture-a5-a6/architecture-a5/A5-3-PLAN.md` §R12 disposition (option C chosen)
- `docs/history/legacy-packages/architecture-a5-a6/architecture-a5/A5-3-RECEIPT.md` §3 finding 2
