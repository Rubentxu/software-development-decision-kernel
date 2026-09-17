---
id: arch-spec-042-evidence-observation-provenance
title: Evidence observation and provenance substrate
status: accepted
milestone: A4
slice: A4-0
cycle: p-63676b11dc0ef88f/a4-0-verification-provenance-foundation
adrs: [ADR-0122-EVIDENCE-OBSERVES-SOFTWARE]
---

# arch-spec-042 — Evidence observation and provenance substrate

Closes the provenance leg the A3 closeout recorded as missing, as a **general
capability**, not as a patch for `why architecture`.

```text
Finding → Contract → Decision/Spec → Evidence → OBSERVES → SoftwareRelation
```

## Purpose

A producer observes software and records the observation with its provenance. SDDK
decides afterwards how to interpret it.

```text
Evidence ≠ truth      Provider ≠ authority      Observation ≠ decision
```

Any producer — a deterministic local analyzer, CogniCode's static observations,
Chronos's runtime traces — can emit observations **without becoming authority**.

## Scope

**In:** `SoftwareRelation` + `RelationId`; `SoftwareObservation` + `ObservationId`;
`ObservationOrigin`; `ObservationBasis`; contradiction-preserving
`EvidenceResolution`; projection into the one `SemanticGraphProjection`; the `why
architecture` integration.

**Out:** generic Verify, generic DebVerify, `AlignmentAssessment`, alignment lenses,
LLM evaluator, provider adapters, CogniCode, Chronos, JCode, new governance
policies, workbooks. The substrate stays neutral to its future consumers.

## Requirements (REQ-A4S0-NNN)

### Identity

- **REQ-A4S0-001** — `SoftwareRelation` is an ADT
  `{ from: SoftwareEntityRef, kind: CoreRelationKind, to: SoftwareEntityRef }`,
  never a rendered string. Pin: `acceptance_relation_is_an_adt_not_a_rendering`.
- **REQ-A4S0-002** — `RelationId = sha256(domain | from | kind | to)`.
  Pin: `acceptance_relation_id_is_stable_and_content_addressed`.
- **REQ-A4S0-003** — A different semantic relation yields a different id, including
  when only the direction changes. Pin: `acceptance_relation_id_is_direction_sensitive`.
- **REQ-A4S0-004** — `ObservationId = sha256(domain | subject | evidence | origin | basis)`.
  Timestamps, messages, severities and rendered text are excluded from both ids.
  Pin: `acceptance_observation_id_is_clock_stable`,
  `acceptance_identities_exclude_volatile_fields`.
- **REQ-A4S0-005** — The relation kind reuses `CoreRelationKind`; no second relation
  taxonomy is minted. Pin: `acceptance_relation_kind_reuses_core_vocabulary`.

### Provenance

- **REQ-A4S0-006** — `ObservationOrigin`
  (`DeterministicLocal | StaticProvider | RuntimeProvider | HumanDeclared | Inferred`)
  answers *who and how*. Pin: `acceptance_origin_is_closed_and_exhaustive`.
- **REQ-A4S0-007** — The substrate references the existing universal
  `evidence_ref::EvidenceRef`; it adds no third evidence type. Pin:
  `acceptance_reuses_the_universal_evidence_ref`.
- **REQ-A4S0-008** — `ObservationBasis { revision, knowledge_basis, input_digest }`
  records what the observation was taken against. Pin:
  `acceptance_basis_is_recorded`.
- **REQ-A4S0-009** — Freshness reuses the existing KMT vocabulary (`KmtStatus`) when
  an expected basis is supplied, and is **absent** otherwise — never defaulted to
  fresh. Pin: `acceptance_freshness_is_absent_not_assumed`.

### Contradictions

- **REQ-A4S0-010** — Two incompatible observations of one relation coexist; nothing
  overwrites anything. Pin: `acceptance_contradictory_evidence_coexists`.
- **REQ-A4S0-011** — There is no `latest-wins`: order of insertion does not change
  the resolution. Pin: `acceptance_no_latest_wins`.
- **REQ-A4S0-012** — Resolution is a closed ADT, never a confidence number:
  `Supported | Contradicted | Conflicted { supporting, contradicting } |
  Insufficient(gap)`. Pin: `acceptance_resolution_is_closed_and_score_free`.
- **REQ-A4S0-013** — Missing relation or missing provenance is an explicit gap, never
  invented. Pin: `acceptance_missing_relation_stays_unknown`.

### Graph and derivation

- **REQ-A4S0-014** — Observations project into the **one** `SemanticGraphProjection`.
  No second graph, no second DB; any index is rebuildable. Pin:
  `acceptance_observations_project_into_the_one_projection`.
- **REQ-A4S0-015** — A graph rebuild preserves observations semantically. Pin:
  `acceptance_rebuild_preserves_observations`.
