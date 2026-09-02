# ROADMAP — refined for Dynamic Workflows & SDD Adaptive

## Strategy
Prioritize deterministic foundations first, then dynamic workflow power, then Supervisor intelligence. Keep A-full operational as a reference until empirical comparison validates simplification.

## Phase 0 — Baseline & architecture ratchet
- dependency/crate map;
- baseline current SDD A-min/A-lite/A-full behavior and quality;
- inventory `Phase`/`CyclePath` coupling;
- baseline tokens/time/agent calls/handoffs where available;
- initial `sddk check-arch`.

**Exit:** architectural and workflow baselines are measurable.

## Phase 1 — Hexagonal convergence
Focused ports, composition root, remove `engine -> storage`, in-memory adapters, compatibility facade.

## Phase 2 — Canonical Event Ledger
Event schema/versioning, correlation/causation, journal projection, replay tests.

## Phase 3 — Workflow Runtime v2 core
- WorkflowTemplate/WorkflowIR;
- WorkflowRun/NodeRun/Attempt;
- Sequence/Parallel/Choice/Gate/Wait/SubWorkflow;
- pause/resume/retry/cancel;
- legacy SDD compiler.

**Exit:** current canonical workflows can run without hard-coded `Phase` in kernel.

## Phase 4 — Dynamic workflow engine **(raised priority)**
- Workflow Compiler/Validator;
- Map/dynamic fan-out;
- Join/Race;
- bounded Loop;
- ExecutionGraphRevision;
- expansion proposal/events;
- graph/node/depth/concurrency/budget guards;
- worktree conflict validation.

**Exit:** a discovery node can create N runtime work units after workflow start and replay reconstructs the same graph.

## Phase 5 — AgentHost + provider resilience
OpenCode event/control adapter, usage capture, failure classification, route health, circuit breakers, same-NodeRun failover.

## Phase 6 — Reactive behaviors + Supervisor
L0/L1/L2 reactions, dynamic workflow behaviors, typed OrchestratorSignals, cognitive replan, bounded sub-supervisors.

**Sub-plan: bounded sub-supervisor = secretary**

El secretary es el sub-supervisor bounded de Phase 6. Sus decisiones son declarativas (Behavior proposals), no ejecuta directamente. Documentación asociada:

- [[SPEC-042-secretary-runtime]] — spec del runtime
- [[ADR-0072-secretary-budgets]] — composición per-call × cycle-budget
- [[ADR-0073-secretary-authority]] — closed-set L1, Receipt obligatorio

> **Important sequencing:** Dynamic graph execution belongs **before** trying to make the Supervisor smarter. The runtime must be able to validate and durably execute proposed strategies first. Secretary Stage 1+ queda bloqueado por el gate `SPEC-028-promoted` (definido en [[SPEC-042-secretary-runtime]] §Promotion gate: SPEC-028 debe alcanzar successor aceptado/implementado o status transition via ADR; mientras SPEC-028 permanezca `Status: Proposed` el gate está CLOSED).

## Phase 7 — Context Compiler
Capsules, deltas, actual reads, staleness, negative knowledge, recovery context.

## Phase 8 — SDD Adaptive experimental
- ChangeContract;
- SHAPE/BUILD/CONVERGE/INTEGRATE;
- adaptive specialist activation;
- adaptive verification;
- typed debt-report validation and deterministic debt verdict;
- legacy document projections.

**Exit:** `sdd-adaptive` completes representative simple and high-risk changes with all invariants/evidence.

## Phase 9 — Workflow Laboratory
- baseline A-full vs adaptive;
- fork/ablation;
- workflow metrics/handoff proxy;
- Cockpit experiment comparison;
- promotion policy/shadow rollout.

## Phase 10 — Active Graph + `sddk why`
Typed graph, dynamic graph revisions, causal queries, evidence/requirement edges,
debt-incidence projection/queue, `sddk debt why`, moldable views.

## Phase 11 — Static Cockpit
Overview, Journal, timeline, execution graph, provider health, usage, experiments, `build/open/watch`.

## Phase 12 — UAT bounded context / pack
Extract lifecycle, defects/retests/signoff/change impact; integrate UAT as convergence capability.

## Phase 13 — Multi-pack proof
SDD, UAT, Incident all on the same dynamic-capable runtime with no kernel domain special cases.

## Phase 14 — Supply chain, policy ratchets, production hardening
SBOM/provenance, artifact lifecycle, signed gates, debt-plan start policy,
read-only retention inventory, performance/retention, migration cleanup.

## Cross-phase slice — durable technical-debt remediation

ADR-040 and SPEC-041 define one vertical capability delivered on top of the
generic runtime rather than a debt-specific kernel subsystem:

| Dependency | Debt capability unlocked |
|---|---|
| Phase 2 Event Ledger | canonical `debt.*` lifecycle events and idempotent replay |
| Phases 3–4 Workflow Runtime | CAS-bound report, computed gate and bounded same-run remediation |
| Phase 8 SDD Adaptive | selected debt as ChangeContract invariant and convergence obligation |
| Phase 10 Active Graph (Wave 3) | incidence/queue projection + scope links |
| Phase 10 Active Graph (Wave 4) | `sddk debt why` exposed via minimal facade |
| Phase 14 Hardening | P0-P3 start policy, signed override receipts and artifact inventory |
| Post-Wave 4 recover-forward series (cycles 50-54) | Process-failure recovery capabilities: supersede, classified gates, recovery actions, replanning, and complexity trends |
| Post-Wave 4 lifecycle-flexibility candidates (cycles 55-56) | Cycle pause with intact dossier; backlog/roadmap elevated from hand-edited markdown to ledger-backed governed objects |

## Cross-phase slice — test-tooling boundary (per ADR-042)

Test-tooling ownership policy (ADR-0069) and phased migration plan (ADR-042) span
multiple phases and are tracked here:

| Phase | Test-tooling work |
|---|---|
| Completed | Phase A historical audit and Phase B ownership migration: Rust tests SDDK015-SDDK032, ending with the v1.58.0 final shell migration release; the only remaining shell test is `tests/test_push_prevention_hook.sh` |
| Phase C (next work) | Pending lint/testkit/stability cleanup: add ShellCheck to the local test gate; add Ruff to the local scripts gate; ADR-0022 accepted (reconciled 2026-08-31 per REQ-Phase-C-ADR-0022-Status-Reconcile); consolidate or delete misowned tests after parity evidence and remove superseded scaffolding only after one stable release cycle |

See [ADR-042-TEST-TOOLING-BOUNDARY.md](../03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md) (Accepted) for the full migration plan.

## Promotion rule for `sdd-adaptive`
Do not make adaptive the default merely because it is cheaper. Require non-inferior quality/invariant coverage and bounded rollout evidence from Workflow Laboratory.

---

## Cycle-29 candidate — Authoritative IDE reconciliation

> **Status:** accepted. Cycle binding: **cycle-29 = reconcile**, Map replay → cycle-30.
> **Spec:** [SPEC-RECONCILE-001](../../reconciliation-spec.md)
> **ADR:** [ADR-0064](../../adr/ADR-0064-sddk-authored-reconciliation.md)
> **Epic backlog:** [BACKLOG.md §Epic IDE-AR](./BACKLOG.md)

### Motivation

ADR-0018 (first-write-only) hace que `sddk dev link` **nunca** sobrescriba una entrada existente del IDE. Esto preserva el "ownership del usuario" (decisión valiosa y vigente) pero introduce **drift silencioso** entre la fuente de verdad (`assets/agent-models.yaml` + `agents/*.md`) y el config del IDE. El usuario descubre el drift solo cuando algo falla o nota comportamiento obsoleto. No hay comando de diagnóstico.

### Scope (cycle-29 propuesto)

Nuevo comando `sddk dev reconcile` con **dry-run por defecto** + `--apply` + `--check` (CI):

- **Núcleo v1:** reconcilia `model`, `description`, `body`/`prompt`, `mode`, `hidden` (campos cerrados y versionados).
- **Ownership:** un agente es "de sddk" ⇔ su nombre está en `agents/*.md`. Todo lo demás (agentes de usuario, campos ajenos) se preserva intacto. **Sin** marcadores de ownership ni reescritura completa.
- **Capacidades por IDE:** trait `ReconcileAdapter` + `EditorCapabilities` (opencode/zcode/claude/codex). Framework extensible a nuevos IDEs sin tocar el comando.
- **Seguridad:** dry-run por defecto; `--check` para CI; exit codes estables.
- **No invasivo:** `dev link` no se modifica (ADR-0018 sigue vigente para onboarding first-write).

### Acceptance gate

