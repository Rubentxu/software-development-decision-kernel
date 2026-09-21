---
id: arch-spec-A3-S5-ac4-verify-contracts
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-5-ac4-verify-contracts
source: arch-spec-034-architecture-conformance-verification + ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS + ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY
based_on: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/11-FITNESS-RECEIPTS.md
---

# arch-spec-A3-S5 — Verify Contracts + ArchitectureConformanceDelta

## Intent

AC4 of the Architecture Conformance track (per
`docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/09-ROADMAP.md`):

> Given a change basis, compute affected contracts and execute the minimum
> deterministic probes. Provider-dependent probes may remain unknown.
> **Exit:** `ArchitectureConformanceDelta` with evidence-backed statuses.

This is the **cycle-bounded spec** that supersedes nothing historical but
inherits `arch-spec-034-architecture-conformance-verification.md` (which remains
`status: proposed` upstream — the canonical narrative).

## Upstream requirement coverage (AC-034-001..008)

| Upstream | In scope | Where |
|---|---|---|
| AC-034-001 changed units → affected contracts → minimal probes | yes | REQ-AC4-010..019 |
| AC-034-003 VERIFIED/CONTRADICTED/UNKNOWN/STALE/NOT_EVALUATED | yes | REQ-AC4-004, REQ-AC4-020 |
| AC-034-004 deterministic probes without LLM/provider | yes | REQ-AC4-013, REQ-AC4-022 |
| AC-034-006 Base works with CogniCode/Chronos absent | yes | REQ-AC4-026 |
| AC-034-007 exact revision/contract/graph basis in receipts | partial | REQ-AC4-021..023 |
| AC-034-008 no universal weighted score | yes | REQ-AC4-008, REQ-AC4-024 |
| AC-034-002 DebVerify | no — AC5 | — |
| AC-034-005 LLM/INFERRED assessments | no — AC7 | — |

UAT items: `AC-UAT-007` (missing provider → UNKNOWN), `AC-UAT-008` (only
affected contracts enter scope), `AC-UAT-043` (no fabricated aggregate score).

## Resolved open questions

(Q1, blocking) **New module** `crates/sddk-engine/src/architecture_conformance/`.
AC2's public surface is frozen (A3-S3 closed); AC4 is a new A4-domain
computation. Registration mirrors `paradigm_profile` at `lib.rs`.

(Q2, blocking) `ArchitectureConformanceDelta` is a **PROJECTION-like derived
value** (ADR-0095): reconstructible from graph revision + contract set +
evidence + `now`; never a persistence authority. The module performs no IO.

(Q3, blocking) **CLI surface deferred.** AC4's load-bearing deliverable is the
engine capability and its typed delta. The CLI rendering (AC-UAT-043) is a
scoped follow-up; the vector is designed to be renderable without a score.

(Q4) AC4 does not import AC3's `EvidenceBasis`; statuses are `ClaimOutcome`-derived.

(Q5) All collections sorted (`BTreeMap` / sorted `Vec`); two evaluations over
identical inputs yield byte-identical `plan_digest`.

## Implementation refinements (applied during Build)

Three refinements were made while implementing, because the AC1 substrate
constrained the shape. They are recorded here so the spec matches the code.

- **R-A (REQ-AC4-005)** — `AffectedContract.contract_kind` is
  `Option<ContractKind>`. `None` means the contract is reachable from the graph
  but no contract object was supplied; its status is `NotEvaluated`. A non-
  optional kind would have forced a fabricated kind.