- **REQ-A4S0-016** — Derivation is pure: immutable inputs → typed result, with
  deterministic ordering and serde. Pin: `acceptance_derivation_is_deterministic`,
  `acceptance_serde_round_trips`.
- **REQ-A4S0-017** — Read/query paths perform no canonical write. Pin:
  `acceptance_read_paths_do_not_write`.

### Boundary

- **REQ-A4S0-018** — No provider SDK types; no dependency from the substrate toward
  Alignment or Authority. Pin: `acceptance_no_provider_or_alignment_dependency`.
- **REQ-A4S0-019** — `why architecture` closes `evidence → observes → software
  relation` **when** a real observation exists, and keeps its `unresolved_edge`
  **when** it does not. `findings`/`receipt` semantics are unchanged. Pin:
  `why_closes_the_leg_when_evidence_exists`, `why_keeps_the_leg_when_absent`.
- **REQ-A4S0-020** — The A3 baseline stays green. Pin: the whole workspace suite.

## Design sketch

New engine module `crates/sddk-engine/src/observation/` (ADR-0122 names it, as the
root-module fitness rule requires):

```
observation/types.rs       SoftwareEntityRef, SoftwareRelation, RelationId,
                           ObservationId, SoftwareObservation, ObservationOrigin,
                           ObservationBasis, ObservationSet
observation/resolution.rs  EvidenceResolution + resolution over a relation
observation/project.rs     projection into SemanticGraphProjection (rebuildable)
observation/tests.rs
```

`SoftwareEntityRef` reuses `SoftwareUnitRef`, `ComponentRef`, `EntityRef` — no
parallel refs. Contradiction detection is a pure function over an `ObservationSet`;
it never mutates one.

### A4-3R2 — Normative identity-namespace rule (FU-A4-3R-TARGET-NAMESPACE-BRIDGE, closed)

`SoftwareUnitRef`, `ComponentRef`, and `EntityRef` are **three distinct identity
namespaces**. They share the newtype-wrapping convention but no equivalence
semantics: equal inner strings across namespaces do NOT imply subject
equivalence, and any reducer / lookup / projection path that relied on
cross-namespace string equality (`as_str() == as_str()`, `contains`, prefix
stripping, rendered text) was a bug closed by A4-3R2 (release tag pinned in
`docs/debt/FU-A4-3R-TARGET-NAMESPACE-BRIDGE.md`).

Specifically:

- `ObservationSubject::Unit(SoftwareUnitRef)` is the unary subject for AC7
  paradigm-lens targets and **never** names a component or entity.
- `ObservationSubject::Component(ComponentRef)` is the unary subject for
  `ContractPayload::SingleAuthority(ComponentRef)` typed binding.
- `ObservationSubject::Entity(EntityRef)` is the unary subject for
  `ContractPayload::UniqueOwner(EntityRef)` typed binding.
- `ObservationTargetRef` carries the same three unary targets plus
  `Relation` (binary), `Contract`, and `Knowledge`. Their
  `kind_tag()` / `canonical_tag()` always emit a literal namespace prefix
  (`unit:`, `component:`, `entity:`); the prefix is part of identity and is
  never stripped.

**Cross-namespace equivalence requires an explicit typed mapping** (option A in
the FU disposition). Such a mapping is **not** part of A4-3R2; introducing it is
a separate architectural change with its own ADR and scope contract. Within
A4-3R2's scope, no such mapping exists, so observations of a unit can never
collide with a contract naming a component or entity of the same inner string.

The eight falsification pins in `crates/sddk-engine/tests/a4_3r2_namespace_safe_targets.rs`
enforce this rule. The A4-3R corpus (`crates/sddk-engine/tests/a4_3r_typed_constraint_binding.rs`)
pins 4 and 6 are migrated to assert the typed binding (`Component` for
`SingleAuthority`, `Entity` for `UniqueOwner`); pins 12–14 still defend against
substring / rendered-text attacks on the *contract_ref* side and remain green.

## Vocabulary hygiene (FU-A3-CO-2 / FU-A3-CO-3)

A4-0 produces a **disposition** for the three suspect relation kinds and does **not**
apply it, because removal is an AC1 semantics change rather than a
behaviour-preserving cleanup:

| Kind | Producer | Disposition |
|---|---|---|
| `CoreRelationKind::ContractedBy` | none | `REMOVE` (candidate) |
| `CoreRelationKind::SpecifiedBy` (core) | none | `REMOVE` (candidate) |
| `ArchitectureOverlayRelationKind::SpecifiedBy` | `add_contract_metadata` | `KEEP`, `RENAME` to break the collision |

Follow-up gates A4-1: the vocabulary must be settled before generic Verify consumes
it at scale.

## Acceptance tests (planned)

`observation::` — 14 engine pins covering REQ-001..018, plus 2 e2e pins for REQ-019
and the A3 baseline suite for REQ-020.