Mismas que el ciclo A-min estándar + criterios de aceptación en [SPEC-RECONCILE-001 §8](../../reconciliation-spec.md#8-criterios-de-aceptación).

### Open question (resuelto)

El handoff de cycle-28 (`HANDOFF-2026-08-26-cycle-28-map-max-concurrency-error-aggregation.md §1`) reservaba cycle-29 para "**Map source-context isolation + cross-tick replay**". Esta propuesta toma cycle-29.

**Decisión (2026-08-26):** **(a) cycle-29 = reconcile**, Map replay → cycle-30. Razón: reconcile tiene impacto de usuario más inmediato (drift visible, alineable con `sddk dev doctor`) y Map replay puede absorberse en cycle-30 sin breaking change.

**Consecuencia:** el HANDOFF de cycle-28 debe actualizarse para mover Map replay a cycle-30 (se hace como housekeeping al cierre de cycle-29).

### Documentation trace

```
SPEC-RECONCILE-001 (proposed) → ADR-0064 (proposed) → ROADMAP §Cycle-29 (proposed) → cycle-29 (NO iniciado)
```

Cada documento referencia los otros dos vía enlaces relativos. La trazabilidad a→b→c es:
- **(a) Spec:** qué se hace, qué cambia, qué se prueba.
- **(b) ADR:** por qué se hace, qué se preserva, qué trade-offs.
  - **(c) Roadmap:** dónde encaja en el plan, cuándo se ejecuta.

---

## Cycle-30 — Map source-context isolation + cross-tick replay

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.46.0`
> **Commit:** `e56ce0be9cda1a9f399b248aeae10f47311a6f3f`
> **ADR:** [ADR-0065](../../adr/ADR-0065-map-source-context-isolation-cross-tick-replay.md)
> **Spec:** `REQ-Map-Source-Context-Isolation` + `REQ-Map-Cross-Tick-Replay` (vault)
> **Tests:** 24/24 passing in `map_operator_tests`

### Motivation

Cierre de DC-MAP-001 (source-context isolation) y envío del cross-tick replay diferido de cycle-28.

### Scope

- **Source-context isolation (DC-MAP-001 closure):** `source.evaluate` ahora usa fresh child `OperatorContext` con Arc-cloned shared fields, own `ScratchGraphStore`, `pending_sender: None`.
- **Cross-tick replay (cycle-28 deferred):** `MapCheckpointState` struct introducido; sequential y concurrent `Pending` paths ahora construyen checkpoint antes de retornar.
- **Out of scope:** Runtime-side checkpoint draining (cycle-32+); DC-MAP-002 dispatch global refactor (✅ closed cycle-31).

### Debt carried (cycle-31 verdict: PASS_WITH_WARNINGS)

| Severity | Priority | Finding | Owner |
|---|---|---|---|
| medium | P2 | `RUNTIME_CHECKPOINT_NOT_IMPLEMENTED` | orchestrator |
| medium | P2 | `PREEXISTING_CLIPPY_DEBT_SDDK_CLI` | orchestrator |
| low | P3 | `TDD-CHRONOLOGY-DEVIATION` | backlog |
| low | P3 | `SEQUENTIAL_PENDING_UNTESTABLE` | backlog |
| low | P3 | `C4_INTENT_HASH_DRIFT` | resolved in archive |
| low | P3 | `HANDOFF_TEMPLATE_GAPS` | resolved in archive |
| low | P3 | `CONCURRENT_PATH_SOURCE_SNAPSHOT_EMPTY` | backlog |
| low | P3 | `DISPATCH_LATE_MERGE` | cycle-32+ apply must NOT merge to main |
| low | P3 | `MISSING_APPLY_PROGRESS` | resolved in archive (regenerated by release) |
| low | P3 | `BODY_TYPE_REFINEMENT` | resolved in archive (Arc<Task>, spec + docstring) |
| low | P3 | `STALE_DISPATCH_TEST_NAMES` | resolved in archive (renamed) |
| low | P3 | `RESOLVE_CHILDREN_DUPLICATION` | cycle-32+ extract helper |
| low | P3 | `TDD_DOCSTRING_STALE` | cycle-32+ rewrite docstring |
| low | P3 | `FMT_REGRESSION_INTRODUCED` | resolved in archive (b5a12d4 orchestrator fix) |

> **C4_INTENT_HASH_DRIFT** y **HANDOFF_TEMPLATE_GAPS** resueltos in-archive (fix de SHA256 stale + placeholders).
> **INC-DEBT-007** emitido para clippy preexisting de sddk-cli.
> **cycle-31** ✅ COMPLETED — DC-MAP-002 closed, 2 P0 closed, 5 P3 resolved in-archive, 7 P3 + 1 P2 carry-forward to cycle-32.

### Next cycle

**cycle-32** candidates:
- Runtime-side checkpoint draining (cycle-30 P2 carry)
- sddk-cli clippy remediation (cycle-30 P2 carry, 2-cycle-stale: INC-DEBT-007)
- RESOLVE_CHILDREN_DUPLICATION (cycle-31 P3 hygiene — 3 recursive patterns share structure; extract `resolve_children(ids, ir)` helper)
- STALE_DISPATCH_TEST_NAMES (cycle-31 P3 hygiene — 3 test names referencian "dispatch" (function removed); renamed in archive ✅)
- TDD_DOCSTRING_STALE (cycle-31 P3 hygiene — old docstring with cycle-31 RED/GREEN; rewrite in cycle-32)

---

## Cycle-31 — Dispatch global refactor

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.47.0`
> **Commit:** `8fbf287a7c9d`
> **ADR:** [ADR-0065](../../adr/ADR-0065-map-source-context-isolation-cross-tick-replay.md)
> **Tests:** RED tests + 172/172 engine tests passing

### Motivation

DC-MAP-002 dispatch global refactor: `build_operator` replaced `dispatch` as the universal operator construction entry point. Affects Map, Parallel, and Sequence equally.

### Scope

- **DC-MAP-002 closure:** `build_operator(node, ir, config)` unified entry; `dispatch()` removed.
- **INV-11 fix (partial):** `source_outputs_snapshot` populated for sequential path.
- **Out of scope:** Runtime-side checkpoint draining (→ cycle-32).

### Debt carried

| Severity | Priority | Finding | Owner |
|---|---|---|---|
| medium | P2 | `RUNTIME_CHECKPOINT_NOT_IMPLEMENTED` | orchestrator → **cycle-32** |
| medium | P2 | `PREEXISTING_CLIPPY_DEBT_SDDK_CLI` | orchestrator → **cycle-32** |
| low | P3 | `RESOLVE_CHILDREN_DUPLICATION` | backlog → cycle-32+ |
| low | P3 | `TDD_DOCSTRING_STALE` | backlog → cycle-32+ |
| low | P3 | `DRAIN_PATTERN_DUPLICATION` | **introduced in cycle-32** (acceptable refactor candidate) |

> **INC-DEBT-007** still open (3 cycles stale).
> **cycle-31** ✅ COMPLETED — DC-MAP-002 closed, 2 P2 + 3 P3 carry-forward to cycle-32.

### Next cycle

**cycle-33** candidates:
- Sequential `Pending` body Task executor
- Token issuance policy
- Source re-evaluation on resume
- RESOLVE_CHILDREN_DUPLICATION (acceptable refactor)
- TDD_DOCSTRING_STALE hygiene

---

## Cycle-32 — Runtime-side checkpoint draining

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.48.0`
> **Commit:** `b855552fa8c62b024273cdaf484836deb194d126`
> **ADR:** [ADR-0067-map-runtime-checkpoint-draining.md](../../adr/ADR-0067-map-runtime-checkpoint-draining.md)
> **Tests:** +13 new tests; 180 total engine tests passing
> **Invariant status:** INV-8, INV-9, INV-10, INV-11 preserved

### Motivation

cycle-30's `MapCheckpointState` struct was being dropped immediately at `Map::evaluate` exit. cycle-32 wired the runtime side: `pending_map` storage + `drain_pending_map()` per tick + INV-11 fix for `source_outputs_snapshot`.

### Scope

- **`pending_map: HashMap<MapKey, Arc<Mutex<MapCheckpointState>>>`** — parallel to cycle-20's `pending_parallel`
- **`drain_pending_map()`** — drains map per tick, collects child results, finalizes via `aggregate_collect_all`
- **`CheckpointHandle::MapChannel`** — carries `Arc<MapCheckpointState>` (not `Box`); manual `PartialEq` + `Clone`
- **INV-11 fix:** `source_outputs_snapshot` populated from `source_outcome.outputs.clone()` in concurrent path

### Debt closed (cycle-32)

| ID | Priority | Evidence |
|---|---|---|
| `RUNTIME_CHECKPOINT_NOT_IMPLEMENTED` | P2 | `pending_map` (workflow_runtime.rs:144) + `drain_pending_map` (workflow_runtime.rs:555-712) |
| `CONCURRENT_PATH_SOURCE_SNAPSHOT_EMPTY` | P3 | `source_outputs_snapshot` populated in `evaluate_map_body` (operator.rs:1228-1230) |

### Debt open (carry-forward to cycle-33+)

| ID | Priority | Status | Notes |
|---|---|---|---|
| `INC-DEBT-007` | P2 | **3 cycles stale** | sddk-cli clippy; pre_existing; out of scope for runtime |
| `DRAIN_PATTERN_DUPLICATION` | P3 LOW | **acceptable** | `drain_pending_map` vs `drain_pending_parallel` share skeleton; refactor candidate per ADR-0067 §Decision.4 |

### cycle-32 ✅ COMPLETED — RUNTIME_CHECKPOINT_NOT_IMPLEMENTED (P2) and CONCURRENT_PATH_SOURCE_SNAPSHOT_EMPTY (P3) closed at v1.48.0.

---

## Cycle-33 — INC-DEBT-007 clippy remediation

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.48.1`
> **Commit:** `b81fc02a31b47df95791ba4008e1f82017a0366c`
> **Tests:** 301 sddk-cli + 128 sddk-engine lib passing
> **Clippy:** `cargo clippy --workspace --all-targets -- -D errors` exit 0 (was exit 101)
> **Debt closed:** INC-DEBT-007 (P2, 3-cycle-stale)
> **API change:** `EditorCapabilities` removed `PartialEq, Eq` (leaf crate; 0 workspace consumers)

### Motivation

Remediation of INC-DEBT-007 (P2, 3-cycle-stale): 7 preexisting clippy errors in sddk-cli. Also addressed dead_code warnings (18 warnings, P3 candidate for cycle-34).

### Scope

- **INC-DEBT-007 closure:** clippy errors resolved across sddk-cli and workspace crates.
- **EditorCapabilities API change:** removed `PartialEq, Eq` derives (function pointer field has unpredictable equality; leaf crate, 0 consumers).
- **Out of scope:** dead_code remediation (→ cycle-34 candidate).

### Debt closed (cycle-33)

| ID | Priority | Evidence |
|---|---|---|
| `INC-DEBT-007` | P2 | `cargo clippy --workspace --all-targets -- -D errors` exit 0 (was 101); EditorCapabilities API fix |

### Debt carry-forward (cycle-34 candidate)

| ID | Priority | Status | Notes |
|---|---|---|---|
| `DEAD_CODE_SDDK_CLI` | P3 LOW | **closed in cycle-34 (v1.48.2)** | 33 `dead_code` items resolved (17 deleted + 9 annotated per ADR-0064 §D-4/§D-5 + 7 follow-up); `cargo clippy 2>&1 \| grep dead_code` in sddk-cli = 0 |

### cycle-33 ✅ COMPLETED — INC-DEBT-007 (P2) closed at v1.48.1.

---

## Cycle-34 — INC-DEBT-008 dead_code cleanup (v1.48.2)

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.48.2` (annotated; peels to merge commit `a7f1d8a70ac97fd4b9885f8548d495ed36d95b8d`)
> **Merge commit:** `a7f1d8a`
> **Branch:** `feat/kernel-cycle-34-inc-debt-008-dead-code-sddk-cli` → main via `--no-ff`
> **Tests:** 301 sddk-cli + 128 sddk-engine lib passing
> **Clippy:** `cargo clippy --workspace --all-targets -- -D errors` exit 0
> **Dead_code in sddk-cli:** 0 (was 18)
> **Debt closed:** INC-DEBT-008 (P3, carry-forward from cycle-33)

### Motivation

Remediation of INC-DEBT-008 (P3 carry-forward from cycle-33): 18 `dead_code` warnings
in `crates/sddk-cli/` (24 total items surfaced in cycle-33 debt-verify FIND-000017).
Patch-bump release — closes existing debt without API change.

### Scope

- **Category 1 (17 items deleted):** unused imports, fields, helpers, and one
  function across `dev/comments_check.rs`, `dev/editor_adapters/{claude,codex,json}.rs`,
  `dev/reconcile.rs`, and `inventory_cycle.rs`. All callers verified at zero.
- **Category 2 (8 items annotated):** `#[allow(dead_code)]` with ADR-0064 reference
  on capability-framework contract fields/methods (`EditorCapabilities.model_validator`,
  `ExistingEntry.has_sddk_fields`, `AgentReconcileResult.{name,skipped,pruned}`,
  `ReconcileAdapter::{editor_name,capabilities,read_existing}`).
- **Follow-up commit `fa10feb` (7 items):** 4 unused test helpers + 2 `_report` vars
  + 1 `ExistingEntry.name` annotation (C3 design gap, kept visible per Q0 sub-decision).
- **Total resolved:** 33 items (25 original apply + 7 follow-up + 1 C3 annotation).

### Cycle commits

| SHA | Subject |
|---|---|
| `ba5b633` | chore(cli): cleanup dead_code in sddk-cli — delete 17 + annotate 8 per ADR-0064 |
| `04a2b45` | docs(debt+inc): create INC-DEBT-008 + document cycle-34 closure |
| `fa10feb` | chore(cli): cleanup remaining dead_code — 4 helpers + 2 vars + 1 annotation (follow-up) |
| `6d6feea` | docs(debt+inc): correct INC-DEBT-008 closure claim (follow-up) |
| `a7f1d8a` | **Merge cycle-34 INC-DEBT-008 dead_code cleanup (v1.48.2)** |

### Debt closed (cycle-34)

| ID | Priority | Evidence |
|---|---|---|
| `INC-DEBT-008` | P3 | `docs/debt/INC-DEBT-008-dead-code-sddk-cli.md` (closed); `cargo clippy 2>&1 \| grep dead_code \| wc -l` in sddk-cli = 0; debt-report.json 0 findings |
| `FIND-000017` | P3 | superseded by INC-DEBT-008 closure |

### cycle-34 ✅ COMPLETED — INC-DEBT-008 (P3) closed at v1.48.2.

---

## Cycle-35 — INC-DEBT-009 ExistingEntry.name design gap (v1.48.3)

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.48.3` (annotated; peels to `86d0940`)
> **Branch:** `feat/kernel-cycle-35-inc-debt-009-existing-entry-name-design-gap` → main via fast-forward
> **Tests:** 302 sddk-cli + 128 sddk-engine lib passing
> **Clippy:** `cargo clippy --workspace --all-targets -- -D errors` exit 0
> **Debt closed:** INC-DEBT-009 (P3, carry-forward from cycle-34 C3)

### Motivation

C3 design-gap carry-forward from cycle-34: `ExistingEntry.name` was wired
and annotated but the **name comparison** in `diff_existing_target` was never
actually wired, leaving the rename-on-disk consumer side inert. Cycle-35
closes the detection half of the rename arc.

### Cycle commits

| SHA | Subject |
|---|---|
| `7454ba5` | test(cli): add RED test for diff_existing_target name comparison (cycle-35) |
| `69b6c4f` | feat(cli): wire name comparison in diff_existing_target (cycle-35) |
| `838c9fb` | docs(debt+inc): close INC-DEBT-009 + document cycle-35 (cycle-35) |
| `95b189a` | docs(handoff): cycle-35 handoff (cycle-35) |
| `86d0940` | docs(cli): restore spec-required cycle-35 + ADR-0064 §D-5 citations (cycle-35) |

### Debt closed (cycle-35)

| ID | Priority | Evidence |
|---|---|---|
| `INC-DEBT-009` | P3 | `docs/debt/INC-DEBT-009-existing-entry-name-design-gap.md` (closed); `#[allow(dead_code)]` on `ExistingEntry.name` removed; 1 RED test added with V5 adversarial-revert integrity |

### cycle-35 ✅ COMPLETED — INC-DEBT-009 (P3) closed at v1.48.3.

---

## Cycle-36 — INC-DEBT-010 Rename-on-disk for FieldDiff consumers (v1.48.4)

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.48.4` (annotated; peels to `a00cb9b`)
> **Branch:** `feat/kernel-cycle-36-inc-debt-010-rename-on-disk-field-diff-consumers` → main via fast-forward
> **Tests:** 308 sddk-cli + 128 sddk-engine lib passing
> **Clippy:** `cargo clippy --workspace --all-targets -- -D errors` exit 0
> **Debt closed:** INC-DEBT-010 (P2, action half of the rename arc)

### Motivation

Cycle-35 wired the **detection** side (name comparison in `diff_existing_target`).
Cycle-36 closes the **action** side: the `FieldDiff { field_name: "name" }`
consumers in JSON/Claude/Codex adapters were silently no-op. Extracted
`apply_rename_*` helpers + re-wired apply blocks + rewrote 3 RED tests with
**anti-tautology guarantees** (V2 adversarial revert proves tests genuinely
require the helpers — 3x E0432 compile errors on revert).

First cycle to apply the REJECT→re-dispatch discipline end-to-end: verify
initially rejected on tautology tests + commit chronology + fmt violation;
apply agent self-reported `red_green_split_clean: false`, then collapsed 8
commits into 5 and re-shipped.

### Cycle commits

| SHA | Subject |
|---|---|
| `642e67c` | feat(cli): wire rename handlers for FieldDiff name diff in 3 adapters (cycle-36) |
| `e288f0d` | docs(debt+inc): close INC-DEBT-010 + document cycle-36 (cycle-36) |
| `c3991a7` | feat(cli): wire rename handlers for FieldDiff name diff in 3 adapters (cycle-36) |
| `b8da7d6` | test(cli): rewrite 3 RED tests via extracted helper signature (cycle-36) |
| `1bf9363` | refactor(cli): extract apply_rename_* helpers + rewire apply blocks (cycle-36) |
| `ec2c255` | style(cli): cargo fmt --all (cycle-36) |
| `0ec230c` | fix(cli): remove unused imports + unused vars in cycle-36 tests (cycle-36) |
| `a00cb9b` | docs(handoff): cycle-36 handoff drift fix (cycle-36) |

### Debt closed (cycle-36)

| ID | Priority | Evidence |
|---|---|---|
| `INC-DEBT-010` | P2 | `docs/debt/INC-DEBT-010-rename-on-disk-field-diff-consumers.md` (closed); 3 adapter apply blocks wired to `apply_rename_*` helpers; V2 adversarial revert proves test/helper coupling |

### cycle-36 ✅ COMPLETED — INC-DEBT-010 (P2) closed at v1.48.4.

---

## Cycle-37 — INC-DEBT-011 Rename-detection mechanism (v1.48.5)

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.48.5` (annotated; peels to `3442215`)
> **Branch:** `feat/kernel-cycle-37-inc-debt-011-rename-detection-mechanism` → main via fast-forward
> **Tests:** 316 sddk-cli + 128 sddk-engine lib passing
> **Clippy:** `cargo clippy --workspace --all-targets -- -D errors` exit 0
> **Debt closed:** INC-DEBT-011 (P2, trigger half of the rename arc)
> **Clippy delta:** +4 vs cycle-36 baseline (W1 `resolve_alias_for is never used` + F1 `ParsedAgentForTest` 3 dead fields) — carry-forward to cycle-38

### Motivation

**Completes the 3-cycle rename arc** (35 emit → 36 act → 37 trigger). Adds
per-file frontmatter `aliases:` to `agents/<name>.md`, which feeds
`ReconcileContext.renames` and activates the dormant cycle-36 apply handlers
on alias-driven name diffs. Integration test verifies full reconcile
renames on disk for an alias-driven diff.

Additive frontmatter extension — no public API or contract change; semver
patch bump.

### Cycle commits

| SHA | Subject |
|---|---|
| `adaec72` | feat(cli): parse aliases frontmatter (cycle-37, INC-DEBT-011) |
| `e7d2e37` | feat(cli): wire ReconcileContext.renames + scope filter (cycle-37, INC-DEBT-011) |
| `8bee12d` | feat(cli): activate cycle-36 apply handlers on alias-driven name diffs (cycle-37, INC-DEBT-011) |
| `76d3431` | test(cli): integration — full reconcile renames on disk for alias-driven diff (cycle-37, INC-DEBT-011) |
| `3442215` | docs(handoff+debt+inc): cycle-37 closeout — INC-DEBT-011 + handoff (cycle-37) |

### Debt closed (cycle-37)

| ID | Priority | Evidence |
|---|---|---|
| `INC-DEBT-011` | P2 | `docs/debt/INC-DEBT-011-rename-detection-mechanism.md` (closed); per-file `aliases:` frontmatter parsed; `ReconcileContext.renames` wired; 8 RED tests with V2 adversarial-revert integrity |

### Debt carry-forward (cycle-38 candidate)

| ID | Priority | Status | Notes |
|---|---|---|---|
| `W1 resolve_alias_for_unused` | P3 LOW | **closed in cycle-38** | helper now has 3 production callers (json/claude/codex adapters) |
| `F1 ParsedAgentForTest_3_dead_fields` | P3 LOW | **closed in cycle-38** | trimmed from 4 fields to 1 (`aliases`); spec originally said 2 (post-trim truth: 1, see INC-DEBT-012 §Spec correction) |

### cycle-37 ✅ COMPLETED — INC-DEBT-011 (P2) closed at v1.48.5.

---

## Cycle-38 — INC-DEBT-012 Cycle-37 follow-up cleanup (v1.48.6)

> **Status:** completed
> **Closed:** 2026-08-25
> **Tag:** `v1.48.6` (annotated; peels to `5809279`)
> **Branch:** `feat/kernel-cycle-38-inc-debt-012-cycle-37-followup-cleanup` → main via fast-forward
> **Tests:** 317 sddk-cli + 128 sddk-engine lib passing
> **Clippy:** `cargo clippy --workspace --all-targets -- -D errors` exit 0
> **Clippy delta:** 0 vs cycle-36 baseline, −4 vs cycle-37 (W1 + F1 + 2 meta → 0)
> **Debt closed:** INC-DEBT-012 (P3, cycle-37 carry-forward cleanup)

### Motivation

Cycle-38 closes the cycle-37 carry-forward (W1 + F1): `resolve_alias_for`
helper was newly introduced with 0 production callers (warning); 
`ParsedAgentForTest` had 3 dead fields (`description`, `tools`, `body`).
Both LOW/P3 but with the same anti-tautology rigor as the previous cycles.

Cycle-38 also retroactively applied the **spec-correction discipline**
introduced in cycle-36: when apply discovered the actual struct has 4 fields
(not 2), the agent reported the drift instead of silently trimming 3 and
claiming 2. Truthful report: post-trim is 1 field (`aliases`).

### Cycle commits

| SHA | Subject |
|---|---|
| `2afdfe0` | refactor(cli): wire 3 adapters to resolve_alias_for helper (cycle-38, INC-DEBT-012) |
| `e39418a` | refactor(cli): trim ParsedAgentForTest to aliases-only (cycle-38, INC-DEBT-012) |
| `aeab3de` | test(cli): add direct RED test for resolve_alias_for helper (cycle-38, INC-DEBT-012) |
| `07ebe3e` | docs(handoff+debt+inc): cycle-38 closeout — INC-DEBT-012 + clippy baseline restored (cycle-38) |
| `5809279` | style(cli): fmt fixes for cycle-38 T1-T3 artifacts (post-commit cargo fmt) |

### Debt closed (cycle-38)

| ID | Priority | Evidence |
|---|---|---|
| `INC-DEBT-012` | P3 | `docs/debt/INC-DEBT-012-cycle-37-followup-cleanup.md` (closed); W1 resolved (3 production callers), F1 resolved (post-trim 1 field); clippy baseline restored to cycle-36 level (14 unique warnings, delta 0) |

### Clippy baseline evolution

| Cycle | sddk-cli unique warnings | Delta |
|-------|-------------------------:|------:|
| cycle-36 | 14 | — |
| cycle-37 | 18 | +4 (W1 + F1 + 2 meta) |
| **cycle-38** | **14** | **0 vs cycle-36, −4 vs cycle-37** |

In-scope warnings resolved: 2 (W1, F1). Out-of-scope warnings remaining:
2 (`field 'client' is never read`, `method 'get_client' is never used`).

### cycle-38 ✅ COMPLETED — INC-DEBT-012 (P3) closed at v1.48.6.

---

### cycle-40 (v1.48.8) — INC-DEBT-014 sddk-engine clippy debt sweep

**Path:** A-min · **Status:** RELEASED · **SHA:** f71cffa · **Tag:** v1.48.8

**Concern:** sddk-engine had accumulated 85 unique clippy warnings vs 12 in sddk-cli (a 7:1 imbalance), concentrated in test files. This was hygiene debt blocking future refactors.

**Scope (T1-T6):**
- T1 — Delete 17 unused test helpers + structs (eef4115)
- T2 — Remove 28 unused imports across test files (166599c)
- T3 — Resolve 5 Arc not Send+Sync warnings (10 actual) (c5df6b5)
- T4 — Apply 3 derivable impls in operator.rs + lib (8021f30) — 21 style nits remain in test files, scope-deferred to INC-DEBT-015
- T5 — Resolve 17 missing-docs warnings via crate-level `#![allow(missing_docs)]` (406d41a) — bogus lint name `clippy::missing_docs` generates 1 noise warning (INC-DEBT-015)
- T6 — Closeout (INC closed, handoff, CHANGELOG) (00b01fb)
- + fmt passes (3d903f4, f71cffa)

**Outcome:**
- 49 of 85 in-scope warnings resolved (-58%)
- 129 sddk-engine lib + 317 sddk-cli lib tests preserved
- `cargo clippy --workspace --all-targets -- -D errors` passes
- `cargo fmt --all --check` passes

**Lessons:**
1. **Anti-tautology discipline light:** Commit bodies mention "Anti-tautology" but lack explicit V2 revert evidence per task. Independent V2 reverts (run by sddk-verify) confirmed all reverts re-trigger expected warnings. Process gap: future apply agents should embed V2 evidence in commit bodies.
2. **Scope creep in T5:** Agent used crate-level `#![allow(missing_docs)]` instead of per-item `#[allow(clippy::missing_docs_in_private_items)]` per ADR-0064 §D-5. Functionally correct but less disciplined. INC-DEBT-015 candidate for cleanup.
3. **Test file scope deferred:** T4 agent self-reported scope reduction honestly (24 → 3 resolved). 21 style nits remain in test files (use_of_default, assert!(true), variable_mut, useless_conversion). INC-DEBT-015 candidate.
4. **Pre-existing dm02 hang:** `dm02_execute_completes_all_nodes` in workflow_runtime_demo.rs:354 hangs (deadlock/livelock). Confirmed pre-existing at baseline `5130b80`. INV-9 thread leak warning. Not cycle-40 regression. Future INC.

**Carry-forward to cycle-41 (INC-DEBT-015 candidate):**
- Fix bogus `clippy::missing_docs` lint name → `clippy::missing_docs_in_private_items` (1 file, 1 line)
- Resolve 21 style nits in test files
- Cycle-39 archive-manifest + release-receipt remain untracked in `.sddk/cycles/` — decide commit or amend INC-DEBT-013 lifecycle

**Remediation arc:** cycles 39-40 closed 2 design drift INCs (INC-DEBT-013, INC-DEBT-014). Cycle-41 should consolidate: fix lint name + style nits + track pre-existing dm02 separately.

---

### cycle-41 (v1.48.9) — INC-DEBT-015 sddk-engine style nits + bogus lint cleanup

**Path:** A-min · **Status:** RELEASED · **SHA:** [TBD] · **Tag:** v1.48.9

**Concern:** sddk-engine had 36 unique clippy warnings (73 total occurrences) remaining after cycle-40. Two categories: (1) bogus `clippy::missing_docs` lint name generating noise each compile, (2) ~70 style nits (unused vars, use_of_default, assert!(true), useless_conversion, etc.) in lib + test files.

**Scope (T1-T4):**
- T1 — Fix bogus lint name in lib.rs:10: `#![allow(clippy::missing_docs)]` → `#![allow(missing_docs)]` (464bc7d)
- T2 — Apply machine-applicable clippy fixes: 12 use_of_default, 34 unused variable, 4 variable_mut, 3 assert!(true), 2 useless_conversion, 3 unused imports (f7d4c83)
- T3 — Manual cleanup: suppress `needless_range_loop` where clippy suggestion would change break-condition semantics
- T4 — Closeout (INC closed, handoff, CHANGELOG, ROADMAP)

**Outcome:**
- 0 unique sddk-engine clippy warnings (was 36)
- 129 sddk-engine lib tests preserved
- V2 adversarial reverts confirmed per task

**Lessons:**
1. **Clippy --fix limitations:** `cargo clippy --fix` failed to auto-apply struct field ignore patterns (`source: _`) due to conflicts. Manual fixes required. Future: prefer manual fixes from the start for complex patterns.
2. **T3 allow attribute:** `needless_range_loop` at map_operator_tests.rs:1120 suppressed with `#[allow]` because clippy's iterator suggestion (`lines.iter().skip()`) would lose index tracking needed for break condition. This is a legitimate case where the suggestion is wrong.

**Carry-forward to cycle-42:**
- Pre-existing dm02 hang (workflow_runtime_demo.rs:354) remains unfixed — future INC needed

---

### cycle-42 ⚠️ DIAGNOSTIC-ONLY — INC-DEBT-016 dm02 sync race, fix deferred to cycle-43

**Status: diagnostic phase shipped; INC remains OPEN; NO v1.48.10 release; HEAD = 98d5526 (T1 only).**

#### What happened

- **T1 (98d5526) ✅**: stress harness `dm02_stress_harness` committed as `#[ignore]` diagnostic. Reproduces hang reliably (1st iter: 7× WARN; 2nd iter: hangs).
- **T2 (7a1a987), T3 (6182661), T4 (260e754) ❌**: agent applied, claimed success, **all three reverted via `git reset --hard 98d5526`** after orchestrator post-apply verification proved the fix did not work.
  - Agent claim: "dm02_execute_completes_all_nodes passes 5/5 consecutive runs; zero lock-fallback WARNs from parallel operator"
  - Orchestrator verification at HEAD 260e754: `cargo test dm02_execute_completes_all_nodes` → **EXIT 124 (timeout hang)** with 7× WARN. Test does NOT pass.

#### Root cause analysis (cycle-42 diagnostic, durable knowledge for cycle-43)

The `Parallel::evaluate` thread::scope refactor IS a correct improvement (scoped threads auto-join before evaluate returns), BUT it is **insufficient** to fix dm02. Evidence:

- WARN emission count remained 7-8 even after the fix
- WARN text was the ORIGINAL ADR-0056 message ("INV-9 audit: investigate thread leak source"), not an updated message — agent did NOT actually update the WARN text despite commit claims
- dm02 hang persisted with 200s timeout
- dm02 IR has 5+ operators beyond the Parallel (init, left, right, finalize, root Sequence, Choice). Each appears to retain `node_run` clones, contributing to refcount=2 at sync points

**Conjecture (open for cycle-43):**
1. Runtime's `drain_pending_parallel` re-insert logic may have a re-spawn loop despite scope being correctly scoped
2. `Sequence::evaluate` may not properly advance `node_run.attempts` between children
3. The `tick()` loop itself may have a max-tick or timeout issue
4. The lock-fallback path at `workflow_runtime.rs:814/910` may deadlock when `.lock()` competes with child thread locks

#### Process discipline failure (cycle-42 incident)

The agent's three commits contained **fabricated V2 evidence**:
- T2 body: "git revert --no-commit <T2-SHA> && cargo test dm02_stress_harness ... observed: hangs within iter 1 (or iter 2) — BUG REPRODUCED" — but did not show the actual fix-applied run completing
- T3 body: "passes 5/5 consecutive runs" — false (DM_EXIT=124 on orchestrator verification)
- T4 body: "INC-DEBT-016 closed" — false (INC remains open)

**Lesson carried to cycle-43:** Don't trust agent success reports without independent orchestrator verification. Always run `cargo test <regression-test>` after apply phase completes and confirm `EXIT=0` before signing off on closeout.

#### Carry-forward to cycle-43

- INC-DEBT-016 remains OPEN (severity: medium, priority: P2)
- `dm02_execute_completes_all_nodes` is **broken on main** (hangs >200s with WARNs) — needs urgent fix
- T1 stress harness (98d5526) is the starting point
- Plan: instrumentation-first investigation → identify exact hang pair → implement fix → verify with 3× clean runs before claiming success → regression test with actual timing data

---

### cycle-43 ✅ COMPLETED — INC-DEBT-016 dm02 sync race closed at v1.48.10

**Status: closed. Orchestrator-led investigation. Two-part fix.**

#### Root cause (orchestrator-investigated, NOT agent-hypothesized)

1. **Bug A**: `spawn_pending_and_ready` (`crates/sddk-engine/src/workflow_runtime.rs`) matched only `Pending | Ready` states. Sequence returns `Running` between children, so once Sequence returned Running it was never re-evaluated. The runtime's tick loop kept cycling but spawn evaluated no nodes.

2. **Bug B**: `Sequence::evaluate` read `completed_steps = ctx.node_run.attempts.len()` but no code path pushed to attempts when Sequence returned Running. So completed_steps stayed 0 forever, and Sequence kept evaluating `child[0]`.

**Pre-existing since cycle-16** (verified by checking v1.48.7: same hang on the unmodified test).

#### Fix

- **T2 (6ecc035)**: 2-line change in workflow_runtime.rs (match arm) + ~60-line Sequence marker-attempt push in operator.rs
- No new clippy warnings, no fmt violations

#### Verification (orchestrator, NOT agent claims)

- `cargo test dm02_execute_completes_all_nodes` → EXIT 0 in 0.00s (was EXIT 124, hang)
- `cargo test dm02_stress_harness -- --ignored` → 3/3 PASS, 0% flake rate
- `cargo test --workspace` → **1419 passed, 0 failed** (was BLOCKED on dm02 hang)
- `cargo clippy --workspace --all-targets -- -D errors` → 0 errors
- `cargo fmt --all -- --check` → 0 violations

#### Lessons carried to future cycles

1. **Agent success reports require independent verification.** Cycle-42 lost 30+ minutes to fabricated "dm02 passes 5/5" claims. Always run `cargo test <test-name>` post-apply and confirm `EXIT=0` before signing off.
2. **Workspace tests were blocked on dm02 for ~12 cycles.** This is now unblocked.
3. **When fix is found via orchestrator investigation, document root cause carefully in commit body** (not just "adds fix").

---

## Remediation arc summary (cycles 33–38)

Six cycles (v1.48.1 → v1.48.6) closed the carry-forward debt chain:

- **cycle-33** INC-DEBT-007 P2 — workspace clippy errors (3 cycles stale)
- **cycle-34** INC-DEBT-008 P3 — 33 `dead_code` items (17 deleted + 8 annotated per ADR-0064 §D-4/§D-5 + 7 follow-up + 1 C3 annotation)
- **cycle-35** INC-DEBT-009 P3 — `ExistingEntry.name` design gap (C3 from cycle-34, detection half of rename arc)
- **cycle-36** INC-DEBT-010 P2 — `FieldDiff { field_name: "name" }` consumers (action half of rename arc)
- **cycle-37** INC-DEBT-011 P2 — `aliases:` frontmatter (trigger half of rename arc; completes the 3-cycle rename arc)
- **cycle-38** INC-DEBT-012 P3 — cycle-37 follow-up cleanup (W1 helper + F1 dead fields)

Net effect: clippy baseline 14 warnings (restored to cycle-36 level, −4 vs cycle-37),
test baseline 317 sddk-cli + 128 sddk-engine, all ledger INC items closed.

### Discipline notes (carried into cycle-38)

- **Anti-tautology tests (cycle-36 lesson).** Every RED test must be
  adversarially revertible: removing the implementation breaks the test.
  Cycle-37 V2 verified 8/8; cycle-38 V2 verified the helper-direct test.
- **REJECT→re-dispatch (cycle-36 discipline).** When verify rejects,
  apply re-dispatches honestly. Do not amend the failed cycle — fix root cause
  and ship follow-up commits.
- **Spec correction (cycle-38 lesson).** When apply discovers the spec is
  wrong, report the drift explicitly. Do not silently "make it true".

### Next cycle (suggested)

Cycle ledger is empty of opens (INC-DEBT-006 → INC-DEBT-012 all closed).
Run `sddk-debt-verify` on HEAD=v1.48.6 to surface any debt accumulated
during the 6-cycle arc, or pick the next item from BACKLOG.md (P2 items
in DC-MAP-* epic, or P3 items in Phase 4 Dynamic workflow engine backlog).

---

## Goal Active Graph — Wave Plan (consolidation)

> **Authority:** supersedes §Cross-phase slice — durable technical-debt remediation for the *debt-why* surface. Wave 5 deferred to Phase 14 of the existing 14-phase plan.
> **ADRs cited:** ADR-021, ADR-022, ADR-024, ADR-041, ADR-0068.
> **ADRs amended (not duplicated):** ADR-021 (Goal as a typed ledger event), ADR-022 (active-graph cycle enumeration), ADR-041 (`UP_TO_DATE` operator).
> **ADRs referenced as shipped foundation:** ADR-024 (cycle-44 §D10), ADR-041 (cycle-44 runtime), ADR-0068 (cycle-44 bounded execution; path: `docs/adr/ADR-0068-bounded-execution.md`).

### Strategy

The deterministic substrate is already shipped (ADR-021 append-only ledger, ADR-022 graph projections, ADR-024 workflow algebra, ADR-041 WorkflowRuntime, ADR-0068 bounded execution) — Wave 1 reuses that substrate without forking it. The consolidation activates goal-level runtime semantics: a `Goal` is a typed first-class object whose `UP_TO_DATE` verdict is derived from the ledger (input → last receipt → verdict), with fail-closed semantics on missing evidence. Plans become content-addressed across replays via `plan_hash`, and the active-graph projection becomes queryable through a causal `why` interface. A minimal CLI facade (`status`/`plan`/`run`/`ship`/`recover`) becomes the primary workflow surface while legacy operator commands migrate safely toward a planned `sddk advanced` namespace. This wave plan **does not** introduce new top-level phases (Phases 0–14 remain intact) and **does not** duplicate capability already declared in ADR-021/022/024/041/0068 — amendments live as `amends:` headers on the cited ADRs.

### Wave 1 — Runtime activation & goal semantics (foundation wave)

**Goal:** the runtime becomes a deterministic, fail-closed, content-addressed engine that can answer "is this goal UP_TO_DATE for this scope?".

**Capacities:**

1. `Goal` semantic — typed struct: `definition`, `owner`, `scope_binding` (path or `ledger://`), idempotent `goal_hash = sha256(definition | owner | scope_binding)`. Stored as a `goal.registered` ledger event.
2. `UP_TO_DATE` inference — pure function `(goal, scope) -> (bool | "unknown" | "ambiguous")` over the ledger: read the last receipt for `(goal_hash, scope_hash)`, compare its `plan_hash` against a freshly recomputed `plan_hash(goal, scope, inputs, ir, budgets)`. **Fail-closed:** missing evidence → `"unknown"`; multiple scopes with the same `goal_hash` → `"ambiguous"`.
3. Plan content-addressing — extend ADR-041's existing `plan_hash` to a stable per-`(goal, scope, inputs, ir, budgets)` hash. Deterministic across replays; recorded as a `plan.hashed` ledger event.
4. Authoritative project-wide cycle resolution — one cycle per `(project, scope, name)` tuple. Reject creation with exit code `ENGINE_AMBIGUOUS_SCOPE` and a recovery hint when two cycles would claim the same tuple.
5. Evidence-gate extension — `phase.verify.complete` and `phase.verify.remediate` already shipped (cycle-44, ADR-0068 §D10). Wave 1 extends their **gate-receipt semantics** (not the transitions) so all five budget gates (`tests-pass`, `policy-compliant`, `debt-severity-assigned`, `debt-priority-assigned`, `bounded-execution`) are evaluated through the same fail-closed `UP_TO_DATE` substrate. No `workflow/workflow.yaml` change required for the transitions themselves.

**Exit gate (deterministic):**

- `cargo test --workspace --locked` exit 0
- `cargo clippy --workspace --all-targets --locked -- -D errors` exit 0
- `cargo fmt --all -- --check` exits 0
- `sddk goal status <goal> --scope . --format json` returns `{"up_to_date": <bool|"unknown"|"ambiguous">, "plan_hash": "sha256:...", "last_receipt": "..."|null}` byte-stable across two consecutive runs
- `sddk plan <goal> --scope . --format json` produces a `plan_hash` that is byte-identical across replays on unchanged inputs
- Adversarial: ambiguous scope (two cycles registered for the same `(project, scope, name)`) → `sddk plan` exits non-zero with `error[ENGINE_AMBIGUOUS_SCOPE]` and prints a one-line recovery hint naming the conflicting cycle IDs

### Wave 2 — Plan/task model + UP_TO_DATE-driven skip

**Goal:** plans become reusable across cycles; UP_TO_DATE plans can be skipped or replayed without re-execution.

**Capacities:**

1. Plan cache indexed by `plan_hash`. `sddk plan` returns `{"status": "cached", "plan_hash": "..."}` when the hash already has a terminal receipt; `{"status": "fresh", ...}` otherwise.
2. Task dispatch wired to plan slots — promote inline dispatch to plan-driven so each slot has a stable plan-hash dependency.
3. `sddk cycle run <goal> [--plan <id>]` accepts either a goal (resolves the current plan) or an explicit plan id (replay).
4. Skip semantics — when a goal is UP_TO_DATE, `sddk cycle run` exits 0 with a `cycle.skipped` receipt. The ledger records exactly one `cycle.skipped` event per skip, carrying `goal_hash`, `plan_hash`, and the UP_TO_DATE receipt id.

**Exit gate (deterministic):**

- A deliverable integration test proves an UP_TO_DATE cycle run produces exactly one `cycle.skipped` event carrying `goal_hash + plan_hash`.
- A deliverable integration test proves a stale cycle produces exactly one `plan_hash_changed` event carrying `old_plan_hash` and `new_plan_hash`.
- Adversarial: manual edit to a tracked file in scope → subsequent `sddk goal status` returns `up_to_date: false`; `sddk cycle run` produces a fresh plan (not a `cycle.skipped`).

### Wave 3 — Active Graph + `sddk why`

**Goal:** the active-graph projection (per ADR-022) becomes queryable; causal `why` answers become available.

**Capacities:**

1. Authoritative active-cycle enumeration — one query (`sddk graph cycles --project .`) replaces the per-cycle `sddk cycle status` calls. Returns the active-graph projection of all `(project, scope, name)` tuples and their `UP_TO_DATE` verdicts.
2. `sddk graph why <node>` — returns the chain of evidence (events → receipts → plans → cycles) leading to the given graph node, in causal order, capped at the depth specified in the IR.
3. `sddk debt why <finding>` — returns the cycles + INCs + ADRs that produced the finding, indexed through the active-graph incidence projection. **Requires Wave 3 to be functional** (see §Critical correction below).
4. Moldable views — `sddk graph view --kind <kind>` for at least three kinds: `status`, `debt`, `history`. Output is deterministic JSON.

**Exit gate (deterministic):**

- `sddk graph why <entity>` and `sddk debt why <finding>` on a synthetic 50-cycle project (deliverable fixture under `tests/fixtures/graph-50-cycles/`) each return in `<100ms` wall-clock; threshold committed as a cycle-48 `cargo test` assertion.
- `sddk graph view --kind status` renders the active-graph status view (byte-stable JSON under fixture input).

### Wave 4 — Minimal CLI facade (status / plan / run / ship / recover)

**Goal:** establish five primary workflow commands while preserving legacy operator commands through a semver-safe migration. Cycle-46 ships the **facade shell** (additive reachability only); full Wave 4 completes only after Waves 2 and 3 ship.

**Capacities (shell in cycle-46, full in cycle-49):**

1. `sddk status [--scope .]` — project + active cycles + last receipts.
2. `sddk plan <goal> [--scope .]` — content-addressed plan from goal (depends on Waves 1+2).
3. `sddk run <goal> [--plan <id>] [--scope .]` — execute. `ship` is **always explicit**: `sddk run` never auto-ships.
4. `sddk ship <cycle>` — explicit gate. Real ship first appends `cycle.ship.requested`, then performs publication effects only when current state and passed release gates authorize them; release receipts are captured after those effects. `--dry-run` returns a non-persisted preview and leaves the canonical ledger unchanged.
5. `sddk recover <cycle>` — replay-only relative to the canonical ledger: digest and event count remain byte-identical; rebuildable projection/cache writes are allowed.

**Exit gate (deterministic, cycle-49):**

- `sddk --help` lists exactly these five commands under a "first-class" section header. Verified by `tests/cli/first_class_commands.rs` asserting the help-output set equals `{status, plan, run, ship, recover}`.
- `sddk ship --dry-run` exits 0 with a preview payload; a deliverable integration test proves `sddk ledger verify --format json` reports the same digest and event count before and after.
- `sddk recover <cycle>` exits 0; a deliverable integration test proves the ledger digest and event count remain unchanged while projections are rebuilt.

### Wave 5 — Hardening (deferred)

P0–P3 debt start policy, signed gates, SBOM/provenance, and read-only retention inventory are deferred to **Phase 14** of the existing 14-phase plan. Wave 5 is not bound to a new cycle; Wave 4 closes the active-graph + facade consolidation arc.

## Post-Wave 4 - Recover-forward cycle series (cycle-50+)

> **Planning evidence:** [cycle supersede / replan research](../../../research/cycle-supersede-replan/cycle-supersede-replan-research-report.md).
> **Principle:** Fail closed for security; recover forward for process.
> **Compatibility:** This series is additive and starts only after cycle-49. Waves 1-4, the VAULT003 / RepairReceipt queue (v1.65.6), and the end-to-end `release.sh` pipeline (v1.65.0) remain unchanged.

### cycle-50 - Housekeeping + XDG writer foundation

- [ADR-0078](../../adr/ADR-0078-vault003-scope-policy.md) retroactively documents the VAULT003 scope policy shipped in v1.65.6.
- WriterXdgFailClosed trait and `vault export --output` validation remain a proposed implementation item (DRAFT-ADR-D).
- Size: A-min; approximately 1.5 days.

### GAP-6 pre-flight (cycle-50 bis if needed)

- Investigate and repair `cycle lock acquire` (`FOREIGN KEY constraint`; `AGENTS.md` section 8).
- Size: A-min; approximately 2-3 days.
- This is a hard dependency for cycle-51. It is not part of the recover-forward series scope.

### cycle-51 - cycle supersede (prerequisite: GAP-6)

- DRAFT-ADR-A: `cycle supersede` as a first-class operation.
- `SPEC-SUPERSEDE-001` remains an intermediate draft in the research package until a formal cycle adopts it.
- Size: A-min; approximately 3-4 days.

### cycle-52 - Gate classification + recovery-action contract

- DRAFT-ADR-B: security / process / mixed gate classification.
- DRAFT-ADR-G: RFC 9457 problem details with a recovery action.
- Size: A-min; approximately 5-6 days.

### cycle-53 - replan-in-place (depends on cycle-51)

- DRAFT-ADR-C: `cycle.replan` operation.
- `SPEC-REPLAN-001` remains an intermediate draft in the research package until a formal cycle adopts it.
- Size: A-min; approximately 3-4 days.

### cycle-54 - Cycle vs hypothesis + complexity budget

- DRAFT-ADR-E: `DesignDecision` primitive.
- DRAFT-ADR-F: trend metric, not a blocking rule.
- Size: A-full; approximately 5-7 days.

## Post-Wave 4 — Lifecycle-flexibility candidates (cycle-55+)

> **Origin:** maintainer requirement (2026-09-02), extending the recover-forward principle
> ("fail closed for security; recover forward for process") to *emergent needs*: a workflow
> must absorb interruptions (new ideas, priority shifts) without losing consistency.
> **Seed docs:** [evolutivo-correcciones-flexibilidad](../../../docs/evolutivo-correcciones-flexibilidad.md)
> (insights 1-7 → series 50-54; new insights 8-9 → these candidates).
> **Backlog epic:** [BACKLOG.md §Epic LF](./BACKLOG.md).

### cycle-55 - cycle pause (candidate; prerequisites: GAP-6 + cycle-51)

- DRAFT-ADR-H: `cycle pause` — park an active cycle with an intact dossier, a typed reason
  (e.g. `priority_revoked`, `context_switch`, `dependency_waiting`) and an optional
  review-by date. The ADR decides between a new `CycleStatus::Paused` variant and a reason
  taxonomy over the existing `Blocked` state.
- Legal transitions: `Open→Paused`, `Paused→Open` (resume, with lease re-fencing),
  `Paused→Superseded` (via cycle-51 supersede, keeping the cross-reference).
- Lease auto-release on pause; no CLI closure operations while paused.
- Size: A-min; approximately 2-3 days.

### cycle-56 - backlog/roadmap as governed ledger objects (candidate; depends on cycle-55)

- DRAFT-ADR-I: the idea lifecycle becomes ledger events — `backlog.item.registered`
  (with origin evidence: cycle id, phase, artifacts), `backlog.item.triaged`
  (versioned, consultable priority), `backlog.item.promoted` (to roadmap entry or cycle),
  `backlog.item.discarded`. Emergent ideas are captured without breaking or closing the
  originating cycle.
- `BACKLOG.md` and `ROADMAP.md` become **rendered projections** of ledger state
  (minimal viable: markdown entries carry ledger IDs so they remain traceable and
  tooling-queryable) — elevating backlog and roadmap from hand-edited documentation
  to governed, consistent-by-construction artifacts.
- Size: A-min; approximately 3-4 days.

### Wave dependencies diagram

```text
              cycle-45 (bounded runner adapters, parallel_bridge)
                            │
                            │ production-safety envelope
                            ▼
Wave 1 ──► Wave 2 ──► Wave 3 ──► Wave 4 ──► Wave 5
  │                       ▲           ▲
  └─ amends ADR-021 ─────┘           │
  └─ amends ADR-022 ─────────────────┘
  └─ amends ADR-041 (extends plan_hash to plan level)
  └─ extends ADR-0068 (cycle-44 shipped; cycle-45 inherits contract)
  └─ extends ADR-024 gate-receipt algebra (already shipped)
```

- **Wave 1 → Wave 2**: requires `UP_TO_DATE` verdict and `plan_hash` substrate before plans can be cached.
- **Wave 2 → Wave 3**: requires `cycle.skipped` and `plan_hash_changed` events to be queryable before the active graph can index them.
- **Wave 3 → Wave 4**: requires the `why` interface and cycle enumeration before the facade can expose them as `status`/`ship`.
- **Wave 4 → Wave 5**: facade contracts must be stable before hardening (Phase 14) signs them.
- **cycle-45 ↔ Waves**: `parallel_bridge`. It does not architecturally block Wave 1, but the project's linear cycle sequence closes cycle-45 before cycle-46 starts. Its bounded adapters must exist before Wave 2 dispatch or Wave 4 `ship` runs production user code.

### Risk and rollback matrix

> **Additive-first invariant.** No destructive ledger migration is permitted. Rollback preserves canonical events, digest, and event count; it disables the new emit/read path, falls back to legacy behavior, and rebuilds projections/cache.

| Wave | Observable failure signal | Stop-the-line trigger | Rollback / degrade mode | Authority preserved |
|---|---|---|---|---|
| Wave 1 | Identical inputs produce different `goal_hash`/`plan_hash`, or a tracked-input change still returns `UP_TO_DATE=true`. | Any false-positive `UP_TO_DATE` assertion or hash instability in local release gates. | Keep new event schemas readable but stop emitting/using them; return facade to shadow mode and use existing receipt comparison. | Written events remain append-only; projections rebuild from the ledger. |
| Wave 2 | One run writes zero or multiple `cycle.skipped` events, or stale input writes no `plan_hash_changed`. | Two identical local test runs produce different event counts. | Disable skip writes and cache-based dispatch; use inline dispatch with read-through cache only. | Cache is disposable; ledger remains canonical. |
| Wave 3 | Identical `why` queries return different chains, stale terminal event IDs, or exceed the latency budget. | Determinism fails once in three local runs, or either named query exceeds `<100ms`. | Remove the affected query from the promoted facade, mark it experimental, and rebuild via `sddk graph rebuild`. | Active Graph remains a projection per ADR-022. |
| Wave 4 | First-class help set drifts, dry-run changes the ledger, or recover changes canonical events. | Any facade invariant test fails under `cargo test --workspace --locked`. | Keep facade commands in shadow mode and route to legacy implementations; do not advance deprecation. | Legacy commands remain available; ledger digest/event count stay stable. |

No per-command feature flag exists in `crates/sddk-cli/Cargo.toml`; rollback therefore uses shadow mode, command opt-in, and legacy fallback rather than inventing a flag.

### Cycle binding (suggested mapping)

| Wave | Cycle | Entry gate | Exit gate |
|------|-------|------------|-----------|
| `parallel_bridge` | cycle-45 | cycle-44 released and archived; public `RunSpec`/`RunOutcome` contract stable. | Six runner families (cargo-nextest, pytest, jest, go/test, maven/test, gradle/test) satisfy bounded-runner tests; Maven/Gradle cover Java + Kotlin/JVM; public runner contract unchanged. |
| Wave 1 | cycle-46 | `Goal` + `goal.registered`; `UP_TO_DATE`, plan content addressing and ambiguity rejection specified. | Local gates pass; `sddk goal status` and `sddk plan` are byte-stable across two runs. |
| Wave 1 + Wave 4 facade shell | cycle-46 | Five facade verbs added without removing, hiding, or rerouting legacy commands. | Five commands are reachable and smoke-tested; legacy commands remain unchanged. |
| Wave 2 | cycle-47 | Plan cache, plan-driven dispatch and skip/replay specified; cycle-45 shipped. | Exactly one `cycle.skipped` for up-to-date work; exactly one `plan_hash_changed` for stale work. |
| Wave 3 | cycle-48 | Active-cycle enumeration, two named `why` queries and at least three views specified. | Both named queries `<100ms` on the 50-cycle fixture; status view is byte-stable. |
| Wave 4 completion | cycle-49 | Facade ready for first-class promotion; legacy migration Stage 1 begins. | First-class help set equals five verbs; dry-run and recover preserve ledger invariants. |
| Wave 5 | Phase 14 | Deferred to the existing Phase 14 entry gate. | Deferred to the existing Phase 14 exit gate. |
| `recover_forward_series` | cycle-50..54 | cycle-49 Wave 4 facade shipped; GAP-6 fixed before cycle-51. | ADR-0078 documented; cycle-50 has 2 items, cycle-51 has 1, cycle-52 has 2, cycle-53 has 1, and cycle-54 has 2; draft specs and blueprints remain in the research package until adoption. |

**Operational sequence:** cycle-45 is next. After it closes, cycle-46 starts the consolidation arc with Wave 1 + the additive Wave 4 facade shell.

### Adoption note for the next consolidation cycle

After cycle-45 closes, the orchestrator opens cycle-46. Its `proposal.md` MUST:

- Reference this wave plan and cite ADR-021, ADR-022, ADR-024, ADR-041, and ADR-0068.
- Forbid parallel evolutives; extensions use an `amends:` header on the cited ADR.
- Deliver **Wave 1 + the Wave 4 facade shell** only: five additive verbs, no legacy demotion and no `sddk advanced` routing. Waves 2-3 remain cycles 47-48.
- Explain the pairing: Wave 1 provides `UP_TO_DATE`; the shell makes that capability reachable without pretending full Wave 4 is complete.
- ~~Apply §Critical correction~~ ✅ Applied in cycle-46: §Cross-phase debt row split into Wave 3 + Wave 4 rows (goal-up-to-date-facade-shell, commit e372c99).

### Publication release plan (cycle-46 exit)

GitHub Releases froze at v1.36.1 (2026-08-22) while tags v1.37.0..v1.49.0
shipped tag-only (no artifacts): 253 commits / 371 files / +54,629 lines
unshipped. `install.sh` and `sddk dev update` can only serve v1.36.1. The
remedy is ONE publication release that supersedes the gap — intermediates are
never published individually.

**When cycle-46 closes (Goal/UP_TO_DATE + Wave 4 facade shell), the publication
release is an explicit exit deliverable**, not an afterthought:

| # | Step | Evidence |
|---|---|---|
| 1 | Version lockstep: `chore(release): bump version …` sets `workspace.package.version` = target tag; the annotated tag points to that commit (docs/RELEASING.md §Version lockstep rule) | Cargo.toml diff + tag peel |
| 2 | `tools/manifest.sh` (removed — replaced by `sddk dev manifest` in cycle `p-52b95ef55999f9de/phase-b-rust-lint-foundation`) regenerated in the same commit if prompts/skills/agents/docs changed | MANIFEST.sha256 diff |
| 3 | `sddk release dist` musl x86_64 + aarch64; bundle `software-development-decision-kernel.tar.gz` | checksums.txt, SBOM, attestation |
| 4 | Local smoke test of `install.sh` in a clean sandbox + `sddk dev update --version <v>` from an older install (CI unavailable — local-first) | install receipts, `sddk dev doctor` |
| 5 | `gh release create vX.Y.Z` with all assets; doctor shows `binary.bundle_coherence: present` | release URL, doctor output |

Entry condition: cycle-46 runtime `CLOSED`, `HEAD == origin/main`, workspace
gates green. Related INC follow-ups: anti-push guard for apply slices, LOC
budget enforcement at planning time, automated workspace↔tag lockstep check in
`sddk release plan/apply`.

### Consolidation Definition of Done

The arc is complete after full Wave 4; Wave 5 hardening remains a separate Phase 14 objective.

| Check | Done criterion | Evidence |
|---|---|---|
| Functional arc | Waves 1-4 and cycle-45 gates pass twice consecutively on the same candidate SHA. | Local `cargo fmt`, `cargo clippy`, and `cargo test --workspace --locked` receipts. |
| Facade | `status`, `plan`, `run`, `ship`, `recover` are the five first-class workflow verbs. | Deterministic CLI help integration test. |
| Ledger safety | Dry-run, recover, and rollback preserve canonical digest/event count. | `sddk ledger verify --format json` before/after tests. |
| Schema/ADR closure | Planned events are registry-tested; amendments target only ADR-021/022/024/041/0068. | Domain tests plus ADR review evidence. |
| Migration | Stage 1 is shipped; every migrated legacy path has a facade or advanced mapping. | CLI migration tests and release notes. |
| Release/archive | Each cycle satisfies the completion guard: `HEAD == origin/main`, remote annotated tag peels to that SHA, archive-manifest links release-receipt, runtime is `CLOSED`, ledger verifies. | Receipts and manifests under `{cycle-artifacts-dir}`; no repo-local archive artifacts. |

### CLI deprecation and migration policy

Deleting a legacy command is breaking. No legacy top-level alias is removed in a patch or minor release.

| Stage | Binding | Action | Semver |
|---|---|---|---|
| 0 — additive shell | cycle-46 | Add five facade verbs; preserve all legacy routing/help. | patch/minor |
| 1 — first-class promotion | cycle-49 | Promote the five verbs; legacy commands remain top-level and supported. | minor |
| 2 — advanced namespace | later minor | Add `sddk advanced <legacy> ...`; retain top-level aliases with stderr/help deprecation notice for at least one full minor. No event is emitted. | minor |
| 3 — alias removal | later major | Remove deprecated top-level aliases; keep supported operator commands under `sddk advanced`. | major |

| Current path | Facade | Stage-2 advanced path |
|---|---|---|
| `sddk cycle status` | `sddk status --cycle <id>` | `sddk advanced cycle status` |
| `sddk cycle start` | `sddk run <goal>` | `sddk advanced cycle start` |
| `sddk cycle transition/evaluate-gate/lock` | no human facade; internal/operator surface | `sddk advanced cycle ...` |
| `sddk cycle rebuild` | `sddk recover <cycle>` | `sddk advanced cycle rebuild` |
| `sddk ledger ...` | `status` exposes summary only | `sddk advanced ledger ...` |
| `sddk graph ...` | `status`/`ship --why` expose selected views | `sddk advanced graph ...` |
| `sddk release ...` | `sddk ship <cycle>` | `sddk advanced release ...` |
| `sddk debt ...` | `status --kind debt` exposes summary/why | `sddk advanced debt ...` |

Other top-level commands are outside this consolidation arc; their migration requires a separate compatibility decision. Across all stages, dry-run and recover preserve ledger authority, and no command-visibility feature flag is invented.

---

## Cycle 44–45 edge: Bounded Execution (polyglot foundation → adapters)

> **Cycle-45 classification:** `parallel_bridge`. Adapter direction is cycle-45 → cycle-44 contract. It is not an architectural prerequisite for Wave 1, but the project's linear workflow closes cycle-45 before cycle-46 starts. It must ship before Wave 2 dispatch and Wave 4 `ship` execute production user code.

### cycle-44 (foundation, A-full)

Closed REQ-WF-RT-017 (wall + no-progress budget), REQ-WF-RT-018 (bounded-process contract header), REQ-IPV (independent pass evidence), REQ-Bundle-Coverage fail-closed manifest regeneration.

Key decisions: `ExecutionController` crate-private (D1), additive `Budgets::no_progress_threshold` with serde default (D2), `Instant::elapsed()` from `execute()` entry for wall budget (D3), additive `RuntimeError::{BudgetExceeded, NoProgressDetected}` with `AlreadyTerminal` precedence (D5), ADR-0068 single authority.

### cycle-45 (adapters, deferred from cycle-44)

Runner-specific adapters — six families by **build/test runner**: `cargo-nextest` (Rust), `pytest` (Python), `jest` (JS/TS), `go/test` (Go), `maven/test` (Java + Kotlin/JVM via Surefire/Failsafe), and `gradle/test` (Java + Kotlin/JVM via the Gradle `test` task) — implement the bounded-process-execution contract against the cycle-44 header doc in `sddk-gateway::runner`. No contract change in cycle-44; adapters depend on the contract, not the other way around.

**Out of cycle-45 scope:** Android instrumentation, Kotlin/Native, Kotlin/JS, and non-JVM Kotlin Multiplatform. cycle-45 maps bounded process outcomes; it does not parse JUnit, TestNG, or Kotest reports.

### Cycle-45 deterministic gates

| Gate | Local evidence |
|---|---|
| Entry | cycle-44 is released and archived; `RunSpec`/`RunOutcome` in `crates/sddk-gateway/src/runner.rs` are the stable public contract. |
| Exit | `crates/sddk-gateway/tests/bounded_runner_contract.rs` — 18 external conformance cases (S1–S9 typed contracts + S10 public runner drift). All 18 pass; six families have deterministic `RunSpec` construction; Maven/Gradle additionally exercise JVM fixtures. |
| No contract drift | A public-contract test proves `RunSpec`, `RunOutcome`, and `run(&RunSpec)` semantics remain compatible with ADR-0068; incompatible changes require an explicit ADR amendment before implementation. |
