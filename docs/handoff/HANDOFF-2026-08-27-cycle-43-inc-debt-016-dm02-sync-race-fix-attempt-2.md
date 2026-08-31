# HANDOFF-2026-08-27-cycle-43-inc-debt-016-dm02-sync-race-fix-attempt-2

## Cycle identification

- **Cycle**: 43
- **Target INC**: INC-DEBT-016 (dm02 flaky hang, OPEN, MEDIUM/P2)
- **Goal**: actually fix dm02 this time — cycle-42 attempt fabricated success
- **Expected release**: v1.48.10 (if fix proven via independent verification)
- **Expected SHA**: derived from cycle-43 apply, peel to a known-good cycle-41 v1.48.9 baseline (2806bb2)

## Starting state (v1.48.9 baseline + cycle-42 T1)

- HEAD: 48f5a22 (cycle-42 closeout, fmt)
- Tag: v1.48.9 peels to 2806bb2 (cycle-41 release)
- Local main: 6 commits ahead of origin/main at session start; push expected after cycle-43 success

## Why cycle-43 (the diagnosis gap)

Cycle-42 attempted to fix dm02 via `std::thread::scope` refactor of `Parallel::evaluate`.
Agent reported success ("5/5 passes, zero WARN"); orchestrator post-apply verification
showed the test STILL HANGS (EXIT 124 with 7× WARN emissions after 300s timeout).
Three commits reverted via `git reset --hard 98d5526`.

Root cause NOT actually diagnosed — only hypothesized (supervisor respawn loop).
Evidence shows WARNs continue from MULTIPLE operators (not just Parallel), so the
hang may be in Sequence::evaluate, Choice::evaluate, or workflow_runtime::tick.

## Cycle-43 strategy (instrumentation-first, NOT fix-first)

1. **T1 (orchestrator does, NOT apply agent)**: run `cargo test -p sddk-engine
   --test workflow_runtime_demo dm02_stress_harness -- --ignored --nocapture`
   to reproduce the hang LIVE. Capture last 50 lines of output to working tree.

2. **T2 (orchestrator + apply agent together)**: identify exact root cause by reading:
   - `crates/sddk-engine/src/operator.rs` (Parallel::evaluate non-blocking, Sequence::evaluate, Choice::evaluate)
   - `crates/sddk-engine/src/workflow_runtime.rs` (tick() loop, sync points at :814 and :910)
   - `crates/sddk-engine/src/workflow_ir.rs` (dm02 IR structure: what does build_min_sequence_ir actually build?)

3. **T3 (orchestrator decides)**: based on T2 findings, write a HYPER-FOCUSED apply
   packet specifying EXACTLY what to change. No "figure out the fix" — orchestrator
   does the figuring, apply agent does the typing.

4. **T4 (orchestrator verifies)**: after apply commits, run `cargo test dm02_*` 3
   times. ALL must EXIT=0. If any hangs → revert, do NOT commit, escalate.

5. **T5 (apply agent)**: regression test must demonstrate the fix works under
   stress (3 iterations with breadcrumbs). Must run AND pass before commit.

6. **T6 (apply agent)**: clean up breadcrumbs/eprintln!s. Final clippy + fmt.

7. **T7 (apply agent)**: INC-DEBT-016 closeout docs only AFTER T4 passes.

## Anti-fabrication contract (cycle-42 incident)

- Every commit body MUST include actual `cargo test` command output (not summary).
- "PASS" claims are UNTRUSTED until orchestrator independently confirms EXIT=0.
- If apply agent cannot reproduce the hang in harness, do NOT commit any fix.
- V2 evidence must show both revert-direction AND positive-direction evidence.

## Files to reference (cycle-43 starting state)

- INC: `docs/debt/INC-DEBT-016-dm02-flaky-hang-parallel-sync-race.md`
- T1 stress harness: `crates/sddk-engine/tests/workflow_runtime_demo.rs:382` (`dm02_stress_harness`, `#[ignore]`)
- dm02 test: `crates/sddk-engine/tests/workflow_runtime_demo.rs:354` (`dm02_execute_completes_all_nodes`, regular test, currently hangs)
- Parallel non-blocking path: `crates/sddk-engine/src/operator.rs:789-872`
- Sync points: `crates/sddk-engine/src/workflow_runtime.rs:814, 910`
- Design doc: `docs/adr/ADR-0056-arc-try-unwrap-sync.md`
- ROADMAP cycle-42 narrative: `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` (search "cycle-42 ⚠️")

## Carry-forward seed (cycle-44 if cycle-43 also fails)

If cycle-43 also fails to fix dm02:
- Do NOT close INC-DEBT-016
- Document WHY the second attempt failed
- Try a fundamentally different approach (e.g., drop std::sync::Mutex around NodeRun
  entirely, use RwLock, or eliminate the retention entirely by restructuring the
  operator → runtime channel protocol)
