# Implementation Backlog — Ordered

## Milestone M0 — Ratchet
- [ ] Add architecture dependency snapshot.
- [ ] Add allowlisted current violations.
- [ ] `check-arch` rule framework.
- [ ] Add contract tests around current CLI/workflow behavior.

## M1 — Ports & composition
- [ ] Introduce `EventAppender`, `EventReader`.
- [ ] Split workflow persistence ports.
- [ ] Split evidence/context ports.
- [ ] Move concrete storage construction out of app core.
- [ ] Remove production `engine -> storage` dependency.
- [ ] In-memory test adapters.

## M2 — Event foundation
- [ ] Event schema/version registry.
- [ ] Canonical event validator.
- [ ] Correlation/causation helpers.
- [ ] Subscription/reaction dispatcher.
- [ ] Journal projection.

## M3 — Workflow v2
- [ ] Definition schema/parser.
- [ ] WorkflowRun state machine.
- [ ] NodeRun state machine.
- [ ] Attempt model.
- [ ] Scheduler.
- [ ] parallel/join.
- [ ] wait-for-event.
- [ ] retry/timeout/cancel.
- [ ] legacy SDD compiler.

## M4 — OpenCode adapter
- [ ] Host capabilities.
- [ ] Event normalization.
- [ ] usage capture.
- [ ] execute turn.
- [ ] context injection.
- [ ] abort/resume.
- [ ] compatibility tests.

## M5 — Failover/router
- [ ] Failure classifier.
- [ ] Route candidates.
- [ ] Health projection.
- [ ] Circuit breaker behavior.
- [ ] retry policy.
- [ ] route explainability.
- [ ] quota failover acceptance test.

## M6 — Behaviors/supervisor
- [ ] Reaction level classifier.
- [ ] Behavior idempotency.
- [ ] OrchestratorSignal schema.
- [ ] SupervisorDecision schema.
- [ ] cognitive host invocation.
- [ ] delegation policy.

## M7 — Context
- [ ] ContextCapsule schema.
- [ ] selectors.
- [ ] actual-read events.
- [ ] staleness projection.
- [ ] negative knowledge.
- [ ] recovery deltas.

## M8 — Graph/Why
- [ ] typed graph builder.
- [ ] provenance edges.
- [ ] causal queries.
- [ ] rebuild test.
- [ ] `sddk why`.

## M9 — Cockpit
- [ ] snapshot schema.
- [ ] static renderer.
- [ ] overview.
- [ ] journal.
- [ ] timeline.
- [ ] provider health.
- [ ] causal lens.
- [ ] `build/open/watch`.

## M10 — UAT extraction
- [ ] domain split.
- [ ] repositories/ports.
- [ ] campaign/run/defect/retest/signoff.
- [ ] workflow definitions.
- [ ] change-impact integration.

## M11 — Multi-pack proof
- [ ] SDD pack.
- [ ] UAT pack.
- [ ] Incident pack.
- [ ] no kernel domain special-casing audit.

## M12 — Evaluation/forks
- [ ] fork metadata.
- [ ] isolated worktrees.
- [ ] outcome comparison.
- [ ] golden capability fixtures.
- [ ] routing shadow mode.

## M13 — Supply chain
- [ ] SBOM/provenance object types.
- [ ] artifact lifecycle projection.
- [ ] release gate policies.

## M13b — Durable debt remediation

Depends on M2, M3/M3b, M8, M11b and the M13 artifact primitives.

- [ ] Add SDD-pack `DebtReportV2`, tagged proposals and canonical serialization fixtures.
- [ ] Implement `sdd.debt.validate/evaluate` in Rust; callers cannot provide verdicts.
- [ ] Store `debt-report` in CAS and sign report/subject/baseline/policy/evaluator bindings in gate evidence.
- [ ] Register versioned `debt.*` event payloads and idempotent operation IDs.
- [ ] Build replayable incidence, queue, Active Graph and optional Markdown projections.
- [ ] Implement governed risk acceptance, expiry, early resolution and emergency override receipts.
- [ ] Implement deterministic P0-P3 policy with reason codes and deferral budgets.
- [ ] Add `sddk debt validate/evaluate/queue/plan/accept-risk/why` host commands.
- [ ] Bind immutable debt-plan input in Workflow Runtime v2; add an optional legacy cycle-start bridge.
- [ ] Enforce selected debt through ChangeContract and bounded CONVERGE remediation.
- [ ] Add three-run, reopen, alias, expiry, P0, stale-plan and projection-replay acceptance fixtures.
- [ ] Add read-only `sddk artifact inventory`; do not add deletion or compaction.

