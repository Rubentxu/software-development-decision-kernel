# ADR-073 — Planning Ledger Persistence

**Canonical title:** `ADR-073-PLANNING-LEDGER-PERSISTENCE.md`
**Status:** Accepted
**Date:** 2026-09-04
**Decision authority:** User-confirmed architectural locks (Q1–Q4) + spec-phase defaults
**Related Work Item:** PLN-LEDGER-002 (order 110, horizon H1)
**Direct dependency:** PLN-LEDGER-001 (v1.85.0, `327bfb2`)

## 1. Status

Accepted. The four decisions below are locked for PLN-LEDGER-002. This ADR records their architecture authority. The ADR was promoted from draft during apply phase.

## 2. Context

PLN-LEDGER-002 is the second H1 planning-ledger cycle. Its exit gate requires that a fresh checkout reconstruct an identical planning graph without interpreting narrative markdown. PLN-LEDGER-001 (v1.85.0, `327bfb2`) shipped the domain model — six types (`WorkItemV1`, `DependencyEdgeV1`, `WorkItemStatus`, `EvidenceAttachmentV1`, `DecisionRecordV1`, `PlanningProvenanceChainV1`), the `CasPort` port plus `FilesystemCas` adapter, MIGRATION_14 schema (`work_items_v1`, `work_item_dependencies_v1`), the 12-variant `WritableSurface` matrix, six planning lifecycle event emitters, and 14 baseline unit tests + 5 CAS tests.

The exploration found ten gaps: there is no CRUD code for any of the four planning entities, no `DependencyResolutionService` for the `Draft → Active` invariant, no full-graph identity computation, no provenance-chain verification at the engine/storage boundary, no `sddk plan <sub>` CLI surface, and no integration tests for the four new authority surfaces or the six event emitters. The existing CLI namespace is occupied by a D2 facade (`sddk plan <name>` → `cycle start`). The persistence shape for evidence and decision metadata and the interpretation of "narrative markdown" are also unresolved.

PLN-LEDGER-002 inherits the `WorkItemV1::compute_identity()` method but the per-entity hash includes volatile fields (`created_at`, `status`) per FIND-PLN-008, making a full-graph identity computation that survives state changes impossible without explicit exclusion. The exit gate's "narrative markdown" clause is ambiguous without a locked interpretation: `EXECUTION-SPINE.yaml` declares `schema_version: 2` and uses machine-readable keys, but is also narrative prose from the perspective of the planner.

## 3. Decision

### 3.1 Deprecate the existing `sddk plan <name>` facade

The current `crates/sddk-cli/src/plan.rs` shadow router delegates `sddk plan <name>` to `cycle start` (39 lines, D2 facade). This facade MUST be deprecated. The namespace `sddk plan <sub>` MUST be freed for the work-item subspace.

| Surface | Status |
|---|---|
| `sddk plan <name>` | DEPRECATED — emits `--deprecation-warning` to stderr; still delegates to `cycle start` |
| `sddk plan workitem {create,show,list,transition}` | NEW |
| `sddk plan dep add` | NEW |
| `sddk plan evidence attach` | NEW |
| `sddk plan decision record` | NEW |
| `sddk plan graph` | NEW |

The test in `crates/sddk-cli/tests/first_class_commands.rs:84-100` (`plan_help_mentions_cycle_start_delegation`) MUST be updated to assert the new delegation target or replaced. The migration path is documented in the deprecation notice.

### 3.2 Lock `DependencyEdgeKind` semantics

The two-variant enum `DependencyEdgeKind` (`Blocks`, `BlocksOnClosure`) gets explicit semantics:

| Source status | Target transition | `Blocks` | `BlocksOnClosure` |
|---|---|---|---|
| non-terminal (Draft/Active/Paused) | non-terminal (Draft→Active, Active↔Paused) | **BLOCK** | allow |
| non-terminal | terminal (→Done/Superseded/Cancelled) | **BLOCK** | **BLOCK** |
| terminal (Done/Superseded/Cancelled) | any | allow | allow |

A new pure domain service `DependencyResolutionService` (`crates/sddk-domain/src/planning/service.rs`) implements `resolve_can_activate` and `resolve_can_terminalize`. The service takes `&[DependencyEdgeV1]` and a status lookup closure as parameters — it has no I/O. Cycles in `Blocks` edges are detected via Kahn's algorithm.

### 3.3 DecisionRecord rationale is inline, no CAS split

`DecisionRecordV1.rationale` is a `String` field (PLN-LEDGER-001 §3.5). PLN-LEDGER-002 stores it inline as `decision_records_v1.rationale TEXT NOT NULL`. No CAS write occurs for decision bodies; no `body_ref` column; no separate `decision_bodies_v1` table. Rationales are short (< 4 KB typically) and not content-addressable artifacts.

| Field | Storage |
|---|---|
| Decision metadata (id, work_item_id, kind, actor_ref, schema_version) | SQL `decision_records_v1` (MIGRATION_15) |
| Decision rationale | SQL `decision_records_v1.rationale TEXT NOT NULL` |
| Decision body in CAS | NOT USED |

