---
id: arch-spec-A3-S1-knowledge-substrate
title: Knowledge + KMT substrate (cycle-bounded)
status: accepted
cycle: p-63676b11dc0ef88f/a3-1-kmt-foundation
proposed_at: 2026-09-14
accepted_at: 2026-10-02
accepted_by: "decision normativa del operador sobre INC-DEBT-048, opcion (a)"
supersedes_history: false
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
---

# arch-spec-A3-S1 — Knowledge + KMT substrate (cycle-bounded)

> Cycle-bounded delta spec. Establishes the minimal typed substrate required
> before any ArchitecturalContract, graph overlay, paradigm profile or
> `advisory_context` seam can be safely added.

## Intent

`KnowledgeAssertion`, `KnowledgeBasis` and `KnowledgeFreshness` (the freshness
and invalidation evaluator) become first-class SDDK semantic types
**without leaking A4 semantics** (Alignment / Verify / DebVerify) and **without
introducing provider SDK types into the Knowledge module**.

## Requirements

Each requirement is testable via a deterministic unit test or compile-time
guarantee.

### State-class discipline

- **REQ-A3S1-001** Every new public type SHALL carry a doc-level state-class
  annotation (`# State class: FACT | OBJECT | PROJECTION | EPHEMERAL`). A
  `cargo test` SHALL iterate the public surface of `sddk_engine::knowledge`
  and fail if any type lacks the annotation.

### KnowledgeAssertion

- **REQ-A3S1-010** `KnowledgeAssertion` SHALL be `pub struct` with fields:
  `id: KnowledgeId`, `basis_hash: BasisHash`, `declared_at: EventTime`,
  `kind: KnowledgeKind`, `payload: KnowledgePayload`.
- **REQ-A3S1-011** `KnowledgeId` SHALL be a typed newtype around `String` with
  `Display` + `FromStr` + deterministic equality.
- **REQ-A3S1-012** `BasisHash` SHALL be a 32-byte content hash (typed newtype)
  deterministically derived from `(id, declared_at, kind, payload)`.
- **REQ-A3S1-013** `KnowledgeKind` SHALL be a closed ADT with at least the
  variants: `Observation`, `Declaration`, `Inference`, `Reference`. The ADT
  SHALL be `non_exhaustive=false` to force explicit future extension.
- **REQ-A3S1-014** `KnowledgePayload` SHALL be a closed ADT (`Object`,
  `Relation`, `Fact`) holding typed opaque bytes (Vec<u8>) with a declared
  content-type tag.
- **REQ-A3S1-015** `KnowledgeAssertion` SHALL NOT be constructible without
  going through `KnowledgeAssertion::declare(...)` which computes the
  `BasisHash`. A test SHALL assert the field is private.

### KnowledgeBasis

- **REQ-A3S1-020** `KnowledgeBasis` SHALL be `pub struct` with fields:
  `assertions: BTreeMap<KnowledgeId, KnowledgeAssertion>`,
  `basis_hash: BasisHash`, `revised_at: EventTime`.
- **REQ-A3S1-021** `KnowledgeBasis::basis_hash` SHALL be deterministically
  derived from the sorted `(id, inner_basis_hash)` pairs **and** the basis's
  `revised_at`, under an explicit versioned domain tag (test asserts
  insertion-order independence). Two bases with identical assertions revised
  at different times SHALL therefore have different `basis_hash` values: a
  revision is a change of the basis, not a relabelling of the same one.
  A basis carrying `revised_at: None` SHALL reproduce the legacy `v1` domain
  verbatim, so the historical identity of a pre-existing basis stays
  *reproducible*. The version tag separates the two domains so a stored `v1`
  hash fails closed against a `v2` basis (mismatch ⇒ re-verify) rather than
  comparing equal by accident.
- **REQ-A3S1-022** `KnowledgeBasis::insert` SHALL return the new basis_hash and
  the previous one. It SHALL be `&mut self` only.
- **REQ-A3S1-023** `KnowledgeBasis::revise` SHALL be a free function that
  produces a new `KnowledgeBasis` with monotonically non-decreasing
  `revised_at`. A test SHALL assert that revising with a stale timestamp
  yields an error.

