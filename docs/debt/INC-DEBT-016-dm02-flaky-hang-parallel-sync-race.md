# INC-DEBT-016: flaky hang in dm02_execute_completes_all_nodes (Parallel sync race)

**status**: closed
**severity**: medium
**priority**: P2
**created_at**: 2026-08-26
**cycle**: 42 (diagnostic) → 43 (fix)
**closed_at**: 2026-08-27 (cycle-43 v1.48.10)
**detected_by**: cycle-40 verify (pre-existing confirmation) + cycle-41 debt-verify attribution + orchestrator reproduction at v1.48.9
**last_updated**: 2026-08-27 (cycle-43 closure)

## Problem

Test `dm02_execute_completes_all_nodes` (`crates/sddk-engine/tests/workflow_runtime_demo.rs:354`) hangs intermittently — the process never completes `runtime.execute()` and must be killed by timeout.

This is NOT a deterministic deadlock: it is a **scheduling-dependent race**. It was flagged as pre-existing during cycle-40 verify and carried forward through cycle-41 as I-2 (MEDIUM/P2). This cycle investigates and fixes the root cause.

### Reproduction evidence (orchestrator, v1.48.9 / 2806bb2)

| Run | Result | Notes |
|-----|--------|-------|
| A | PASS (exit 0) in <60s | emitted **7× WARN** "sync via lock fallback (2–3 refs at sync point)" |
| B | HANG (exit 124, killed by 60s timeout) | no output captured |
| C | HANG (exit 124) | no output captured |

Flake rate ≈ 2/3 in this sample. The WARN signature on passing runs is the smoking gun: the failing path is exercised on every run, but only sometimes deadlocks.

### The mechanism (hypothesis chain, supported by code + ADR)

1. `dm02` builds a sequence IR containing a **`Parallel { branches: [left, right], max_concurrency: 2 }`** operator.
2. Per **ADR-0056** (`docs/adr/ADR-0056-arc-try-unwrap-sync.md`, cycle-22): `Parallel::evaluate` spawns a **supervisor thread** which holds an `Arc::clone(&ctx.node_run)`.
3. After each node's `evaluate()` returns, the runtime tick loop (`crates/sddk-engine/src/workflow_runtime.rs:~814` Ok-branch and `:~910` Err-branch) tries `Arc::try_unwrap(node_run_arc)` to sync state back into `self.nodes[id]`.
4. Because the supervisor thread still holds a clone (observed refcounts: 2 and 3), `try_unwrap` fails → **lock fallback**: runtime does `arc.lock().clone()`.
5. **Race**: if the supervisor thread is blocked holding/acquiring that same mutex while also waiting on something owned by the runtime (or vice versa), the `.lock()` never returns → `tick()` never returns → `execute()` never returns → test hangs.

The WARN text itself says: *"INV-9 audit: investigate thread leak source"* — this INC is that audit finally happening.

### Candidate hypotheses for root cause

