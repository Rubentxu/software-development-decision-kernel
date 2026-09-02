# Product Backlog — Dynamic Workflow Refinement

## Epic DW — Dynamic Workflow Runtime
- WorkflowTemplate schema.
- WorkflowIR schema and hash/provenance.
- WorkflowCompiler service.
- WorkflowValidator service.
- Map/Join/Race/Loop operators.
- ExecutionGraphRevision.
- ExpansionProposal command/event lifecycle.
- graph budget/conflict/worktree guards.
- deterministic replay test.

## Epic SDD-A — Adaptive SDD
- ChangeContract schema.
- SHAPE capability and dynamic specialist selection.
- WorkGraph/WorkUnit model.
- BUILD worktree mapping.
- CONVERGE verdict/gap schema.
- adaptive verification router.
- proposal/spec/design/tasks/report projections.
- INTEGRATE behavior composition.

## Epic LAB — Workflow Laboratory
- WorkflowExperiment entity.
- A-full/adaptive comparable evaluation contract.
- fork/ablation runner.
- workflow metrics.
- handoff/read-use proxy.
- static Cockpit comparison views.
- promotion/shadow policy.

## Epic SECRETARY-A — Secretary Subagent (Phase 6 foundation)

> **Status:** Stage 0 docs-only (este ciclo) · **Stage 1:** `proposed / blocked-by-SPEC-028-promoted` (gate definido en [[SPEC-042-secretary-runtime]] §Promotion gate)
> **Owner:** secretary-orchestrator
> **Spec:** [[SPEC-042-secretary-runtime]] · **ADRs:** [[ADR-0072-secretary-budgets]] · [[ADR-0073-secretary-authority]]

### Stage 0 deliverables (cycle actual)

- `SPEC-042-secretary-runtime.md` (`docs/sddk-decision-kernel-architecture/04-specs/`)
- `ADR-0072-secretary-budgets.md` (`docs/adr/`)
- `ADR-0073-secretary-authority.md` (`docs/adr/`)
- Epic `SECRETARY-A` en este BACKLOG
- `ROADMAP.md §Phase 6` amendment
- Entrada opt-in en `CHANGELOG.md`

### Stage 1 deliverables (próximo ciclo — **bloqueado por el gate `SPEC-028-promoted`**)

- `agents/secretary.md`
- `prompts/sddk/secretary.md`
- `skills/secretary-runtime/SKILL.md`
- `workflows/sddk-secretary.yaml`
- Implementación closed-set L1 (agent-only MVP)
- Per-call budget composition
- Anti-fabrication tests baseline

### Stage 2 deliverables (futuro — requiere ADR adicional)

- Opcional `sddk secretary status | rebuild | verify | agenda-add | agenda-drop` en `crates/sddk-cli` (solo si Stage 1+ lo justifica)
- Requiere ADR explícito para levantar la regla "no Rust binary"

### Priorización comparativa

| Epic | Business value | Dependencies | Stage 0 fit | Cycle candidate |
|------|---------------|-------------|-------------|-----------------|
| SECRETARY-A | Alto — cierra gap documental Phase 6 arrastrado desde fundación; habilita metric `% fallos sin LLM Supervisor` en Stage 1+ | Stage 0 sin deps runtime; Stage 1+ requiere el gate `SPEC-028-promoted` + Phase 4 DW parcial | ✅ Óptimo — cero runtime deps; solo docs | **Este ciclo (Stage 0)** |
| DW | Medio-alto — Dynamic Workflow Engine | Phase 4 (no SECRETARY-A) | ⚠️ Dependiente de Phase 4 | cycle-30+ |
| SDD-A | Medio — Adaptive SDD | Phase 8 (no SECRETARY-A) | ❌ Fuera de scope | cycle-31+ |
| LAB | Medio — Workflow Laboratory | Phase 9 (no SECRETARY-A) | ❌ Fuera de scope | cycle-32+ |
| IDE-AR | Medio — Authoritative IDE reconciliation | Ninguna (cycle-29 completado) | ❌ Fuera de scope | cycle-29 |
| DEBT | Medio — Durable remediation | ADR-021/022/031/032/034/039, SPEC-023/027/031/034/035/038 | ❌ Fuera de scope | backlog |
| MAP | Alto — Map operator (completado) | Ninguna (✅ completado cycle-31) | N/A (completado) | cycle-31 ✅ |
| TEST-BOUNDARY | Medio — Test-Tooling Boundary | Ninguna (Phase C en curso) | ❌ Fuera de scope | Phase C |