- **R-B (REQ-AC4-019, new REQ-AC4-030)** — `Contradicted` is carried by a new
  delta-level field `ArchitectureConformanceDelta.contradictions: Vec<ContractId>`
  rather than by an AC1 claim. Reason: AC1's `ContractEvaluation::evaluate`
  deliberately never produces `Contradicted` ("A4 Verify is responsible for
  `Contradicted`"). AC4 therefore accepts `contradiction_witnesses` as an input
  (the seam the AC6 mutation probes will drive) and records them without ever
  constructing a claim itself — preserving REQ-AC4-028. `claims` holds only
  `evaluate()` outputs (`Verified` / `Unknown` / `Stale`).

- **R-C (REQ-AC4-013)** — `compute_conformance_delta` takes the basis as a
  single `ConformanceInputs { contracts, evidence, contradiction_witnesses }`
  value instead of three positional arguments.

## Scope (must)

- Declare `DeltaContractStatus` as a closed enum of 5 variants mirroring
  AC-034-003 semantics.
- Declare `ProbeRequirement` as a closed enum mapping each `ContractKind` to a
  minimal deterministic probe, plus an `Unknown` fallback.
- Declare `AffectedContract` (contract kind + triggering units + probe plan).
- Declare `ArchitectureConformanceDelta` (affected, claims, unknowns,
  contradictions, stale, not_evaluated, vector, plan_digest,
  contract_set_digest, graph_digest, evaluated_at).
- Declare `ConformanceVector` with 7 named dimensions, each a `VectorStatus`.
- Implement a pure `compute_conformance_delta(overlay, contracts, evidence, now)`
  that resolves affected contracts through the AC2 overlay public API and
  evaluates each via `ContractEvaluation::evaluate` (AC1).
- Deterministic digest over the probe plan and over the contract set.
- Provider-dependent / not-supplied contracts become `NotEvaluated` (AC-UAT-007).

## Scope (must NOT)

- NO import of `paradigm_profile` (AC3) — lens evaluation is AC7.
- NO import of `alignment` / `debverify` (AC5 surfaces).
- NO import of `provider` / `host_sdk` / `agent_host` (AC-034-006).
- NO write to the semantic graph — AC4 is a read-only consumer of AC2.
- NO numeric aggregate score anywhere in the public types (AC-034-008).
- NO direct construction of `ArchitectureClaim` — always via
  `ContractEvaluation::evaluate` (AC1 invariant).
- NO requirement on CogniCode (AC10) or Chronos (AC11).

## Requirements (REQ-AC4-NNN format)

### Status vocabulary

- **REQ-AC4-001** — `DeltaContractStatus` is a closed enum with exactly 5
  variants: `Verified | Contradicted | Unknown | Stale | NotEvaluated`,
  matching AC-034-003.
  Pin test: `DeltaContractStatus::ALL.len() == 5`.

- **REQ-AC4-002** — `DeltaContractStatus::from_claim_outcome(ClaimOutcome)`
  maps `Verified→Verified`, `Contradicted→Contradicted`, `Unknown→Unknown`,
  `Stale→Stale`. `NotEvaluated` is produced only when no contract object was
  supplied for an affected contract id.
  Pin test: `status_from_outcome_is_total`.

- **REQ-AC4-003** — `DeltaContractStatus` is `Ord` and has a canonical tag for
  receipts (`"verified" | "contradicted" | "unknown" | "stale" | "not_evaluated"`).
  Pin test: `status_tags_unique`.

- **REQ-AC4-004** — A status is never fabricated: an affected contract with no
  evidence evaluates to `Unknown`, never `Verified`.

### Delta shape

- **REQ-AC4-005** — `AffectedContract { contract_kind: ContractKind,
  triggering_units: Vec<SoftwareUnitRef>, probes: Vec<ProbeRequirement> }`;
  `triggering_units` is sorted and deduplicated.
  Pin test: `acceptance_triggering_units_sorted_dedup`.

- **REQ-AC4-006** — `ArchitectureConformanceDelta.affected` is a
  `BTreeMap<ContractId, AffectedContract>` (deterministic iteration order).
  Pin test: `acceptance_affected_is_sorted`.

- **REQ-AC4-007** — `ArchitectureConformanceDelta.claims` is a
  `BTreeMap<ContractId, ArchitectureClaim>`; exactly one claim per evaluated
  contract, none for `NotEvaluated`.
  Pin test: `acceptance_one_claim_per_evaluated_contract`.

- **REQ-AC4-008** — The delta exposes `unknowns()`, `contradictions()`,
  `stale()`, `not_evaluated()` as sorted `Vec<ContractId>` accessors.
  **No numeric aggregate score method or field exists.**
  Pin test: `anti_encroachment_no_numeric_score_field` (compile-time/structural).

### Probe plan

- **REQ-AC4-009** — `ProbeRequirement` is a closed enum with exactly 8 variants:
  `OwnershipProbe | DependencyProbe | ProjectionProbe | CompatibilityWindowProbe |
  ProviderBoundaryProbe | ExtensionProbe | Unknown`.
  (7 concrete + `Unknown`.)
  Pin test: `ProbeRequirement::ALL.len() == 8`.

- **REQ-AC4-010** — `ProbeRequirement::for_contract_kind(ContractKind)` is a
  total, deterministic mapping:
  `SingleAuthority→Ownership`, `UniqueOwner→Ownership`,
  `ForbiddenDependency→Dependency`, `ProjectionOnly→Projection`,
  `BoundedCompatibility→CompatibilityWindow`, `ProviderBoundary→ProviderBoundary`,
  `Extension(_)→Extension`.
  Pin test: `probe_mapping_is_total_and_stable`.

- **REQ-AC4-011** — The probe plan for an affected contract is the deduplicated,
  sorted set of `ProbeRequirement`s of the affected contract kinds that map to
  that contract's id.
  Pin test: `acceptance_probe_plan_dedup`.

- **REQ-AC4-012** — `plan_digest` is sha256 over the canonical serialization of
  the affected map (contract ids, kinds, sorted triggering units, sorted probes).
  Two deltas over identical inputs MUST have equal `plan_digest`.
  Pin test: `acceptance_plan_digest_deterministic`.

### Computation

- **REQ-AC4-013** — `compute_conformance_delta` is **pure**: no IO, no clock
  reads, `now` is a parameter (`EventTime`).
  Pin test: `acceptance_compute_is_deterministic`.

- **REQ-AC4-014** — Affected contracts are discovered via the AC2 overlay public
  API: for each changed unit, `overlay.find_contracts_for_unit(unit)`; the
  anchor `NodeId` is resolved to a `ContractId` via
  `overlay.projection()` `props_inline["contract_id"]`.
  Pin test: `acceptance_affected_resolved_from_overlay`.

- **REQ-AC4-015** — Only contracts reachable from the **changed** units enter
  the delta (AC-UAT-008). A contract constraining an unchanged unit is absent.
  Pin test: `acceptance_only_changed_units_enter_scope`.

- **REQ-AC4-016** — An affected contract id with no supplied
  `ArchitecturalContract` object yields `NotEvaluated` (no claim), and is listed
  in `not_evaluated()`.
  Pin test: `acceptance_missing_contract_object_is_not_evaluated`.

- **REQ-AC4-017** — An affected contract with an object but empty evidence
  evaluates to `Unknown` (via AC1 rule 1), never `Verified`.
  Pin test: `acceptance_empty_evidence_is_unknown` (AC-UAT-007).

- **REQ-AC4-018** — An affected, supplied contract with valid evidence
  evaluates to `Verified` through `ContractEvaluation::evaluate`; the claim's
  `evaluator()` reflects the AC1 evaluator (not invented by AC4).
  Pin test: `acceptance_valid_evidence_is_verified`.

- **REQ-AC4-019** — An affected contract whose evidence contradicts it yields
  `Contradicted` and appears in `contradictions()`.
  Pin test: `acceptance_contradicted_listed`.

- **REQ-AC4-020** — `contract_set_digest` is sha256 over the sorted supplied
  contract basis hashes; two deltas with the same contract basis produce the
  same digest even when their evidence differs.
  Pin test: `acceptance_contract_set_digest_stable_across_evidence`.

- **REQ-AC4-021** — `graph_digest` equals `overlay.digest()` at compute time.
  Pin test: `acceptance_graph_digest_matches_overlay`.

- **REQ-AC4-022** — No provider call, no LLM call, no network occurs during
  `compute_conformance_delta` (AC-034-004). Pin: the module has no provider
  imports (source-grep).

### No-score discipline

- **REQ-AC4-023** — `ConformanceVector` exposes exactly 7 named dimension
  accessors: `authority`, `ownership`, `dependencies`, `compatibility`,
  `negative_paths`, `paradigm_alignment`, `runtime_evidence`. No index/score.
  Pin test: `acceptance_vector_has_seven_dimensions`.

- **REQ-AC4-024** — `VectorStatus` is a closed enum of exactly 5 variants
  (`Verified | Partial | Unknown | Contradicted | NotEvaluated`); there is no
  numeric weight and no aggregate score (AC-034-008, AC-UAT-043).
  Pin test: `acceptance_vector_status_is_closed`.

- **REQ-AC4-025** — `paradigm_alignment` and `runtime_evidence` are
  `NotEvaluated` in AC4 (owned by AC7 / AC11).
  Pin test: `acceptance_ac7_ac11_dimensions_not_evaluated`.

### Anti-encroachment

- **REQ-AC4-026** — `architecture_conformance` source does NOT `use` any of
  `alignment`, `debverify`, `provider`, `host_sdk`, `agent_host`,
  `paradigm_profile`, `effective_instructions`, `capability`.
  Pin test: `anti_encroachment_no_forbidden_imports`.

- **REQ-AC4-027** — `architecture_conformance` does NOT mutate the semantic
  graph: the only AC2 method invoked on the overlay is a read
  (`find_contracts_for_unit`, `projection`, `digest`).
  Pin test: `anti_encroachment_read_only_overlay`.

- **REQ-AC4-028** — `architecture_conformance` does NOT construct
  `ArchitectureClaim` directly; it must route through
  `ContractEvaluation::evaluate`.
  Pin test: `anti_encroachment_no_claim_construction`.

- **REQ-AC4-029** — ADR-0112 implementation evidence is updated to cite the
  AC4 module; `arch-spec-034` upstream remains `status: proposed`.
  Pin: doc check (ADR mirror).

## Acceptance tests (planned, 29)

| # | Test | REQ |
|---|---|---|
| 1 | `acceptance_status_from_claim_outcome_is_total` | 002 |
| 2 | `acceptance_status_tags_unique` | 003 |
| 3 | `acceptance_delta_status_closed_five` | 001 |
| 4 | `acceptance_probe_requirement_closed_eight` | 009 |
| 5 | `acceptance_probe_mapping_total_stable` | 010 |
| 6 | `acceptance_probe_mapping_covers_all_contract_kinds` | 010 |
| 7 | `acceptance_triggering_units_sorted_dedup` | 005 |
| 8 | `acceptance_affected_is_sorted` | 006 |
| 9 | `acceptance_one_claim_per_evaluated_contract` | 007 |
| 10 | `acceptance_unknowns_contradictions_stale_sorted` | 008 |
| 11 | `acceptance_probe_plan_dedup` | 011 |
| 12 | `acceptance_plan_digest_deterministic` | 012 |
| 13 | `acceptance_compute_is_deterministic` | 013 |
| 14 | `acceptance_affected_resolved_from_overlay` | 014 |
| 15 | `acceptance_only_changed_units_enter_scope` | 015 |
| 16 | `acceptance_missing_contract_object_is_not_evaluated` | 016 |
| 17 | `acceptance_empty_evidence_is_unknown` | 017 |
| 18 | `acceptance_valid_evidence_is_verified` | 018 |
| 19 | `acceptance_contradicted_listed` | 019 |
| 20 | `acceptance_contract_set_digest_stable_across_evidence` | 020 |
| 21 | `acceptance_graph_digest_matches_overlay` | 021 |
| 22 | `acceptance_vector_has_seven_dimensions` | 023 |
| 23 | `acceptance_vector_status_is_closed` | 024 |
| 24 | `acceptance_ac7_ac11_dimensions_not_evaluated` | 025 |
| 25 | `anti_encroachment_no_forbidden_imports` | 026 |
| 26 | `anti_encroachment_read_only_overlay` | 027 |
| 27 | `anti_encroachment_no_claim_construction` | 028 |
| 28 | `anti_encroachment_no_numeric_score_field` | 008/024 |
| 29 | `bonus_empty_changed_units_yields_empty_delta` | boundary |

## Determinism contract

Two `compute_conformance_delta` calls over identical
`(overlay, contracts, evidence, now)` MUST yield:
- identical `plan_digest`,
- identical `contract_set_digest`,
- identical `graph_digest`,
- identical `affected` ordering and `claims` key set.

## State classes (ADR-0095)

| Value | Class |
|---|---|
| `ArchitectureConformanceDelta` | PROJECTION (derived, reconstructible, no identity) |
| `AffectedContract`, `ProbeRequirement`, `ConformanceVector`, `VectorStatus`, `DeltaContractStatus` | EPHEMERAL (in-memory computation values) |
| `ArchitecturalContract` | OBJECT (owned by AC1; AC4 reads only) |
| `ArchitectureClaim` | PROJECTION (owned by AC1; AC4 reads only) |