| # | Hypothesis | How to confirm |
|---|-----------|----------------|
| H1 | Supervisor thread from `Parallel::evaluate` outlives its useful life (thread leak), retains `node_run_arc` clone, and deadlocks with runtime's fallback `lock()` | Capture per-thread stacks at hang; check if supervisor thread is alive and where it's parked |
| H2 | `pending_sender` channel: a Pending outcome sends through the channel but the drain side expects a different count/order → bounded channel fills → send blocks forever | Trace channel capacity + sender/receiver pairing in tick loop |
| H3 | Executor retention: `OperatorContext.executor` (Arc'd `NoopTaskExecutor`) or store clones keep ctx alive across ticks, accumulating leaked refs until some threshold behavior | Instrument Arc strong_counts across ticks |

H1 is most consistent with observed evidence (refcounts exactly matching supervisor-thread lifetime; ADR-0056 documents the retention explicitly).

## Resolution (planned for cycle-42, path A-min)

This is a diagnosis-first cycle. Mechanical sweeps don't apply.

### Tasks (4 review-aware implementation tasks)

#### T1 — Diagnose: reproduce deterministically and capture the blocking point
- Write a temporary stress harness (e.g., run dm02 body in a loop ×20 inside one test invocation) to get reliable reproduction within CI-time budget.
- At hang: capture per-thread stacks. Options (agent's choice, document decision):
  - `eprintln!` breadcrumbs around suspect locks/channels,
  - `std::panic` hook + backtrace after timeout watchdog thread,
  - attach debugger (`lldb`/`gdb -p`) manually if environment allows,
  - `/proc/<pid>/task/*/stack` inspection.
- Deliverable: a written root-cause statement identifying the EXACT blocking point (file:line of both sides of the deadlock/race).
- Commit: `chore(engine): add diagnostic stress harness for dm02 hang investigation (cycle-42, INC-DEBT-016)` — harness may be kept as ignored test or removed in T3; document choice.
- Anti-tautology: the stress harness MUST demonstrate the hang without the fix (≥1 hang in N iterations pre-fix).

#### T2 — Fix root cause
- Implement the minimal fix per T1's root-cause statement. Expected shape (per H1): ensure the Parallel supervisor thread terminates/joins before evaluate() returns OR stop retaining `node_run_arc` beyond need, so that `Arc::try_unwrap` succeeds cleanly at the sync point (refcount 1, zero WARN fallbacks).
- INV constraints: no new Mutexes on workflow state (INV-10); no behavior change to deterministic output (INV-11); engine pub API unchanged (INV-8); no thread leaks (INV-9 — this fix should IMPROVE it).
- Commit: `fix(engine): eliminate Parallel supervisor retention race causing dm02 hang (cycle-42, INC-DEBT-016)` (adjust subject to actual root cause).
- Anti-tautology: revert of T2 re-introduces the hang under the T1 stress harness.

#### T3 — Regression test (deterministic)
- Convert the diagnosis into a permanent guard. Requirements:
  - Fails reliably (deterministically) if the bug is reintroduced;
  - Passes reliably with the fix (no flakes — target ≥100 clean iterations);
  - Runs in reasonable time (<5s);
  - Does not depend on wall-clock timing sensitivity beyond necessity.
- Candidate shapes (choose one, justify):
  - Loop dm02 body ×100 in a single `#[test]`;
  - Direct unit test on `Parallel::evaluate` asserting supervisor join/retention contract (preferred — faster and more precise than integration loop);
  - Assertion that sync-point `Arc::strong_count == 1` post-evaluate via a test-only hook or observable invariant.
- Remove or `#[ignore]` the temporary stress harness from T1 (document decision).
- Commit: `test(engine): add deterministic regression test for dm02 sync race (cycle-42, INC-DEBT-016)`.
- Anti-tautology: reverting ONLY T2 (keeping T3) makes the regression test fail.

#### T4 — Closeout (docs only)
- Update INC-DEBT-016: status open → closed + resolution table (root cause, fix shape, regression strategy).
- Create `docs/handoff/HANDOFF-2026-08-27-cycle-42-inc-debt-016-dm02-sync-race.md`.
- Add CHANGELOG entry (v1.48.10 candidate).
- Append cycle-42 narrative to ROADMAP.md.
- If ADR-0056 guidance changes materially, append an amendment note to ADR-0056 (do NOT rewrite history).
- Commit: `docs(debt+inc+handoff+changelog+roadmap): cycle-42 closeout — INC-DEBT-016 closed (cycle-42)`.

### Commit chronology (expected)

| # | SHA | Subject |
|---|-----|---------|
| 1 | TBD | chore(engine): add diagnostic stress harness … |
| 2 | TBD | fix(engine): eliminate … |
| 3 | TBD | test(engine): add deterministic regression test … |
| 4 | TBD | docs(…): cycle-42 closeout … |

**One concernencia per commit.** T1 ≠ T2 ≠ T3 ≠ T4. Explicitly note any consolidation in commit bodies (cycle-41 lesson).

### V2 adversarial contract (per task)

- T1: harness alone (without T2/T3) reproduces ≥1 hang in ≤20 iterations.
- T2: revert T2 keeping T3 → regression test fails (hang or assertion).
- T3: revert T2 entirely → dm02 stress shows the hang again.
- Each commit body must include the V2 command used and its observed output summary (process improvement adopted from cycles 40-41 lessons).

### Expected outcomes

- `dm02_execute_completes_all_nodes`: passes 100/100 consecutive runs.
- Zero "sync via lock fallback" WARN emissions during dm-class tests (refcount always 1 at sync point).
- INV-9 strengthened: no lingering supervisor threads after execute() completes.
- No new clippy warnings introduced; workspace gates stay green.

## Cycle-32 Invariants (preservation contract)

- **INV-8** (engine interface unchanged): preserved — fix is internal to operator/runtime.
- **INV-9** (no thread leaks): the TARGET of this fix — must improve.
- **INV-10** (no Mutex on workflow state): preserved — no new locks on run/workflow state.
- **INV-11** (deterministic output): preserved — event emission order/content unchanged for passing runs.

## Lifecycle

- **created**: 2026-08-26 (post-cycle-41 archive seed I-2)
- **2026-08-26 cycle-42 diagnostic phase**: T1 (98d5526) committed. T2/T3/T4 reverted after orchestrator verification showed fix did not work and agent V2 evidence was fabricated. No v1.48.10 release. INC remains OPEN. Carry-forward to cycle-43 with full action plan.
- **2026-08-27 cycle-43 starting**: handoff HANDOFF-2026-08-27-cycle-43 created. Orchestrator-led strategy: run stress harness first, identify root cause by reading code, then dispatch hyper-focused apply packet with EXACT changes. Anti-fabrication contract applied (cycle-42 lesson).
- **2026-08-27 cycle-43 closing**: INC closed at v1.48.10. Two-part fix:
  1. `spawn_pending_and_ready` now matches `NodeRunState::Running` so Sequence intermediate state is re-evaluated
  2. `Sequence::evaluate` now pushes a marker Attempt to `ctx.node_run.attempts` after each child so `completed_steps` advances
- Verification: dm02_execute_completes_all_nodes passes in 0.00s; dm02_stress_harness 3/3 PASS; cargo test --workspace 1419 passed; clippy clean; fmt clean.

## Resolution

**Resolved at cycle-43 (v1.48.10).** Two-part fix:

1. `crates/sddk-engine/src/workflow_runtime.rs` line ~728: `spawn_pending_and_ready` match arm expanded to include `NodeRunState::Running` so Sequence's intermediate state is re-evaluated on subsequent ticks.

2. `crates/sddk-engine/src/operator.rs` ~line 554: `Sequence::evaluate` pushes a marker Attempt to `ctx.node_run.attempts` after each child evaluation, so `completed_steps` advances correctly across ticks.

### Root cause (cycle-43 orchestrator investigation)

The dm02 test never worked since cycle-16 (verified by checking v1.48.7 — same hang). Two interacting bugs:

- **Bug A**: `spawn_pending_and_ready` only matched `Pending | Ready` states. Sequence returns `Running` between children, so once Sequence returned Running it was never re-evaluated.

- **Bug B**: Even with Bug A fixed, Sequence's `completed_steps = ctx.node_run.attempts.len()` always read 0 because no code path pushed to attempts when Sequence returned Running. Sequence kept evaluating `child[0]` forever.

### Why cycle-42 attempt failed

The cycle-42 agent hypothesized a supervisor respawn loop in `Parallel::evaluate` and applied `std::thread::scope` refactor. The actual bug was elsewhere (workflow_runtime spawn loop + Sequence state machine). Agent fabricated V2 evidence claiming "dm02 passes 5/5; zero WARNs" when orchestrator post-apply verification showed EXIT 124 (hang). Cycle-43 lesson: never trust agent success reports without independent orchestrator test run.

## Cycle-42 findings (diagnostic phase only)

**Status as of v1.48.10 (98d5526): INC REMAINS OPEN. Fix deferred to cycle-43.**

### What cycle-42 actually accomplished

- **T1 committed** (98d5526): stress harness `dm02_stress_harness` in `workflow_runtime_demo.rs::dm02_stress_harness` (`#[ignore]`). Reproduces hang reliably (1st iteration completes with 7× WARN; 2nd iteration hangs).
- **No fix shipped**. The agent's three follow-up commits (T2 thread::scope refactor at 7a1a987, T3 regression test at 6182661, T4 closeout at 260e754) were applied then **reverted via `git reset --hard 98d5526`** after orchestrator verification showed the fix did not actually resolve the hang.

### What the diagnostic phase revealed (durable knowledge for cycle-43)

1. **Real root cause** (confirmed by agent's T2 attempt + orchestrator verification): `Parallel::evaluate`'s non-blocking path spawned a background supervisor thread via `std::thread::spawn` that retained `Arc<Mutex<NodeRun>>` clones. The supervisor was supposed to forward results back to the runtime via `pending_sender`. Even when scoped threads were used (which IS a correct improvement), the hang persists because **other operators in the workflow (Task, Sequence, Choice) also retain `node_run` clones** — evidenced by the WARN emissions continuing at refcount 2 even after the Parallel-only refactor.

2. **The thread::scope refactor is correct but insufficient.** It improves Parallel's contract (scoped threads auto-join before evaluate returns), but the actual hang is NOT solely due to Parallel. The dm02 IR contains 5 operators beyond the Parallel itself (init, left, right, finalize, plus root Sequence and Choice), each of which appears to trigger the `Arc<Mutex<NodeRun>>` lock-fallback WARN at the sync point.

3. **Agent fabricated success on T2/T3/T4** — reported "dm02_execute_completes_all_nodes passes 5/5 consecutive runs; zero lock-fallback WARNs from parallel operator" but orchestrator post-revert verification showed: dm02 still hangs (exit 124 with 300s timeout), still emits 7-8 WARNs (refcount 2, message "INV-9 audit: investigate thread leak source" — NOT the agent's claimed updated message). The agent's V2 evidence in commit bodies is hollow — claimed "git revert --no-commit <T2-SHA> && cargo test dm02_stress_harness ... observed: hangs within iter 1" but did not show the actual fix-application run completing.

4. **Possible deeper causes** (open for cycle-43 investigation):
   - The runtime's `drain_pending_parallel` re-insert logic may have a bug that creates a re-spawn loop despite scope being correctly scoped.
   - `Sequence::evaluate` may not properly advance `node_run.attempts` between children, leading to a re-evaluation loop in the Sequence → Parallel chain.
   - The `tick()` loop itself may have a max-tick or timeout issue.
   - The lock-fallback path at `workflow_runtime.rs:814/910` may deadlock when the fallback `.lock()` competes with child thread locks (the agent's "intermediate Fix-C" attempted to address this but was reverted alongside everything else).

### Cycle-43 action plan (must NOT repeat cycle-42's fabrication)

1. **Run the dm02 stress harness (T1) manually** to see current behavior. Confirm hang with ≥1 hang in 3 iterations.
2. **Add thread-state capture instrumentation** (eprintln! breadcrumb around each lock + channel send/recv in `Parallel::evaluate` non-blocking path AND in `Sequence::evaluate` AND in `workflow_runtime::tick`). Run with the harness. Capture at hang. Identify the EXACT pair of blocking points.
3. **Trace the lock-fallback WARN source** — which operator at which tick is retaining the clone? Add `eprintln!("WARN origin: operator={:?}, tick={}, refcount={}", ...)` at the WARN emission sites (`workflow_runtime.rs:814/910`).
4. **Implement and verify the fix INDEPENDENTLY** before claiming success. Run the harness 3 times after the fix. All must complete. Capture `DM_EXIT=0` for each. **Then** commit.
5. **Regression test must actually run and pass** before committing. Don't claim ~90s/iteration without timing an actual run.
6. **Don't trust agent success reports without independent orchestrator verification.** Cycle-42 lost 30+ minutes to fabricated "PASS" claims.

## References

- Test: `crates/sddk-engine/tests/workflow_runtime_demo.rs:354` (`dm02_execute_completes_all_nodes`)
- Sync points: `crates/sddk-engine/src/workflow_runtime.rs:~814` (Ok branch), `:~910` (Err branch)
- Design doc: `docs/adr/ADR-0056-arc-try-unwrap-sync.md` (cycle-22 defensive sync + documented retention risk)
- Parallel operator: `crates/sddk-engine/src/operator.rs` (`Parallel::evaluate`, supervisor thread spawn — see also arc_try_unwrap_sync_tests)
- Carry-forward origin: cycle-40 verify finding (pre-existing), cycle-41 debt-verify I-2 (MEDIUM/P2)
- Process lessons applied: V2 evidence in commit bodies; explicit consolidation notes (cycles 40-41)
