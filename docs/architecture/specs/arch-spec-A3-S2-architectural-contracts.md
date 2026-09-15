---
id: arch-spec-A3-S2-architectural-contracts
status: proposed
proposed_at: 2026-09-15
source: ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS, arch-spec-032-architectural-contracts
cycle: p-63676b11dc0ef88f/a3-2-architectural-contract
supersedes_history: false
---

# arch-spec-A3-S2 — Architectural Contracts (cycle-bounded)

This is the **cycle-bounded spec** for A3-S2. The canonical spec lives in
`docs/architecture/specs/arch-spec-032-architectural-contracts.md` (status:
proposed). This file pins the concrete scope, REQs, acceptance, negative
fixtures and anti-encroachment probes for the cycle
`p-63676b11dc0ef88f/a3-2-architectural-contract`.

## Intent

Implement AC1: typed `ArchitecturalContract` and `ArchitectureClaim` types that
make architecture intent machine-verifiable, linkable to Decision/Spec
provenance, and queryable through the SemanticGraph.

## Source of truth

- ADR-0112 (proposed) — `docs/architecture/adrs/ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS.md`
- arch-spec-032 (proposed) — `docs/architecture/specs/arch-spec-032-architectural-contracts.md`
- A3-S1 substrate — `docs/architecture/specs/arch-spec-A3-S1-knowledge-substrate.md`
  (BasisHash reuse)

## State classes (per ADR-0095)

- `ArchitecturalContract` — **OBJECT** (durable, revisioned, basis-addressed).
- `ArchitectureClaim` — **PROJECTION** (rebuildable from contract + evidence + now).
- `ContractEvaluation`, `evaluate(...)` — **EPHEMERAL** (pure transforms).
- `ContractId`, `ContractKind`, `ContractPayload`, `DecisionRef`, `SpecRef`,
  `Revision`, `ClaimOutcome`, `EvaluatorRef`, `EvidenceRef` — **variant enums
  and typed newtypes**, exhaustive.

## Type contract

```text
ArchitecturalContract
├── id: ContractId              (typed newtype over String)
├── kind: ContractKind          (closed enum, 6 variants + Extension seam)
├── payload: ContractPayload    (typed by kind; sum type)
├── basis_hash: BasisHash        (reuse from S1, domain-separated)
├── decided_by: DecisionRef      (Decision | ADR | ExternalDecision)
├── specified_by: SpecRef        (SpecId | AdrId | ArchSpecId)
├── revision: Revision           (typed newtype over String)
└── declared_at: EventTime       (typed newtype over i64)

ContractKind (closed enum)
├── SingleAuthority             (one Component owns this capability)
├── UniqueOwner                 (one Entity owns this resource)
├── ForbiddenDependency         (dep A → B is forbidden)
├── ProjectionOnly              (entity is rebuilt, not stored)
├── BoundedCompatibility        (compat window has upper bound)
├── ProviderBoundary            (no inward provider SDK types)
└── Extension(NamespacedKind)   (custom seam)

ContractPayload (sum type)
├── SingleAuthority(ComponentRef)
├── UniqueOwner(EntityRef)
├── ForbiddenDependency { from: ComponentRef, to: ComponentRef, reason: String }
├── ProjectionOnly { source_kind: CoreNodeKind | NamespacedKind }
├── BoundedCompatibility { deprecated_after: EventTime, replaced_by: ContractId }
├── ProviderBoundary { boundary: BoundaryKind, surface: String }
└── Extension { kind: NamespacedKind, fields: BTreeMap<String, Value> }

ArchitectureClaim
├── contract_id: ContractId
├── outcome: ClaimOutcome       (Verified | Contradicted | Unknown | Stale)
├── evidence_refs: Vec<EvidenceRef>
├── evaluated_at: EventTime
├── evaluator: EvaluatorRef
├── missing_evidence: Vec<MissingEvidence>   (reuse from S1 enum)
└── note: Option<String>                   (rationale, NOT authority)
```

## Requirements (22 REQs)

### Identity & determinism

- **REQ-A3S2-001**: `ArchitecturalContract` is constructible only via
  `ArchitecturalContract::declare(...)`; private fields, no public struct
  literal construction outside the module.
- **REQ-A3S2-002**: `basis_hash` is a deterministic function of
  `(id, kind, payload, decided_by, specified_by, revision)` with the same
  domain separation prefix as `KnowledgeAssertion::basis_hash`.