The rationale non-emptiness invariant is enforced by `DecisionRecordV1::new()` at the domain layer, which returns `Err(DecisionError::EmptyRationale)` for whitespace-only input. The storage layer's `insert_decision_record` calls the domain constructor first.

### 3.4 Structured YAML is declarative, not narrative

Per the exit gate's "without interpreting narrative markdown" clause:

- "Narrative markdown" = free-text prose paragraphs without a machine schema.
- Structured YAML with a declared `schema_version` is declarative, not narrative.
- `EXECUTION-SPINE.yaml` declares `schema_version: 2` and uses machine-readable keys (`order`, `id`, `horizon`, `depends_on`, `objective`, `exit_gate`, `status`) — it is declarative.
- PLN-LEDGER-002 sources from SQL + CAS ONLY. The replay path MUST NOT invoke `serde_yaml::from_str` on `EXECUTION-SPINE.yaml`.
- YAML interpretation is PLN-LEDGER-003's concern: it owns the import pipeline that converts `EXECUTION-SPINE.yaml` rows into `work_items_v1` rows.

This interpretation preserves PLN-LEDGER-002's scope as a persistence cycle rather than a migration cycle. PLN-LEDGER-003 closes the bulk-import gap explicitly.

### 3.5 Shape C persistence extended with MIGRATION_15

Per PLN-LEDGER-001 ADR-072 §3.1, Shape C hybrid persists topology in SQL and bodies in CAS. PLN-LEDGER-002 adds:

| Entity | Storage | Migration |
|---|---|---|
| WorkItem topology | SQL `work_items_v1` | MIGRATION_14 (UNCHANGED) |
| DependencyEdge topology | SQL `work_item_dependencies_v1` | MIGRATION_14 (UNCHANGED) |
| EvidenceAttachment metadata | SQL `evidence_attachments_v1` | MIGRATION_15 (NEW) |
| EvidenceAttachment body | CAS filesystem (`~/.local/share/sddk/cas/<sha[0:2]>/<sha[2:4]>/<sha>`) | existing `FilesystemCas` adapter |
| DecisionRecord (incl. rationale) | SQL `decision_records_v1` | MIGRATION_15 (NEW) |

MIGRATION_15 is purely additive: no DROP, no ALTER, no rename. `LATEST_SCHEMA_VERSION` bumps 14 → 15. The schema version cascade touches `event_store.rs:328` and `sqlite_storage.rs:35,978,1484` per FIND-PLN-003 precedent — a mechanical 4-line test update.

### 3.6 Five candidate new event types remain ADR-071 compliant

If new CRUD-coverage events are needed, they MUST use `Type::Custom(<id>)` with `schema_version: 1` and MUST NEVER be added to `std_registry`:

- `planning.work_item.created`
- `planning.work_item.transitioned`
- `planning.dependency.added`
- `planning.evidence.attached`
- `planning.decision.recorded`

The actual emit set is decided at design time; the spec leaves the door open without mandating all five. The six existing planning lifecycle events from PLN-LEDGER-001 are reused unchanged.

## 4. Consequences

### Positive

- The D2 facade deprecation clears `sddk plan` for the work-item subspace without breaking the `cycle start` delegation (still available with deprecation notice).
- `DependencyResolutionService` makes the `Draft → Active` invariant testable in isolation (no engine or storage coupling).
- The two-kind semantics (`Blocks` vs `BlocksOnClosure`) prevent both over-blocking (always blocking non-terminal transitions) and under-blocking (never blocking terminal closure).
- Inline rationale storage eliminates the CAS overhead for short text bodies.
- The narrative-vs-declarative interpretation keeps PLN-LEDGER-002 narrowly scoped to persistence; YAML interpretation is owned by PLN-LEDGER-003.
- MIGRATION_15 is purely additive; existing data is preserved.
- The five candidate event types (if emitted) comply with ADR-071 without polluting `std_registry`.
- `ActorKind` stays 3-variant closed; `WritableSurface` stays 12-variant — no extension.

### Costs and risks

- The deprecated facade `sddk plan <name>` requires an opt-in migration period; users relying on it must switch to `sddk cycle start --name <name>`.
- The `DependencyResolutionService` algorithm's two functions cover the locked transitions but not arbitrary transitions; future state-machine extensions (e.g., a `Reviewed` variant) require extending the service.
- DecisionRecord inline storage couples rationale length to SQL column limits (TEXT handles up to ~1 GB on SQLite, but the assumption of "< 4 KB typically" is enforced only by convention, not by domain validation).
- The narrative-vs-declarative interpretation may be revisited by PLN-LEDGER-003 if it determines the YAML is not declarative enough.
- MIGRATION_15's schema bump 14 → 15 cascades 4 lines per FIND-PLN-003.
- The 58-test pyramid is a meaningful time investment; if it slips, FIND-PLN-004/005/006 remain open.

### Verification obligation

The implementation MUST deliver ~58 new tests across four layers (per AC-PLN2-12 + AC-PLN2-15):