> **Por qué REQ-A3S1-021 incluye `revised_at`** (aceptado 2026-10-02, cierra
> INC-DEBT-048 opción (a)).
>
> La redacción anterior derivaba el hash **sólo** del conjunto de assertions.
> Bajo esa redacción existían dos bases indistinguibles: el mismo contenido
> revisado a distinta hora. No es un caso hypothetical — era el estado real,
> y `KMT::evaluate` compara hashes **antes** que timestamps, así que una
> revisión puramente temporal era invisible al freshness y devolvía `Fresh`
> sin mirar `revised_at`, contradiciendo REQ-A3S1-033.
>
> La participación de `revised_at` no es un parche de ese defecto: es lo que
> hace que «revisar» signifique algo. Una revisión es un cambio del basis, no
> una etiqueta nueva sobre el mismo contenido.
>
> **Acoplamiento con REQ-A3S1-033.** El defecto de `evaluate_freshness` está
> *acoplado* a esta decisión, no es independiente: bajo la derivación `v1` el
> freshness de una revisión temporal nunca se alcanzaba a comparar. Fijada la
> derivación `v2`, un `revised_at` más nuevo que la tolerancia produce
> `Unknown { MissingEvidence::FutureEvidence }` por la rama de timestamps.
> Ver `knowledge::tests::audit_inc_debt_048_pure_temporal_revision_is_not_invisible`.
>
> **Relación con ADR-0126** (`accepted`). ADR-0126 §3 consume
> `KnowledgeBasis::basis_hash()` como entrada de la derivación de
> `IntelligenceLoopReceiptId`; no define esa derivación. Su §4 excluye
> `evaluation_time` de la identidad del receipt: es la hora en que alguien
> evalúa, no el contenido evaluado. `revised_at` es contenido. No hay
> conflicto; la reconciliación está escrita en
> `docs/architecture/adrs/ADR-0126-INTELLIGENCE-LOOP-COMPOSITION-SEAM.md`.
>
> **Impacto en datos: cero, medido.** `KnowledgeBasis` no se persiste
> (`basis_hash` no aparece en `crates/sddk-storage/`). El cambio de dominio
> `v1 → v2` no invalida ninguna identidad almacenada; hoy es una propiedad
> preventiva, no un riesgo operativo.

> **Reconciliación (ADR-0154, 2026-10-03).** `KMT` y `KnowledgeFreshness` no son
> dos nombres del mismo tipo: eran **tres conceptos compitiendo por unas siglas**.
> `pub struct KMT` evaluaba frescura y `KmtIndex` indexaba unidades del árbol, y
> la prosa llamaba al segundo con las siglas del primero. **ADR-0154** fija
> `KMT` = Knowledge Merkle Tree (la estructura, que es lo que C3m.1 construye),
> y por eso el evaluador pasa a `KnowledgeFreshness` y el índice a
> `KmtUnitIndex`. *Knowledge Management Tiers* se retira porque **no describe
> nada que el código tenga**: no hay tiers, ni niveles, ni jerarquía que se
> evalúe. Esta spec se reescribió en el mismo movimiento porque REQ-A3S1-035
> declaraba canónico un símbolo que el ADR renombra — una spec que nombra un
> símbolo que ya no existe declara canónica la nada.

### Knowledge identity / freshness / invalidation

- **REQ-A3S1-030** `KmtStatus` SHALL be `pub enum`:
  - `Fresh { basis_hash: BasisHash }`
  - `Stale { observed: BasisHash, expected: BasisHash, observed_at: EventTime }`
  - `Invalidated { reason: InvalidationReason, invalidated_at: EventTime }`
  - `Unknown { reason: MissingEvidence, last_observed: Option<BasisHash> }`
- **REQ-A3S1-031** `InvalidationReason` SHALL be a closed ADT (`Superseded`,
  `Contradicted`, `Withdrawn`, `Stale`).
- **REQ-A3S1-032** `evaluate_freshness(observed: &KnowledgeBasis,
  expected: &KnowledgeBasis, now: EventTime) -> KmtStatus` SHALL be a pure
  function. Same inputs → same output (test asserts determinism).
- **REQ-A3S1-033** A `KnowledgeBasis` whose `basis_hash` matches the
  expected `basis_hash` evaluates to `Fresh`. A differing `basis_hash` with a
  non-newer `revised_at` evaluates to `Stale`. A `revised_at` newer than the
  expected tolerance evaluates to `Unknown { MissingEvidence }`.
- **REQ-A3S1-034** `invalidate(basis: &mut KnowledgeBasis, reason:
  InvalidationReason, at: EventTime)` SHALL transition the basis to
  `Invalidated`. After invalidation, `evaluate_freshness` SHALL always return
  `Invalidated` (test asserts monotonicity).
- **REQ-A3S1-035** `KnowledgeFreshness::evaluate(basis, expected, now)` is the canonical
  entry point. Direct construction of `KmtStatus::Fresh` outside this entry
  point SHALL be prevented (private tuple field + `From` only).

### Anti-encroachment

- **REQ-A3S1-040** The new module `crates/sddk-engine/src/knowledge/` SHALL NOT
  import `crate::alignment`, `crate::verify`, `crate::debverify`,
  `crate::paradigm_lens`, `crate::authority_engine` types. A clippy lint
  (manual dependency declaration) SHALL be added and the test
  `knowledge_module_has_no_a4_or_provider_imports` SHALL pass.