- **REQ-A3S2-003**: `ArchitectureClaim` is constructible only via
  `ContractEvaluation::evaluate(contract, evidence, now)`.
- **REQ-A3S2-004**: serde round-trip preserves identity (id, basis_hash, kind,
  payload, refs, revision).

### Kind/payload coupling

- **REQ-A3S2-005**: `ContractKind` and `ContractPayload` are coupled at the
  type level. There is no free-form `(kind, payload)` tuple outside the typed
  constructors.
- **REQ-A3S2-006**: `ContractPayload::Extension` accepts only `NamespacedKind`
  matching `lowercase_ascii.kind` shape.
- **REQ-A3S2-007**: `Extension` payload must use `BTreeMap<String, Value>` to
  preserve insertion-order-independent hashing.

### Provenance links

- **REQ-A3S2-008**: `decided_by` MUST point to an existing
  `Decision | ADR | ExternalDecision` reference (typed `DecisionRef` enum).
- **REQ-A3S2-009**: `specified_by` MUST point to an existing
  `SpecId | AdrId | ArchSpecId` reference (typed `SpecRef` enum).
- **REQ-A3S2-010**: missing `decided_by` or `specified_by` → constructor
  fails closed with `ContractError::MissingProvenance`.

### Claim outcomes

- **REQ-A3S2-011**: `ClaimOutcome` is closed: `Verified | Contradicted | Unknown | Stale`.
- **REQ-A3S2-012**: empty `evidence_refs` ⇒ `outcome = Unknown`,
  `missing_evidence` contains `NotProvided`.
- **REQ-A3S2-013**: `now < declared_at` ⇒ `outcome = Stale`.
- **REQ-A3S2-014**: `BoundedCompatibility` with `now >= deprecated_after` and
  no replacement ⇒ `outcome = Stale`.

### Capability isolation

- **REQ-A3S2-015**: `ArchitecturalContract` has NO `grants` field, NO
  `CapabilityId` reference, NO mutation surface. Capability grants are an
  Authority engine concern (ADR-0102), not a contract concern.
- **REQ-A3S2-016**: `Extension(NamespacedKind)` cannot be `provider.<sdk>` per
  ADR-0099; constructor rejects.

### SemanticGraph surface

- **REQ-A3S2-017**: new `CoreNodeKind::ArchitecturalContract` (19th).
- **REQ-A3S2-018**: new `CoreRelationKind::ContractedBy` and
  `CoreRelationKind::SpecifiedBy` (15th and 16th).
- **REQ-A3S2-019**: `ALL` arrays, `domain_tag()` matches and doc-comments
  are updated atomically with the enum extension.
- **REQ-A3S2-020**: `core_relation_kinds_have_14_entries_after_evidence_relations`
  test is updated to 16 + renamed; the prior count is preserved as a
  `_before_ac1` test that asserts the historical baseline.

### Anti-encroachment (compile-time)

- **REQ-A3S2-021** (anti): `architectural_contract.rs` imports NO A4 types
  (`sddk_engine::alignment::*`, `sddk_engine::verification::*`,
  `sddk_engine::governance::*`) and NO provider SDK types. Pinned by a
  `include_str!` + grep test.
- **REQ-A3S2-022** (anti): `ArchitectureClaim` has NO `markdown_payload`,
  `parse_from_markdown`, or `from_str_md` methods. Markdown is not runtime
  authority (per AC-032-006).

## Acceptance (11 tests)

| ID | Test name | What it proves |
|---|---|---|
| ACC-01 | `contract_identity_includes_id_kind_payload_refs_revision` | hash covers all identity fields |
| ACC-02 | `basis_hash_is_deterministic_for_same_inputs` | same inputs ⇒ same hash |
| ACC-03 | `basis_hash_differs_on_payload_change` | hash changes when payload changes |
| ACC-04 | `basis_hash_differs_on_decided_by_change` | hash changes when ref changes |
| ACC-05 | `kind_payload_coupling_is_type_safe` | cannot construct SingleAuthority with ForbiddenDependency payload |
| ACC-06 | `extension_payload_uses_namespaced_kind_and_btreemap` | custom seam works |
| ACC-07 | `extension_payload_rejects_provider_namespace` | `provider.<x>` rejected |
| ACC-08 | `claim_outcome_is_unknown_when_evidence_is_empty` | REQ-A3S2-012 |
| ACC-09 | `claim_outcome_is_stale_when_now_precedes_declared_at` | REQ-A3S2-013 |
| ACC-10 | `claim_outcome_is_stale_when_deprecated_after_elapsed` | REQ-A3S2-014 |
| ACC-11 | `serde_roundtrip_preserves_contract_identity_and_basis_hash` | identity survives serialization |