## M3b — Dynamic workflow core (insert immediately after Workflow v2)

- [ ] WorkflowTemplate vs WorkflowIR contracts.
- [ ] Workflow Compiler and Validator.
- [ ] ExecutionGraphRevision + digest.
- [ ] Map/dynamic fan-out.
- [ ] Join/Race.
- [ ] bounded Loop/Convergence.
- [ ] ExpansionProposal validation/events.
- [ ] graph node/depth/concurrency/budget guards.
- [ ] worktree collision checks.

## M11b — SDD Adaptive & Workflow Laboratory

- [ ] ChangeContract.
- [ ] SHAPE dynamic specialist selection.
- [ ] BUILD WorkGraph/WorkUnits.
- [ ] CONVERGE adaptive verification/remediation.
- [ ] legacy document projections.
- [ ] A-full baseline fixtures.
- [ ] WorkflowExperiment + ablation runner.
- [ ] Cockpit comparison view.

## TEST-BOUNDARY — Test-Tooling Boundary (per ADR-042)

See [ADR-042-TEST-TOOLING-BOUNDARY.md](../03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md) and [TEST-TOOLING-EVIDENCE-AUDIT.md](./TEST-TOOLING-EVIDENCE-AUDIT.md). Ownership policy: [ADR-0069](../../adr/ADR-0069-test-tooling-ownership.md).

### Phase A — Completed audit

- [x] Audit shell contract tests from `643180a` against ADR-0069 ownership cells
- [x] Classify binary behavior as Rust-owned and system orchestration as Shell-owned
- [x] Record findings in TEST-TOOLING-EVIDENCE-AUDIT.md §5 + §6
- **Exit:** Completed through v1.58.0

### Phase B — Completed ownership migration

- [x] Migrate binary-behavior ownership into Rust tests SDDK015-SDDK032
- [x] Retain `tests/test_push_prevention_hook.sh` as the only Shell-owned test
- **Exit:** Completed through v1.58.0

### Phase C — Next work (gated on parity + stability)

- [x] Add `shellcheck` to local CI gate for `tests/test_*.sh`
  + **Parity:** shellcheck installed; gate fails hard si ausente (no WARN + continue)
  + **Stability:** `tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh` linted en cada commit; 5 pre-existing violations cleaned via targeted directives + safe rewrites
- [x] Add Python linter (`ruff`) to local CI gate for `scripts/`
  + **Parity:** ruff instalado; scope = `scripts/` (solo shell scripts — no Python in situ)
  + **Stability:** `golden-dataset/`, `skills/auto-grill/` fuera de scoperuff
  + **n/a — no Python files in `scripts/`**; verified via `git ls-files 'scripts/*.py'` returns empty
- [x] Evaluate ADR-0022 (sddk-testkit, proposed) for adoption or supersession
  + **Parity:** ADR-0022 aceptado; testkit implementado en `crates/sddk-testkit/`
  + **Stability:** `cargo test -p sddk-testkit` pasa; migrate tests from shell a Rust
  + **Accepted:** `Status: accepted (2026-08-29 — sddk-testkit crate implemented and in use)`
- [ ] Consolidate or delete misowned tests after parity evidence
  + **Parity:** shell scripts de binary behavior migrateados a Rust contract tests
  + **Stability:** solo shell-owned tests en `tests/test_*.sh`
- [ ] Remove superseded scaffolding after one stable release cycle
  + **Parity:** scaffolding identificado y documentado en TEST-TOOLING-EVIDENCE-AUDIT.md
  + **Stability:** un release cycle sin regressions confirma supersession
- **Exit:** No redundant test surfaces after parity and a stable release cycle
