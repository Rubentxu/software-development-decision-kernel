# A5-2 — Durability / rebuild / recovery

> Cycle: `p-63676b11dc0ef88f/a5-2-durability-rebuild-recovery`
> Gates: G2 (durability), G3 (rebuildability), G6 (migration/compatibility).
> Risks: R1 (P0), R2 (P0), R20 (P1).
> Hard constraint (A5 programme): **no new abstraction**; falsify and pin
> the existing ones.

## Scope budget (one cycle)

Inventory of everything in the workspace that maps to A5-2:

| Source | Probe | Risk | Status (2026-09-17) |
|---|---|---|---|
| `crates/sddk-storage/tests/workflow_run_restart_survival.rs:89` | `run_survives_restart_with_equivalent_identity_and_provenance` | R1 | `#[ignore]`. **Initial reading**: test forced → green in 0.40 s. **Deeper reading of phase 2 reveals**: the test inserts a `pending→running` event *only into the event log* (`workflow_run_events_v1`), then asserts `loaded.state == Pending` (the stale snapshot) and `latest_workflow_run_state == Running`. The test never asserts that `load_run.state == Running` after restart — which is exactly the bug. The DW-RUNTIME-003 "clock-skew" comment does not match what the test does. The honest reading: the test passes today because it asserts the wrong thing. |
| Read of `crates/sddk-storage/src/graph_store.rs` | `record_workflow_run_transition` writes the event log but never updates `workflow_runs_v1.state`. `load_run` reads the (stale) snapshot directly. | R1 (the real one) | **Code analysis finding**, not yet pinned by a test. Falsifiable in <100 lines. |
| Falsification matrix §6 | "crash between canonical append and projection rebuild" | R1 | not written |
| Falsification matrix §6 | "crash during CAS/ref update — partial state becomes authority" | R2 | not written |
| Production contract §G6 | migration breaks persisted state | R20 | covered by `migration_append_only_tests`; this cycle re-executes |

Out of scope (verified by source comment or anchor):

- `parallel_spec_scenarios.rs:864/984` → sender-drop → **R12 / A5-3**
- `event_envelope_golden.rs` → manual harness → **ACCEPTED_RISK**
- `event_store.rs:139` → relies on bypass → **ACCEPTED_RISK** (chain verify covers it)
- `cli.rs:7146` → REMEDIATING transition missing → A5-2 adjacent, **defer to A5-C**
- `workflow_runtime_demo.rs:377` → T1 manual harness → **ACCEPTED_RISK**

## Deliverables (behaviour, no new architecture)

1. **F1 — rewrite the existing ignored test to assert the bug.**
   Change `run_survives_restart_with_equivalent_identity_and_provenance`
   so phase 2 asserts `loaded.state == WorkflowRunState::Running` (the
   currently-buggy invariant) and a matching `latest_workflow_run_state`.
   Remove the clock-skew comment that is unrelated. The new assertions fail
   today against `main` (RED captured). Keep `#[ignore]` removed; let cargo
   surface the failure so this is a real gate.

2. **F2 (R1.a) — fix `workflow_runs_v1.state` write order.**
   Inside `record_workflow_run_transition`, in the same `Immediate`
   transaction, add `UPDATE workflow_runs_v1 SET state = ?2, updated_at = ?3
   WHERE run_id = ?1` right after the event-log insert. Atomic, so the
   snapshot row and the log row commit together or not at all.
   No schema change. No migration.

3. **F3 (R1.b) — new probe: state survives a process drop.**
   New integration test in `crates/sddk-storage/tests/state_survives_restart.rs`:
   - record a run (Pending), drop the store
   - reopen the store, call `record_workflow_run_transition(..., Running, ...)`
     via the public API (not direct SQL insertion)
   - drop the store, reopen, assert `load_run(...).state == Running` AND
     `latest_workflow_run_state(...) == Running`
   This is the test the existing ignored test *should* have been. F1 keeps
   it as an in-place rewrite of the existing file; F3 adds a parallel one
   that uses the public API throughout.

4. **F4 (R2) — CAS partial-state probe.**
   Read `crates/sddk-storage/src/cas.rs` first to find the actual loader
   and its typed errors, then write a new integration test that corrupts
   a CAS object (truncated, bad checksum, wrong scheme id) and asserts:
   - the loader returns a typed error
   - no degraded read silently substitutes the corrupt blob
   - the still-valid objects in the same store remain readable

5. **F5 (R20) — migration re-execution under `SQLX_OFFLINE=true`.**
   `cargo test -p sddk-storage --test migration_append_only_tests` with
   `SQLX_OFFLINE=true` so the test does not touch a live DB. Confirm
   green; record the SHA and elapsed.

6. **F6 — receipts.**
   - `docs/architecture/a5/A5-2-RECEIPT.md` modelled on `A5-1-RECEIPT.md`,
     with gate evidence per F2–F5 and the "still ignored" list.
   - Migration of two persisted follow-up items: rename
     `INC-DEBT-017-storage-acquire-cycle-lease-no-pre-check.md` only if the
     cycle touches it; otherwise leave untouched.
   - Register the verified finding (the comment in restart_survival was
     wrong about what is broken) as `INC-FINDING-2026-09-17-DW-RUNTIME-003`
     if it is worth remembering (cluster CL-DOC-QUALITY).

## Hard non-goals (declared)

- No new crate / trait / port.
- No MIGRATION_18 (`event_sequence` column) — option (b) makes it unnecessary.
- No REMEDIATING transition design — that is A5-C.
- No sender-drop fix — that is A5-3.
- No `proptest` property test in this cycle.

## Risk gate (cleared 2026-09-17)

- `cargo test --workspace`: 4719 / 0 / 14 observed at `06b8937` (just-released
  cycle tag).
- The ignored restart test, when forced, is **green** (not RED). The ignored
  anchor is therefore not a valid falsification gate; F3 captures the real
  RED by reading the code.
- `crates/sddk-storage/src/graph_store.rs` read end-to-end (lines 415, 531,
  571, 636) — the bug and its locality are confirmed.
- F2's fix is additive within one transaction; no schema, no migration, no
  behavioural surprise for callers (one read of `state` stays correct, the
  other becomes correct for the first time).