- **REQ-A3S1-041** The new module SHALL NOT extend `crate::semantic_kind::CoreNodeKind`. S2 will do that. A compile-time test
  `s1_does_not_introduce_new_corenodekind_variants` SHALL pass by inspecting
  the source file via include_str!.
- **REQ-A3S1-042** The new module SHALL NOT modify `crate::context_capsule`. S5 will. A test `s1_does_not_modify_context_capsule` SHALL pass by
  hashing the file and comparing against the pre-S1 hash stored in a
  `// s1-baseline-hash:` doc comment.

### Persistence boundary

- **REQ-A3S1-050** `KnowledgeBasis` SHALL be storable as `serde_json::Value`
  with stable field ordering (test asserts round-trip equality).
- **REQ-A3S1-051** Persistence to SQLite or any other durable store is
  deferred to S2 (which wires Decision Memory revisions). S1 ships
  in-memory + serialization only.

## Acceptance

| Requirement | Test/Probe |
|---|---|
| REQ-A3S1-001 | `test_knowledge_module_state_class_annotations_complete` |
| REQ-A3S1-010..015 | `test_knowledge_assertion_field_privacy`, `test_knowledge_assertion_basis_hash_deterministic` |
| REQ-A3S1-020..023 | `test_knowledge_basis_insertion_order_independence`, `test_knowledge_basis_revise_stale_timestamp_rejected` |
| REQ-A3S1-030..035 | `test_kmt_fresh_when_basis_match`, `test_kmt_stale_on_mismatch`, `test_kmt_unknown_when_evidence_missing`, `test_kmt_invalidation_is_monotonic` |
| REQ-A3S1-040..042 | `knowledge_module_has_no_a4_or_provider_imports`, `s1_does_not_introduce_new_corenodekind_variants`, `s1_does_not_modify_context_capsule` |
| REQ-A3S1-050 | `test_knowledge_basis_serde_round_trip_preserves_basis_hash` |

## Negative fixtures

| Fixture | What it proves |
|---|---|
| `test_knowledge_assertion_cannot_be_built_without_declare` | Type system forbids hand-rolled construction |
| `test_kmt_invalidation_cannot_be_overridden` | Monotonicity of `Invalidated` |
| `test_knowledge_basis_revise_with_stale_time_is_rejected` | Time-monotonicity of revisions |
| `knowledge_module_has_no_a4_or_provider_imports` | Anti-encroachment at compile time |

## Crosswalk

| Spec | Relationship |
|---|---|
| `arch-spec-006-knowledge-vault-context.md` | Knowledge Vault is the durable human-side source; `KnowledgeBasis` is the runtime projection. No authority changes. |
| `arch-spec-032-architectural-contracts.md` | AC1 contract type slot. S2 will reuse `BasisHash` from S1. |
| `arch-spec-033-architecture-semantic-graph-overlay.md` | AC2 graph overlay. S3 will project `KnowledgeAssertion` to `SemanticNode`. |
| `arch-spec-035-paradigm-lens-system.md` | AC3 paradigm profile. S4 will tag `KnowledgeAssertion` with `ParadigmProfile` metadata. |
| SPEC-002-lifecycle-model | PlanRevision lifecycle. S2 will wire `KnowledgeBasis` revisions. |
| SPEC-005-semantic-graph-and-why | S3 will reuse `GraphRevision`. |
| ADR-0095 (Four state classes) | S1 enforces state-class discipline at the type level. |
| ADR-0100 (Universal Evidence) | `KnowledgeKind::Observation` emits Evidence; mapping deferred to S2. |

## Out-of-scope (explicit)

- No `ArchitecturalContract` / `ArchitectureClaim` types (S2).
- No `ParadigmProfile` (S4).
- No `CoreNodeKind` extension (S3).
- No `ContextCapsule::advisory_context` (S5).
- No lenses, no probes, no receipts, no cross-BC evaluation (A4 / AC4..AC7).
- No persistence beyond in-memory + serde (S2).
- No provider/host SDK type references (any cycle).

## Exit criteria

1. All REQ-A3S1-NNN requirements green.
2. Negative fixtures all pass.
3. `cargo fmt --check` clean.
4. `cargo clippy --workspace --all-targets -- -D warnings` clean.
5. `cargo test -p sddk-engine knowledge::` green.
6. `cargo test --workspace` green (regression check).
7. Receipt `A3-S1-KMT-FOUNDATION-RECEIPT.md` produced at exact commit SHA.
8. Mini-roadmap §A3-S1 marked CLOSED with receipt SHA.
