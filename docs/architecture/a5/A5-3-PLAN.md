# A5-3 — Concurrency / CAS / authority side-effect races

> Cycle: `p-63676b11dc0ef88f/a5-3-concurrency-cas-authority-races`
> Gates: G4 (concurrency correctness), G5 (authority fail-closed).
> Risks: R3 (P0), R4 (P0), R5 (P0), R6 (P1), R12 (P1).
> Hard constraint (A5 programme): **no new abstraction**; falsify and pin
> the existing ones. ADR-0097 remains the common revision substrate
> authority; the decision→effect atomicity boundary must be **named and
> tested** (G5) or declared a blocker.

## Scope budget (one cycle)

| Risk | Probe (falsification matrix §7 / §8) | Anchor |
|---|---|---|
| **R12** | Non-blocking Parallel path: children persisted to `node_runs_v1`? | `parallel_spec_scenarios.rs::864, 984` (both `#[ignore]`); `workflow_runtime.rs:1079` already side-steps the bug |
| **R3** | Two writers on same expected ref → typed conflict (not silent overwrite) | `record_attempt`, `record_node_run_for_run`, `record_run` use `INSERT OR REPLACE` |
| **R6** | Retry does not double-apply | `IdempotencyConflict` path on attempt / node_run / workflow_run; already partially covered |
| **R4** | TOCTOU between decision and effect | authority engine `AdmissionDecision` → caller effect path; no atomicity boundary today |
| **R5** | Denied action → zero side effect; `RequireApproval` bypass | authority engine → caller effect path |

Out of scope (verified by source comment or A5-2 closure):

- `event_envelope_golden.rs` → manual harness → **ACCEPTED_RISK**
- `event_store.rs:139` → tampering test → **ACCEPTED_RISK** (chain verify covers)
- `cli.rs:7146` → REMEDIATING transition missing → **defer to A5-C**
- `workflow_runtime_demo.rs:377` → T1 manual harness → **ACCEPTED_RISK**
- `evaluate_lens` deletion → **A5-4**
- Migration / durability probes → **A5-2 closed**

## R12 read (pre-cycle, 2026-09-17)

The two `#[ignore]` tests in `parallel_spec_scenarios.rs` (lines 864, 984)
both use `make_ir()` which produces an IR with **zero registered operators**.
They then call `WorkflowRuntime::execute()` and expect 3 children to be
persisted. The runtime can't dispatch operators that don't exist.

Independent of the IR setup, **line 1079-1081 of `workflow_runtime.rs` already
side-steps the non-blocking path**:

```rust
// Delta-4: Parallel operators now use the blocking path (pending_sender = None).
// The non-blocking path had a sender-drop bug that prevented proper result collection.
let pending_sender: Option<std::sync::mpsc::Sender<ChildResult>> = None;
```

The non-blocking code path in `operator.rs:1196-1356` still exists but is
**dead code from the runtime's perspective**. The ignored tests test dead
code + missing IR setup — they cannot be un-ignored by fixing only the IR.

**Honest disposition (per A5-DEBT-DISPOSITION §3.4 row MUST_CLOSE_A5):**

The two ignored tests fall into one of three buckets:

| Option | What changes | Cycle impact |
|---|---|---|
| A. Delete | remove the two tests + the now-dead non-blocking path in operator.rs | small, honest, loses any future non-blocking work |
| B. Revive non-blocking path | rewrite supervisor + runtime drain + tests with real operators | large, may not finish in one cycle |
| C. Pin blocking path as production boundary | delete non-blocking path + 2 tests; document the choice | small, honest, preserves intent |

**A5-3 chooses C** — the runtime already chose blocking in Delta-4; the
right hardening move is to **remove the dead code** so it cannot silently
regain activity and to **document the boundary** as the production
decision. This is not "fix the bug"; it's "make the bug impossible to
regress to". B stays in `A5-DEFERRED-POST-BASE.md` if we ever want async
resume.

## R3 read (pre-cycle, 2026-09-17)

`INSERT OR REPLACE` is used in:

| Site | SQL | Risk |
|---|---|---|
| `record_run` (workflow_runs_v1) | `INSERT OR REPLACE INTO workflow_runs_v1` | R3 — second writer silently replaces first |
| `record_workflow_run_transition` (workflow_run_events_v1) | `INSERT INTO workflow_run_events_v1` | safe (append-only) |
| `record_attempt` | `INSERT OR REPLACE INTO node_runs_v1` for node_run, separate attempt insert | **two writes per attempt**, not atomic |
| `record_node_run_for_run` | `INSERT OR REPLACE INTO node_runs_v1` | R3 — second writer silently replaces |

The substrate already enforces `node_runs_v1` and `workflow_runs_v1` as
**append-only by trigger** for `UPDATE` (re-verified by A5-2 F5 via
`update_on_workflow_runs_v1_fails`). But `INSERT OR REPLACE` is a different
op — it does DELETE+INSERT and is **not blocked** by the append-only triggers.

**Honest disposition:**

1. Change `INSERT OR REPLACE` → `INSERT` with typed conflict detection (the
   `ON CONFLICT DO NOTHING` path returns a typed error if the conflict is
   "wrong identity"). The substrate's existing `IdempotencyConflict` variant
   covers this — already returned by the storage layer for some paths.
2. Falsify: two writers calling `record_attempt` for the same `attempt_id`
   on the same `run_id`+`node_id`+`attempt_seq` → one succeeds, other gets
   `StorageError::IdempotencyConflict` (not silent replace).
3. Pin: `record_attempt` and `record_node_run_for_run` test pair
   (R3-falsify, R3-idempotent-retry).

## R6 read (pre-cycle, 2026-09-17)

`record_attempt` and `record_node_run_for_run` already handle
`IdempotencyConflict` as a typed no-op in the non-blocking Parallel path
(see `operator.rs:1293, 1334`). The **storage layer** however does not yet
return `IdempotencyConflict` for attempt / node_run — only for cycle lease
(in `acquire_cycle_lease`). The current operator-layer `IdempotencyConflict`
catch is only triggered by **other call sites**, not by `record_attempt`
itself.

**Honest disposition:**

1. Make `record_attempt` and `record_node_run_for_run` return
   `IdempotencyConflict` when the `(run_id, node_id, attempt_seq)` triple
   is already present and the **incoming attempt differs** from the
   existing one (i.e. a real conflict, not a replay).
2. Pin: a retry test that calls `record_attempt` twice with **identical
   attempt content** → second returns `Ok(())` (idempotent replay); a
   **divergent** retry returns `IdempotencyConflict`.

## R4 / R5 read (pre-cycle, 2026-09-17)

`AuthorityEngine::evaluate(&AdmissionContext) -> AdmissionDecision`
returns one of `Allow`, `Deny`, `RequireApproval`. The caller holds the
Decision and applies the effect. There is **no named atomicity boundary**
between decision and effect today.

**Honest disposition:**

