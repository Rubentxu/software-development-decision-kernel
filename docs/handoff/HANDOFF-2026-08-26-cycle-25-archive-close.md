# HANDOFF-2026-08-26-cycle-25-archive-close — sddk-framework

## What we did this session

Closed cycles 20-24 retroactively. Discovered the "infra gap" was a naming bug
in orchestrator dispatch packets (`phase.archive.complete` → `archive.complete`).

## Numbers

| metric | value |
|---|---|
| cycles shipped this session | 5 (v1.42.0..v1.42.4) |
| cycles closed retroactively | 5 (status=CLOSED) |
| ledger events | 640 → 687 (+47) |
| tags | v1.42.0, v1.42.1, v1.42.2, v1.42.3, v1.42.4 |
| net LOC delta | -350 (cycle-24 only) |

## Cycle-20 (v1.42.0, A-full)

Multi-WU A-full: WU-1 reqwest 0.12 rustls-tls, WU-2 receiver map + ParallelKey,
WU-3 3-phase tick(), WU-4 dual-path Parallel::evaluate, WU-5 Arc<Mutex<T>>
field types (3 retry passes), WU-6 ADR-0052/0053, WU-7 INV-10 grep gate,
WU-8 version bump 1.41.0→1.42.0. HEAD `fd55295`, tag `v1.42.0`.

## Cycle-21 (v1.42.1, A-min, scope-revised)

After discovering `parking_lot::Semaphore` does not exist, scope revised to:
INV-10 shell gate (`tests/gates/inv_10_no_mutex_on_workflow_state.sh`) + ADR-0055
P3 closure note + version bump 1.42.0→1.42.1. HEAD `838574c`, tag `v1.42.1`.

## Cycle-22 (v1.42.2, A-min)

Fix COUPLE-TRY-UNWRAP-SILENT-SYNC. Replaced `if let Ok(...) Arc::try_unwrap`
with `match` + lock fallback + INV-9 WARN log at `workflow_runtime.rs:605` and
`:689`. 4 RED tests in `tests/arc_try_unwrap_sync_tests.rs`. HEAD `bf72f72`,
tag `v1.42.2`.

## Cycle-23 (v1.42.3, A-min)

tick() extraction 436→21 LOC + 3 private helpers + `TickPhaseOutcome` struct +
4 RED tests. Closes ARCH-LONG-METHOD-TICK. HEAD `928743d1`, tag `v1.42.3`.

## Cycle-24 (v1.42.4, A-lite)

`OperatorContext::for_test` constructor + 7 sites refactored. Honest partial
scope: 22 sites stayed literal for legitimate semantic reasons (child_ctx
inherits from parent, custom executors, Some(pending_sender)). Net -350 LOC.
HEAD `885eaa9`, tag `v1.42.4`. ADR-0058.

## Retroactive closure

On 2026-08-24, all 5 cycles were transitioned `RELEASED → CLOSED` with proper
`archive.complete` transitions. Root cause: orchestrator dispatch packets used
wrong transition name `phase.archive.complete` (CLI returned
`ENGINE_UNREGISTERED_EVALUATOR`). Correct name is `archive.complete`, with gates
`ledger-valid` + `vault-index-current`. ADR-0059.

## All cycle-20 debt closed

- COUPLE-TRY-UNWRAP-SILENT-SYNC (cycle-22)
- ARCH-LONG-METHOD-TICK (cycle-23)
- ARCH-OPERATORCONTEXT-DEDUP (cycle-24, partial)

## Next steps

1. Roadmap Phase 4 Epic DW: Map/Join/Race/Loop operators (A-full, multi-cycle)
2. Optional forward cycle to fix orchestrator.md / mcw.md / archive.md transition
   name references (cycle-26 or later)
3. Optional: orphan cycle-25 cleanup (or leave as planning artifact)