- ~38 domain integration tests in 4 new files (dependency_resolution, provenance_chain, identity, state_machine).
- ~12 engine integration tests in 2 new files (event_emitters, authority).
- ~5 storage integration tests in 2 new files (migration_15, cas_crud) plus the 4 entity CRUD files (work_item, dependency_edge, evidence, decision).
- ~3 new + 2 updated CLI integration tests in `first_class_commands.rs` + `cli_compatibility.rs`.

Tests cover all 15 acceptance criteria, all 4 WritableSurface admit/reject scenarios, all 6 event emitters, MIGRATION_15 roll-forward / idempotency / replay, CAS integrity, DecisionRecord inline validation, `DependencyResolutionService` semantics across both edge kinds, full-graph identity determinism, and exit-gate compliance (no YAML parsing in replay path).

## 5. Alternatives considered

### Q1 — CLI namespace

- **Keep the D2 facade as `sddk plan <name>` and add a parallel `sddk work-items` top-level namespace**: rejected because it leaves `sddk plan` semantically ambiguous (two unrelated command families share the prefix) and confuses help output.
- **Rename the facade to `sddk cycle plan <name>`**: rejected because it moves the facade, not the user's mental model; the migration cost is the same.
- **Deprecate the facade and free `sddk plan <sub>`**: selected because no real users depend on the D2 dispatch (it is a 39-line shadow router), the test can be updated alongside, and the resulting CLI surface is internally consistent.

### Q2 — DependencyEdgeKind semantics

- **Both kinds block all transitions**: rejected because it prevents `Paused → Active` resumptions when the source is still `Active` in another worktree — common regression.
- **Both kinds block only terminal transitions**: rejected because it eliminates the meaningful distinction between the two kinds.
- **`Blocks` blocks only `Draft → Active`, `BlocksOnClosure` blocks only terminal transitions**: rejected because it doesn't capture the case where `Active → Done` should also be blocked by a `Blocks` predecessor.
- **`Blocks` blocks any non-terminal transition of the destination; `BlocksOnClosure` blocks only terminal transitions**: selected because it captures the distinction cleanly: `Blocks` says "wait until the source reaches closure in any direction before the destination can progress", while `BlocksOnClosure` says "only closure requires the source to be done — intermediate state changes are free".

### Q3 — DecisionRecord rationale storage

- **CAS split** (rationale as body in CAS, `decision_records_v1.q_rationale_ref`): rejected because rationales are short, not content-addressable, and the CAS lookup overhead is unjustified.
- **JSON-encoded rationale column** (rationale as JSON string in `decision_records_v1.rationale_json`): rejected because it adds parse overhead with no benefit; the rationale is a `String` already.
- **Inline TEXT column**: selected because the rationale is short, validation is enforced by the domain constructor, and SQL indexes on `work_item_id` are sufficient for query patterns.

### Q4 — Narrative markdown interpretation

- **Forbid all YAML, even structured**: rejected because it forces a more invasive migration (PLN-LEDGER-002 would need to convert `EXECUTION-SPINE.yaml` to SQL, which is PLN-LEDGER-003's job).
- **Allow all YAML, including narrative**: rejected because the exit gate explicitly forbids "interpreting narrative markdown" — without a schema, interpretation is unbounded.
- **Structured YAML (with declared `schema_version`) is declarative; free-text prose is narrative**: selected because it aligns with the exit gate's intent — the planner needs a deterministic source of truth, and structured YAML with a schema satisfies that; narrative prose does not.

## 6. References

- ADR-068 — Deterministic IR and compiler-boundary invariants (canonical JSON, hash-stable replay).
- ADR-069 — Explicit authority matrix and three-variant `ActorKind` contract.
- ADR-070 — Engine `AuthorityContext` enforcement and `WRITABLE_SURFACE_MATRIX` (12 variants).
- ADR-071 — Event schema versioning, `Type::Custom` compatibility, and `ActorKind` closed set.
- ADR-072 — Planning Ledger Domain Model (Shape C hybrid persistence; 6-variant `WorkItemStatus`; 4 new `WritableSurface` variants).
- EVT-LEDGER-001 cycle — event replay and provenance substrate shipped in v1.84.0.
- PLN-LEDGER-001 cycle — domain model shipped at v1.85.0 (`327bfb2`).
- `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml` lines 184-190 — PLN-LEDGER-002 objective and exit gate.
- `docs/debt/FIND-PLN-004.md` — test pyramid gap (CLOSED by this ADR).
- `docs/debt/FIND-PLN-005.md` — missing emit integration tests (CLOSED by this ADR).
- `docs/debt/FIND-PLN-006.md` — missing authority integration tests (CLOSED by this ADR).
- `docs/debt/FIND-PLN-007.md` — provenance chain stub (PARTIALLY CLOSED; full close PLN-LEDGER-003).
- `docs/debt/FIND-PLN-008.md` — volatile `compute_identity` (MITIGATED via volatile-field exclusion).
- `docs/debt/FIND-PLN-003.md` — schema version churn (TRIGGERED mechanically).
- exploration-report.md — 15 AC proposal, 10-gap analysis, 12-finding carryover survey, downstream continuity.
- specification.md — canonical PLN-LEDGER-002 behavioral contract (15 ACs × GWT scenarios × 41 REQs).