The cycle cannot fix TOCTOU in one shot — that requires the
**decision→effect atomicity boundary** to be a real artefact (e.g.
"DecisionReceiptId is bound to the proposal fingerprint; effect is
applied only if the proposal still hashes to DecisionReceiptId's
binding"). That is a redesign, not a hardening.

What A5-3 **can** pin is:

1. **R5 (denied ⇒ zero side effect):** every call site that uses the
   authority engine MUST short-circuit on `Deny` before any side effect
   (file write, git push, CAS put, network call). Falsify by enumerating
   callers and writing the missing short-circuit tests.
2. **R5 (RequireApproval bypass):** the caller MUST check
   `is_require_approval()` and **not** apply the effect until the
   approval is granted. Falsify: same enumeration + bypass test.
3. **R4 (TOCTOU):** name the boundary. The cycle will document the
   boundary **as it exists today** (a soft one — the engine returns the
   decision, the caller applies the effect, no atomicity) and **register
   it as a known limitation** with a follow-up cycle in
   `A5-DEFERRED-POST-BASE.md` or in a future A6 plan.

This is honest. The cycle will **not** claim R4 is closed — it will
**pin** the gap as a documented limitation.

## Deliverables (behaviour, no new architecture)

1. **F1 (R12) — RED → GREEN.** Force-run the two ignored tests against
   current `main`; capture the exact failure. Then: delete the two
   ignored tests + the dead non-blocking path in `operator.rs:1196-1356`.
   Document the choice in the A5-3 receipt and register
   `INC-FINDING-A5-3-DELTA-4-NON-BLOCKING-PATH-ABANDONED` as a low/P3
   doc-quality finding.
2. **F2 (R3) — RED → GREEN.** New test
   `crates/sddk-storage/tests/concurrency_record_attempt.rs` (2-3 tests):
   two writers on same `(run_id, node_id, attempt_seq)` triple → first
   `Ok(())`, second `IdempotencyConflict`. Fix `record_attempt` and
   `record_node_run_for_run` to detect the conflict and return the typed
   error instead of silently replacing.
3. **F3 (R6) — RED → GREEN.** Same file, 1 test: identical retry →
   `Ok(())`; divergent retry → `IdempotencyConflict`. Pin the invariant.
4. **F4 (R5) — RED → GREEN.** New test file
   `crates/sddk-engine/tests/authority_fail_closed.rs`: enumerate the
   authority engine's callers in the engine crate; for each, write a
   test that:
   - calls `evaluate()` → `Deny`
   - asserts the call site returns early with no side effect
   - asserts `is_require_approval()` short-circuits too
   Fix any short-circuit gap (likely 1-3 sites; small).
5. **F5 (R4) — document the boundary.** Add a section to
   `docs/architecture/adrs/ADR-0097-COMMON-REVISION-SUBSTRATE.md`
   (or a sibling ADR if one doesn't fit) explicitly stating the
   decision→effect atomicity boundary as it exists today. Register
   `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY` with disposition
   `BLOCKS_BASE_PRODUCTION_READY` (G5 blocker rule) → the receipt
   acknowledges it.
6. **F6 — receipts.** `docs/architecture/a5/A5-3-RECEIPT.md` modelled on
   `A5-2-RECEIPT.md`, with gate evidence per F1–F5 and the
   "still-ignored" list. Register any new findings.

## Hard non-goals (declared)

- No new crate / trait / port.
- No async runtime / non-blocking Parallel revive (option B above).
- No redesign of the authority engine (R4 redesign is deferred; the
  cycle pins the gap).
- No `proptest` property tests in this cycle (candidates listed in
  `A5-FALSIFICATION-MATRIX.md §12` for later).
- No sender-drop revive — the runtime already chose blocking.

## Risk gate (initial, 2026-09-17)

- `cargo test --workspace`: pre-cycle state = green at v1.169.74
  (`78873d3`), minus the 13 still-ignored tests.
- The R12 ignored tests are pre-known to fail for two reasons (empty IR
  + dead path); forcing them with `--ignored` will capture the RED.
- The R3 tests must be **written** before the fix — they don't exist
  yet because no test asserts the typed-conflict invariant today.
- The R5 tests require enumerating authority callers; the enumeration
  is a discovery step, not an assumption.

## Open questions (to resolve in RED phase)

1. **R4 disposition.** Is naming-the-boundary-as-it-exists-today
   acceptable as a cycle deliverable, or does it force the cycle into
   "no R4 closure" and require A6? My read: **acceptable**, because the
   risk register already shows R4 as P0/G5; the cycle pins the gap
   honestly and G5 has the explicit blocker rule ("if no safe boundary
   exists, that is a `BASE_PRODUCTION_READY` blocker"). I'll register
   `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY` and the receipt will say
   R4 is **partially closed**: documented + blocked_at_base, not
   silently absorbed.
2. **R12 dead-code removal.** The non-blocking code in `operator.rs` is
   ~160 lines; removing it requires reading the rest of `operator.rs` to
   confirm nothing else depends on the supervisor machinery. I will
   read those lines (1196-1356) + the supervisor helpers
   (`CountingSemaphore`, `PermitGuard`, `build_attempt`, etc.) and
   check before removing.
3. **R3 fix scope.** `INSERT OR REPLACE` is used in ~4 storage sites.
   The cycle will fix `record_attempt` and `record_node_run_for_run`
   (the two called by Parallel children) and **document** `record_run`
   as a separate decision (workflow-run start has different semantics;
   it may legitimately be re-recorded for retry).