## Negative fixtures (4 tests)

| ID | Test name | What it proves |
|---|---|---|
| NEG-01 | `cannot_build_contract_without_decided_by` | REQ-A3S2-010 |
| NEG-02 | `cannot_build_contract_without_specified_by` | REQ-A3S2-010 |
| NEG-03 | `cannot_claim_verified_with_empty_evidence` | REQ-A3S2-012 |
| NEG-04 | `extension_payload_rejects_non_namespaced_kind` | REQ-A3S2-006 |

## Anti-encroachment probes (4 tests)

| ID | Test name | What it proves |
|---|---|---|
| AENC-01 | `s2_module_has_no_a4_or_provider_imports` | REQ-A3S2-021 |
| AENC-02 | `s2_does_not_grant_capabilities` | REQ-A3S2-015 (grep for `grants`, `CapabilityId`, `granted_by`) |
| AENC-03 | `s2_markdown_is_not_runtime_authority` | REQ-A3S2-022 |
| AENC-04 | `s2_corenodekind_baseline_is_19_after_ac1` | REQ-A3S2-017..020 (counts pinned) |

## Crosswalks (5)

1. ADR-0112 ↔ `ArchitecturalContract` fields (1:1 mapping of decision concepts).
2. arch-spec-032 AC-032-001..007 ↔ REQ-A3S2-001..022 (every spec REQ has an
   implementable cycle REQ; gaps listed).
3. A3-S1 `BasisHash` ↔ A3-S2 `ArchitecturalContract::basis_hash` (same SHA-256
   prefix `sddk.knowledge.assertion.v1\n` extended with a contract sub-prefix
   `sddk.architectural_contract.v1\n`).
4. CoreNodeKind baseline 18 → 19 (architectural_contract).
5. CoreRelationKind baseline 14 → 16 (contracted_by, specified_by).

## Receipt and verification

- Implementation receipt at
  `docs/SDDK-Production-Readiness-Alignment-2026-09-14/A3-S2-AC1-RECEIPT.md`.
- Independent verification report at
  `.sddk/cycles/p-63676b11dc0ef88f-a3-2-architectural-contract/verification-report.md`.
- Targeted test run: `cargo test -p sddk-engine --lib knowledge:: architectural_contract:: semantic_kind::`
  (scoped to the three modules touched; full workspace reserved for release).

## Out of scope (deferred, anti-encroachment)

- ✗ `ArchitecturalContract` membership in the SemanticGraph overlay (S3).
- ✗ `ParadigmProfile` integration (S4).
- ✗ `ContextCapsule.advisory_context` (S5).
- ✗ Verify / DebVerify / ConformanceDelta (A4, AC4, AC5).
- ✗ Paradigm lenses (AC7).
- ✗ Mutation probes (AC6).
- ✗ Markdown ingestion CLI (`sddk ingest contract.md`); AC-032-006 keeps
  semantic object as authority; ingestion deferred to AC8.
- ✗ Provider SDK types in payload (provider-boundary rule enforced by AENC).
- ✗ Capability grants (REQ-A3S2-015 forbids it; ADR-0102 owns capability semantics).
- ✗ Promoting ADR-0112 / arch-spec-032 to `accepted` (separate commit/cycle,
  not S2 deliverable).

## Authoring instructions for the apply agent

- Single new module: `crates/sddk-engine/src/architectural_contract.rs`
  (≤ 1200 LoC expected; split to `mod.rs` + submodules only if exceeded,
  per the A3-S1 rule of thumb).
- Edit `semantic_kind.rs` (add 1 variant + 2 variants, update arrays,
  domain_tag matches, doc-comment counts, rename the 14-entries test).
- Edit `knowledge.rs` (rename `s1_does_not_introduce_new_corenodekind_variants`
  to a historical anchor; add a complementary S2 pin).
- Edit `lib.rs` (add `pub mod architectural_contract;`).
- Use `BTreeMap` everywhere order-independence matters; `HashMap` only inside
  `serde_json::Value` payloads (justified by S1 spec).
- All four anti-encroachment tests are pinned as `#[test]` with `include_str!`
  + grep, NOT as lint annotations.