**Nota:** Stage 1 NO está marcado como ready-to-ship. Solo Stage 0 es entregable de este ciclo. Stage 1 queda explícitamente `proposed / blocked-by-SPEC-028-promoted`.

### Dependencias duras

- El gate `SPEC-028-promoted` (definido en [[SPEC-042-secretary-runtime]] §Promotion gate) es precondición para cualquier trabajo Stage 1+ — SPEC-028 debe alcanzar successor aceptado/implementado o status transition via ADR antes de que Stage 1 abra.
- Dynamic graph execution (Phase 4) debe estar parcialmente shipped antes de Stage 1+
- La regla "Dynamic graph execution before Supervisor smarter" se preserva en [[ROADMAP.md §Important sequencing]]

### Parent→child ordering

Orden de commits de este ciclo: Epic `SECRETARY-A` → `SPEC-042` → `ADR-0072/73` (Epic → SPEC → ADR; mit R3 orphan-adr). Cada nivel precede al siguiente y referencia al anterior.

## Existing priority epics retained
- Hexagonal convergence/focused ports.
- Canonical events/ledger.
- OpenCode AgentHost.
- Provider failover/router.
- Context Capsules.
- Active Graph/Why.
- Static Cockpit.

## Epic IDE-AR — Authoritative IDE reconciliation (cycle-29 candidate)

> **Status:** proposed (no ejecutado). Spec en [`docs/reconciliation-spec.md`](../../reconciliation-spec.md). ADR en [`docs/adr/ADR-0064-sddk-authored-reconciliation.md`](../../adr/ADR-0064-sddk-authored-reconciliation.md). Roadmap entry en [`ROADMAP.md §Cycle-29 candidate`](./ROADMAP.md).

### Capability
`sddk dev reconcile [--apply] [--check] [--format json] [--editor <X>] [--root <path>] [--<ide>-dir <path>]`

### Capabilities per IDE
- **opencode / zcode (JSON):** mutate in-place 5 claves sddk en `agent[name]`, preserva `extras` (claves no-sddk). Parsea `opencode.json` / `zcode.json` completos.
- **claude (YAML frontmatter + body):** parsea `.md` completo, reescribe con frontmatter conocido + claves extra + body. `claude_model_valid` se aplica.
- **codex (TOML):** `toml::from_str` del `.toml`, reescribe con claves sddk + extras (incl. `model_reasoning_*`). `toml::to_string_pretty` + `atomic_write`.

### Core traits
- `EditorCapabilities { ide, supports_mode, supports_hidden, supports_prompt_ref, supports_tools, model_validator }`.
- `ReconcileAdapter { capabilities(), read_existing(), reconcile(ctx, apply) }`.
- Cada adaptador concreto (`OpenCodeAdapter`, `ZCodeAdapter`, `ClaudeAdapter`, `CodexAdapter`) implementa `EditorAdapter` (link) + `ReconcileAdapter` (reconcile).

### Ownership rule
- Agente "de sddk" ⇔ nombre en `root/agents/*.md` (leído por `load_agent_sources`).
- Solo agentes sddk → reconciliables / pruneables.
- Agentes de usuario y campos ajenos → **preservados intactos**.
- `NoModelConfigured` → `skipped` (no se borra entrada existente).

### Acceptance gate
Ver [SPEC-RECONCILE-001 §8](../../reconciliation-spec.md#8-criterios-de-aceptación).

### Out of scope (v1)
- `permission`, `color`, `tools`, `metadata` por IDE (framework extensible los soporta; ciclo posterior).
- Migrar `dev link` a reconciliar por defecto (no en v1 — preserva ADR-0018).
- Limpiar el `model:` inerte del frontmatter fuente (sigue inerte; `agent-models.yaml` manda).

### Test pyramid
- Unit (por IDE): `read_existing`, `reconcile`, idempotencia.
- Integration: `--check` exit codes, `--format json` schema estable.
- E2E: drift simulado → `reconcile --apply` → drift eliminado.
- Regression: `link_e2e_tests.rs`, `models_cmd_tests.rs`, `agent_models_tests.rs` (todos verdes).

### Cycle binding (resolved 2026-08-26)
- **cycle-29 = reconcile** (aceptado).
- Map source-context isolation + cross-tick replay → **cycle-30** (reprogramado).
- Decisión tomada antes de `phase.propose.complete`.
- UAT extraction.
- Supply-chain provenance.

## Epic DEBT — Durable remediation
- `DebtReportV2` SDD-pack schema and canonical Rust validator.
- CAS-bound report plus evaluator-derived `DebtVerdict` and signed gate evidence.
- canonical `debt.*` events with idempotent operation IDs.
- rebuildable incidence, Active Graph and optional `INC-NNN` Markdown projections.
- tagged lifecycle operations for create/observe/reopen/reprioritize/resolve/fingerprint alias.
- governed accepted-risk, expiry, early resolution and emergency-plan override.
- deterministic P0-P3 queue with reason codes and versioned policy.
- immutable debt-plan input bound at workflow start.
- selected-debt ChangeContract invariant and bounded same-run convergence.
- read-only artifact inventory before any compaction proposal.

**Dependencies:** ADR-021/022/031/032/034/039, SPEC-023/027/031/034/035/038,
ADR-040 and SPEC-041. SDD-specific types remain pack-owned; no debt special case
enters the generic workflow kernel.

## Epic LF — Lifecycle Flexibility: cycle pause + backlog/roadmap as governed objects (candidate)

> **Status:** proposed → **in-progress** (Primitive 1: cycle-55 in apply). Seed: [`docs/evolutivo-correcciones-flexibilidad.md`](../../evolutivo-correcciones-flexibilidad.md) — extension con insights 8-9 de la serie recover-forward. Roadmap entries en [`ROADMAP.md §Post-Wave 4 — Lifecycle-flexibility candidates`](./ROADMAP.md).
> **Prerequisites:** GAP-6 (reparación de `cycle lock acquire`) + cycle-51 (supersede de primera clase).
> **Owner:** orchestrator
> **Origin:** requisito del maintainer (2026-09-02): "pausar un ciclo porque surge una necesidad nueva, capturar esa idea en el backlog con especificación y priorización, y que nada se rompa ni se pierda — consistencia por construcción, arquitectura emergente".

### Governance flow (objetivo)

`idea` → `backlog capture` (evento, con evidencia de origen) → `triage` (prioridad versionada) → `roadmap entry` → `cycle`. El markdown de BACKLOG/ROADMAP deja de ser la fuente de verdad y pasa a ser proyección consultable del ledger.

### Primitive 1 — `cycle pause` (→ cycle-55, DRAFT-ADR-H) ✅ RESOLVED (cycle-55, kernel-cycle-55-cycle-pause, v1.70.0)

- Estado `CycleStatus::Paused` o taxonomía de razones sobre `Blocked` (decisión del ADR con evidencia).
- Transiciones legales: `Open→Paused`, `Paused→Open` (resume con re-fencing del lease), `Paused→Superseded` (vía cycle-51, manteniendo la referencia cruzada).
- Razón tipada (`priority_revoked`, `context_switch`, `dependency_waiting`) + fecha de revisión opcional.
- Lease auto-release al pausar; expediente y evidencia intactos; sin transiciones de cierre mientras está pausado.
- **See:** [[ADR-0080-cycle-pause]], [[REQ-Cycle-Pause-Contract]], [[REQ-Cycle-Resume-Contract]], [[SPEC-PAUSE-001]]

### Primitive 2 — backlog/roadmap como objetos del ledger (→ cycle-56, DRAFT-ADR-I)

- Eventos `backlog.item.registered` / `.triaged` / `.promoted` / `.discarded`, con evidencia de origen (ciclo, fase, artefactos) — capturar una idea emergente nunca rompe ni cierra el ciclo que la originó.
- Prioridad como metadata versionada y consultable por tooling (no como una fila de markdown).
- `BACKLOG.md` / `ROADMAP.md` como proyecciones renderizadas (mínimo viable: entradas con IDs de ledger trazables).

### Acceptance criteria (borrador, se cierra en la fase spec de cada ciclo)

- Pausar un ciclo activo conserva expediente y evidencia; reanudar o supersedar deja rastro consultable en el ledger.
- Toda idea emergida durante un ciclo puede capturarse sin cerrarlo, con trazabilidad completa origen→backlog→roadmap→ciclo.
- Cero edición manual del ledger; las proyecciones markdown se regeneran desde el estado del ledger.

### Out of scope (v1)

- Priorización automática asistida por IA.
- Sincronización con trackers externos (Tuleap, Jira, GitHub Projects).

## Epic SD — State-Driven CLI: advisor + context inference (candidate, next iteration)

> **Status:** proposed (candidato, sin ejecutar). Research package completo: [`research/state-driven-cli/RESEARCH.md`](../../../research/state-driven-cli/RESEARCH.md).
> **Prerequisites:** ninguno duro. GAP-UX-1 (v1.66.6) es el precedente; `resolve_project_identity`, `cycle_leases`, transiciones declaradas del engine y `WorkflowRun` ya existen en el dominio (~80% de cimientos presentes).
> **Owner:** orchestrator
> **Origin:** requisito del maintainer (2026-09-02): "el CLI puede inferir muchos datos a partir del estado actual y evitar el sobre-esfuerzo del LLM de adivinar dónde estamos y cómo pasarle los argumentos requeridos cuando ya son fácilmente inferidos". Refinamientos vinculantes: (1) workflows dinámicos futuros — cero secuencias hardcodeadas en el advisor; (2) inferencia > declaración.

### Problema (una línea)

El LLM gasta ~10–25k tokens por ciclo en burocracia: adivinar args deducibles del estado (`--root/--scope/--cycle`), reintentar tras errores genéricos (`STORAGE_NOT_FOUND` + recovery sin comando), y recargar docs (AGENTS §8/§9 + mcw.md) para recordar el flujo.

### Primitive 1 — Context inference (→ cycle-52)

- `--root` por walk-up de marcadores; `--project-id`/`--scope` vía `resolve_project_identity`; `--cycle` desde el lease activo.
- Precedencia: arg explícito > inferido > error tipado con lista de candidatos. Flag `--no-infer` como opt-out.

### Primitive 2 — Frontier advisor `sddk cycle next` (→ cycle-53)

- Computa la frontera de transiciones legales leyendo el **grafo declarado** del workflow (+ ledger events + artifacts). Output humano con `hint:` y `--json` para agentes (~150 tokens vs ~800 de reconstrucción actual).
- **Restricción D1 (vinculante):** cero secuencias canónicas hardcodeadas — el advisor sigue cualquier topología declarada, incluidas las auto-generadas del Epic LF/`WorkflowRun`.

### Primitive 3 — Actionable hints + reconciliación YAML↔ledger (→ cycle-54)

- Todo error cita el comando exacto (generalización de GAP-UX-1; grep de "recovery: create the record" genérico = 0).
- `sddk cycle next --json` pasa a ser la única fuente de estado que los prompts del orquestador consumen, eliminando la recarga de AGENTS §8/§9 + mcw.md por fase. Reconcilia el mismatch workflow-YAML (orquestador) vs transiciones declaradas (kernel) documentado en cycle-51.

### Acceptance criteria (borrador, se cierra en la fase spec de cada ciclo)

- `sddk cycle status` con cero args funciona en proyecto adoptado con lease activo único; en ambigüedad devuelve error tipado con candidatos.
- `sddk cycle next --json` produce la transición legal correcta en ≤1 comando para cualquier estado, y sigue correcto con un workflow de topología no-A-min (proof: YAML alternativo).
- Errores de storage/engine citan comando exacto; el orquestador referencia `cycle next` como fuente de estado.

### Out of scope (v1)

- Ejecución de workflows dinámicos en runtime (dominio del Epic LF / `WorkflowRun` instanciado).
- Sugerencias semánticas asistidas por IA ("qué ciclo debería abrir"); el advisor es determinista y derivado de datos declarados.

## Candidate BSG — CLI bare-slug cycle-id acceptance (deferred)

**Problem:** `sddk cycle status --cycle <bare-slug>` returns generic `STORAGE_NOT_FOUND`; should return a typed error that points the user to the canonical form `<project_id>/<slug>`.

**Proposed fix:** Extend `validate_cycle_project` to accept a bare slug and return a descriptive error message (or distinct error code) that guides the user toward using the full `<project_id>/<slug>` form, rather than conflating bare-slug input with an actual missing row.

**Acceptance criteria:**
- Running `sddk cycle status --cycle gap6-lock-repair` (bare slug) returns a clear, actionable error message indicating the correct canonical form.
- Running `sddk cycle status --cycle p-63676b11dc0ef88f/gap6-lock-repair` (full id) succeeds normally.
- Error message distinguishes "bare slug provided" from "cycle row not found".

**Out of scope:** Normalization of bare slugs across all CLI commands; full slug canonicalization infrastructure.

**Owner:** orchestrator

**Priority:** P3

**Reference:** explore report `p-63676b11dc0ef88f/cycle-artifacts/gap6-lock-repair` (cycle-57, sha256 `eed9b8140669cef66470b55e779e45b9fdcbfe90d7788dab790e5a2b111d823b`)

## Important sequencing
Dynamic graph execution belongs **before** trying to make the Supervisor smarter. The runtime must be able to validate and durably execute proposed strategies first.

---

## Debt carry-forward — cycle-30 (closed 2026-08-25, tag v1.46.0, commit e56ce0b)

> Cycle-30 verdict: **PASS_WITH_WARNINGS** (7 findings: 2 P2 + 5 P3; 2 P3 resolved in-archive)

### P2 — Medium priority (owner: orchestrator)

| ID | Title | Cluster | Attribution | Remediation |
|---|---|---|---|---|
| `RUNTIME_CHECKPOINT_NOT_IMPLEMENTED` | MapCheckpointState built but runtime-side draining out of scope | CL-05 | cycle-30 introduced | **CLOSED in cycle-32 (v1.48.0)** — `drain_pending_map` + `pending_map` field; see ADR-0067 |
| `PREEXISTING_CLIPPY_DEBT_SDDK_CLI` | 7 clippy errors in sddk-cli/ confirmed preexisting on base aac9920 | CL-01 | pre_existing | cycle-32+ or earlier if dedup found |

> **INC-DEBT-007** emitted for `PREEXISTING_CLIPPY_DEBT_SDDK_CLI` (docs/debt/INC-DEBT-007-preexisting-clippy-sddk-cli.md).

### P3 — Low priority (backlog / cycle-31+)

| ID | Title | Cluster | Status |
|---|---|---|---|
| `TDD-CHRONOLOGY-DEVIATION` | cycle-30 collapsed RED + GREEN into single commit 7dd9502 | CL-01 | cycle-31+ may enforce stricter chronology |
| `SEQUENTIAL_PENDING_UNTESTABLE` | Sequential Pending path cannot be exercised through TaskExecutor | CL-05 | architecture-bound; no fix without new operator types |
| `C4_INTENT_HASH_DRIFT` | proposal-c4-intent.md embedded SHA256 stale vs disk | CL-08 | **RESOLVED in archive** — SHA256 regenerated |
| `HANDOFF_TEMPLATE_GAPS` | handoff had unfilled `<commit-sha>` placeholders | CL-09 | **RESOLVED in archive** — placeholders filled |
| `CONCURRENT_PATH_SOURCE_SNAPSHOT_EMPTY` | concurrent Pending path instantiates empty BTreeMap for source_outputs_snapshot | CL-03 | cycle-30 introduced | **CLOSED in cycle-32 (v1.48.0)** — `source_outputs_snapshot` populated from `source_outcome.outputs.clone()` in `evaluate_map_body` (INV-11 fix) |

### Epic MAP — Map operator (cycle-31 focus: DC-MAP-002 dispatch global)

> **Owner:** orchestrator
> **Scope:** DC-MAP-002 dispatch global refactor; affects Map, Parallel, and Sequence equally.
> **Spec:** `REQ-Map-Dispatch-Global` (vault, to be created in cycle-31 propose phase).
> **Status:** ✅ COMPLETED in cycle-31 (v1.47.0, commit 8fbf287)
> **Out of scope for cycle-31:** runtime-side checkpoint draining (architecture-bound; separate concern from dispatch).

### P3 — Low priority (cycle-31 carry-forward)

| ID | Title | Cluster | Attribution | Remediation |
|---|---|---|---|---|
| `DISPATCH_LATE_MERGE` | Apply agent merged feat branch to main prematurely (before release complete) | CL-07 | cycle-31 introduced | cycle-32+ apply must NOT merge to main until release receipt confirmed |
| `RESOLVE_CHILDREN_DUPLICATION` | 3 recursive patterns (Sequence/Parallel/Choice children) share structure; `resolve_children(ids, ir)` helper would deduplicate | CL-06 | cycle-31 introduced | cycle-32+ extract `resolve_children(ids, ir)` helper |
| `TDD_DOCSTRING_STALE` | Old docstring in operator_trait_tests.rs still mentions RED/GREEN cycle-31 steps | CL-01 | cycle-31 introduced | cycle-32+ rewrite docstring to reflect final implementation |
| `STALE_DISPATCH_TEST_NAMES` | 3 test names in operator_trait_tests.rs referenced removed `dispatch()` function | CL-01 | cycle-31 introduced | **RESOLVED in archive** — renamed to `build_operator_*` (commit 8fbf287) |
| `MISSING_APPLY_PROGRESS` | apply-progress.yaml was not emitted by apply agent | CL-07 | cycle-31 introduced | **RESOLVED in archive** — regenerated by release phase |
| `BODY_TYPE_REFINEMENT` | REQ-WF-RT-015 said body stored as `Arc<dyn Operator>` but actual type is `Arc<Task>` | CL-06 | cycle-31 introduced | **RESOLVED in archive** — spec updated + docstring fixed (ADR-0066:70-71) |
| `FMT_REGRESSION_INTRODUCED` | 23 rustfmt violations introduced by cycle-31 commits | CL-01 | cycle-31 introduced | **RESOLVED in archive** — fixed by orchestrator (b5a12d4 fmt commit) |

## Debt carry-forward — cycle-32 (closed 2026-08-25, tag v1.48.0, commit b855552)

> Cycle-32 verdict: **PASS_WITH_WARNINGS** (4 findings: 2 P2 + 2 P3; 2 P2 closed, 2 P3 open)
> **ADR:** [ADR-0067](../../adr/ADR-0067-map-runtime-checkpoint-draining.md)

### P2 — Medium priority (owner: orchestrator)

| ID | Title | Cluster | Attribution | Remediation |
|---|---|---|---|---|
| `RUNTIME_CHECKPOINT_NOT_IMPLEMENTED` | MapCheckpointState built but runtime-side draining out of scope | CL-05 | cycle-30 introduced | **CLOSED in cycle-32 (v1.48.0)** — `pending_map` + `drain_pending_map`; ADR-0067 §Decision.1 + §Decision.4 |
| `PREEXISTING_CLIPPY_DEBT_SDDK_CLI` | 7 clippy errors in sddk-cli/ confirmed preexisting on base aac9920 | CL-01 | pre_existing | **INC-DEBT-007** (3 cycles stale; out of scope for cycle-32) |

### P3 — Low priority (backlog / acceptable)

| ID | Title | Cluster | Attribution | Remediation |
|---|---|---|---|---|
| `CONCURRENT_PATH_SOURCE_SNAPSHOT_EMPTY` | concurrent Pending path instantiates empty BTreeMap for source_outputs_snapshot | CL-03 | cycle-30 introduced | **CLOSED in cycle-32 (v1.48.0)** — INV-11 fix in `evaluate_map_body` |
| `DRAIN_PATTERN_DUPLICATION` | `drain_pending_map` and `drain_pending_parallel` share skeleton — acceptable refactor candidate | CL-06 | cycle-32 introduced | Acceptable per ADR-0067 §Decision.4; refactor candidate for cycle-33+ if warranted |

## Debt carry-forward — cycle-33 (closed 2026-08-25, tag v1.48.1, commit b81fc02)

> Cycle-33 verdict: **PASS** (2 findings: 1 P2 closed, 1 P3 introduced)
> **API change:** `EditorCapabilities` removed `PartialEq, Eq` derives (leaf crate, 0 consumers)

### P2 — Medium priority (owner: orchestrator)

| ID | Title | Cluster | Attribution | Remediation |
|---|---|---|---|---|
| `PREEXISTING_CLIPPY_DEBT_SDDK_CLI` | 7 clippy errors in sddk-cli/ confirmed preexisting on base aac9920 | CL-01 | pre_existing | **CLOSED in cycle-33 (v1.48.1)** — `cargo clippy --workspace --all-targets -- -D errors` exit 0; was exit 101 |

### P3 — Low priority (cycle-34 candidate / introduced)

| ID | Title | Cluster | Attribution | Remediation |
|---|---|---|---|---|
| `DEAD_CODE_SDDK_CLI` | 18 dead_code warnings in sddk-cli (unused exports, dead code paths) | CL-01 | pre-existing | **CLOSED in cycle-34 (v1.48.2)** — 33 items resolved: 17 deleted (C1) + 9 annotated per ADR-0064 §D-4/§D-5 (8 + 1 follow-up C3) + 7 follow-up items; cargo clippy dead_code = 0 in sddk-cli |
| `FIND-000016` | EditorCapabilities removed `PartialEq, Eq` (function pointer field has unpredictable equality) | CL-01 | cycle-33 introduced | **Documented in CHANGELOG**; leaf crate; 0 workspace consumers |

## Epic TEST-BOUNDARY — Test-Tooling Boundary (per ADR-042)

> **Status:** Phase A audit and Phase B ownership migration completed. Phase C is the re-prioritized lint/testkit/stability cleanup. ADR-0069 (accepted) owns the ownership policy. ADR-042 (Accepted) owns the sequencing and migration plan.
> Phased per [ADR-042 §Migration plan](../03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md).

### Phase A — Historical audit (completed)

- Audit 19 shell contract tests from commit `643180a`; classify each per ADR-0069 ownership cells
- Flag binary-behavior tests that belong in Rust (see TEST-TOOLING-EVIDENCE-AUDIT.md §Concrete false positives)
- Light annotation of test filenames with ownership prefix where convention supports it
- Record findings in `TEST-TOOLING-EVIDENCE-AUDIT.md` §5 + §6
- **Outcome:** Completed; findings documented and Phase B ownership migration scoped

### Phase B — Ownership migration (completed)

- Migrated ownership into Rust tests SDDK015-SDDK032, ending with the v1.58.0 final shell migration release
- Retained `tests/test_push_prevention_hook.sh` as the only shell test
- **Outcome:** Completed; Python and JS contract tests are retained as their correct ownership outcomes

### Phase C — Re-prioritized lint/testkit/stability cleanup (next work)

- Add ShellCheck to the local test gate for `tests/test_*.sh`
- Add Ruff to the local scripts gate for `scripts/`
- Evaluate ADR-0022 (sddk-testkit, accepted 2026-08-31 per REQ-Phase-C-ADR-0022-Status-Reconcile) for adoption or supersession
- Consolidate or delete misowned tests after parity (same test passes in new language + original deleted)
- Remove superseded scaffolding only after one release cycle stable

## Debt carry-forward — cycle-34 (closed 2026-08-25, tag v1.48.2, commit a7f1d8a)

> Cycle-34 verdict: **PASS** (0 findings — 33 dead_code items resolved, debt-report clean)
> **ADR-0064 §D-4/§D-5** (capability-framework contract) cited for annotation rationale.

### P3 — Low priority (closed in cycle-34)

| ID | Title | Cluster | Attribution | Remediation |
|---|---|---|---|---|
| `INC-DEBT-008` | 18 dead_code warnings in sddk-cli (carry-forward from cycle-33) | CL-02 | pre-existing | **CLOSED in cycle-34 (v1.48.2)** — see `docs/debt/INC-DEBT-008-dead-code-sddk-cli.md`; 17 deleted + 9 annotated per ADR-0064 + 7 follow-up items |
| `FIND-000017` | Pre-existing dead_code in sddk-cli (24 items surfaced in cycle-33 debt-verify) | CL-02 | pre-existing | **CLOSED in cycle-34 (v1.48.2)** — promoted to INC-DEBT-008; full inventory and ADR-0064 mapping in debt doc |
