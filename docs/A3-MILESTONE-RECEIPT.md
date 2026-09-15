# A3 MILE-STONE RECEIPT — Knowledge + Agent Advisory

> **Milestone:** A3 — R2 + R3 Knowledge + advisory foundation (roadmap P1)
> **Normative source:** `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md` §A3
> **Closeout cycle:** `p-63676b11dc0ef88f/a3-closeout-milestone-reconciliation`
> **Certified revision:** ``98b7fc78d39aa1deab4c37a1c0273d20a8bdb388` (`98b7fc7`)`
> **Version / tag:** `1.169.39` / `v1.169.39`
> **Method:** roadmap → code → tests → receipts → missing requirements. Only genuinely
> absent requirements were implemented (roadmap-driven, not rewrite-driven).

## Verdict

**A3: PASS** — every A3 requirement is `PASS` or `PASS_WITH_COMPAT`. **No unresolved MUST.**

Two requirements carry a declared compatibility note rather than a bare `PASS`;
both are recorded in §3 with the reason and the remaining scope.

## 1. Status legend

| Status | Meaning |
|---|---|
| `PASS` | Implemented, tested, receipted — scope matches the requirement. |
| `PASS_WITH_COMPAT` | Implemented with a **declared** narrower scope, deviation, or open compatibility note. |

## 2. Crosswalk

### 2.1 Knowledge

| # | Requirement | Implementation | Tests | Receipt / cycle | Status |
|---|---|---|---|---|---|
| K1 | `KnowledgeAssertion` | `sddk-engine/src/knowledge.rs` — `declare`, `id`, `basis_hash`, `declared_at`, `kind`, `payload` | `knowledge` suite | A3-S1 | **PASS** |
| K2 | `KnowledgeBasis` | `knowledge.rs` — `empty`, `insert`, `revise`, `invalidate`, `assertions`, `basis_hash`, `revised_at` | `knowledge` suite | A3-S1 | **PASS** |
| K3 | deterministic `BasisHash` | `knowledge.rs` `BasisHash` over canonically-tagged kind + payload | `knowledge` suite | A3-S1 | **PASS** |
| K4 | KMT identity | `KnowledgeId` + `BasisHash`; assertion identity is the pair | `knowledge` suite | A3-S1 | **PASS** |
| K5 | freshness | `KmtStatus` (`Fresh` / `Stale` / `Invalidated` / `Unknown`) via `KMT::evaluate` | `knowledge` suite | A3-S1 | **PASS** |
| K6 | invalidation | `KnowledgeBasis::invalidate` + closed `InvalidationReason` | `knowledge` suite | A3-S1 | **PASS** |
| K7 | SemanticGraph cross-tree overlay | `knowledge::project_into` — every assertion becomes a node in the canonical `InMemorySemanticGraph` type, carrying `basis_hash` / `knowledge_kind` / `payload_kind` / `declared_at` | `acceptance_knowledge_projection_is_deterministic_and_rebuildable`, `acceptance_knowledge_shares_the_one_projection_abstraction` | **A3 closeout** | **PASS_WITH_COMPAT** (§3.1) |
| K8 | rebuild equivalence | `architecture_graph::rebuild` (REQ-AC2-006) + `canonical_bytes`; `acceptance_rebuild_is_equivalent_and_keeps_specified_by`, `acceptance_clock_is_excluded_from_semantic_identity_not_hidden` | `architecture_graph` suite | A3-S3 + **closeout** | **PASS** |

### 2.2 Progressive knowledge

| # | Requirement | Implementation | Tests | Receipt / cycle | Status |
|---|---|---|---|---|---|
| P1 | Software Unit cards / progressive disclosure | `architecture_graph/card.rs` — `SoftwareUnitCard` with identity, kind, locator, module_path, `purpose`, `owned_by`, `dependencies`, `relevant_contracts`, `relevant_decisions`, `relevant_specs`, `evidence_refs`, `graph_refs`, `knowledge_status`, `provenance` | `acceptance_card_is_bounded_and_provenanced`, `acceptance_card_freshness_uses_the_real_kmt_when_evaluable`, `acceptance_card_is_deterministic_and_absent_unit_is_none` | **A3 closeout** | **PASS** |
| P2 | bounded retrieval rather than whole-graph dumps | the card (`card_for_unit`) returns one unit's slice; `architecture graph --scope` narrows by locator prefix | card tests + `architecture_read_cli_e2e` | A3-S11 + **closeout** | **PASS** |
| P3 | provenance/freshness retained through the card | `CardProvenance` (`Declared` / `Observed` / `Absent`) per card; `freshness` from the real `KMT` when an expected basis is supplied, `None` = **not evaluated** (never defaulted to fresh) | card tests | **A3 closeout** | **PASS** |

### 2.3 Agent advisory boundary

| # | Requirement | Implementation | Tests | Receipt / cycle | Status |
|---|---|---|---|---|---|
| A1 | `advisory_context` | `sddk-domain/src/workflow_run.rs` — `AdvisoryContext` (closed `AdvisoryKind` × `AdvisoryProvenance`) on `ContextCapsule` | `acceptance_advisory_context_does_not_change_instruction_identity` | M7.5 + **closeout** | **PASS** |
| A2 | separate instruction hash | `ContextCapsule::effective_instruction_set_hash` vs `context_capsule_hash`; `recompute_hash` takes the instruction hash as an **input** (no second compiler in the delivery path) | same + `acceptance_capsule_hash_is_order_invariant_and_validates` | **A3 closeout** | **PASS** |
| A3 | advisory changes capsule identity, never instruction identity | proven against the **real** `InstructionCompiler` output: same `content_hash`, different `context_capsule_hash` | `acceptance_advisory_context_does_not_change_instruction_identity` | **A3 closeout** | **PASS** |
| A4 | advisory ≠ `InstructionSource` | no conversion exists; the compiler never reads advisory types; not an `InstructionSource` variant | `acceptance_advisory_is_not_instruction_source_capability_or_authority` | **A3 closeout** | **PASS** |
| A5 | advisory ≠ `Capability` | no path from advisory to any capability requirement | same | **A3 closeout** | **PASS** |
| A6 | advisory cannot influence `AuthorityEngine` except as ordinary explicit facts | the authority engine takes explicit facts only; the compiler and the capsule do not feed it | same | **A3 closeout** | **PASS** |

### 2.4 Architecture Conformance A3 items

| # | Requirement | Implementation | Tests | Receipt / cycle | Status |
|---|---|---|---|---|---|
| AC1 | typed `ArchitecturalContract` / `ArchitectureClaim` | `sddk-engine/src/architectural_contract/` | `architectural_contract` + `architecture_conformance` suites | A3-S2 | **PASS** |
| AC2 | architecture nodes/relations in the one projection | `sddk-engine/src/architecture_graph/` — 8 node kinds, **15** relation kinds, routed through `Extension` per REQ-AC2-003/004 | `architecture_graph` suite | A3-S3 (+S15) | **PASS_WITH_COMPAT** (§3.2) |
| AC3 | scoped `ParadigmProfile` (OO, FP, ADT, DSL, extensible) | `paradigm_profile/` + `paradigm_lens/` | `paradigm_profile` + `paradigm_lens` suites | A3-S4, A3-S8 | **PASS** |

## 3. Compatibility notes (the two non-bare passes)

### 3.1 K7 — the cross-tree overlay projects nodes, not payload edges

Delivered: assertions are projected into the canonical projection **type**
(`InMemorySemanticGraph`), deterministically and rebuildably, each node naming its
basis and its kinds.

**Not delivered:** edges decoded from `KnowledgePayload::Relation`. Those bytes are
opaque and carry an arbitrary `content_type`; turning them into graph edges requires
a **documented canonical encoding** that does not exist. Inventing one in a closeout
would fabricate provenance — the same reason `why architecture` refuses to synthesise
its missing `evidence → observes → software` leg.

Also recorded: the AC2 overlay exposes its projection **read-only** ("so the overlay
cannot leak its store"), so knowledge nodes cannot be injected into an overlay
*instance*. The single-projection rule is therefore satisfied at the abstraction
level (one graph type, no second graph, no second DB), which is what ADR-0098
requires, rather than by one shared instance.

Remaining scope: define a canonical relation encoding, then emit edges.

### 3.2 AC2 — dead core vocabulary and a name collision

Two findings, recorded as observations rather than resolved here:

- **`CoreRelationKind::{ContractedBy, SpecifiedBy}` are never emitted.** Added by
  AC1 (14 → 16) and pinned by `s2_post_ac1_corenodekind_baseline`, but nothing
  produces them: the unit↔contract linkage AC2 uses is `ArchitectureClaimedBy` plus
  the contract metadata node, with `find_units_contracted_by` computing the inverse.
- **Two relations share the name `SpecifiedBy` with opposite directions.** Core
  `specified_by` is node → contract ("the contract specifies this node"); AC2's
  `ac2_rel_specified_by` is contract → spec document. AC2 *must* use extension kinds
  (REQ-AC2-004), so both are individually correct; the shared name is the hazard.

Neither blocks A3. Both are AC1/AC2 semantics decisions for A4.

## 4. Early-delivered future milestones

Recorded explicitly, because it explains why A3 looks oversized:

| Artifact | Delivered early in | Placement in the roadmap |
|---|---|---|
| AC4 — architecture Verify | **A3-S5** | A4 |
| AC5 — architecture DebVerify | **A3-S7** | A4 |
| AC6 — critical mutation probes | **A3-S6** | A4 |
| AC7 — paradigm lenses | **A3-S8** | A4 |
| AC8 — self-audit receipt | **A3-S9** | A5 |

Their implementation remains valid and is in production.

**Early delivery does NOT close A4 or A5.** The distinctions that must not be lost:

| This exists | It is **not** |
|---|---|
| `sddk architecture receipt` gating on AC1–AC5 | all of Verify |
| `sddk architecture findings` (AC5 challenge classes) | all of DebVerify |
| `paradigm_lens` (OO / FP / ADT / DSL probes) | complete Software Alignment |
| `ArchitectureConformanceReceipt` (AC8) | `BASE_PRODUCTION_READY` |
| AC6's sandbox over four critical mutations | the whole mutation-testing space |

A4 still owns the Base intelligence loop (Sources + Decision Memory → Knowledge/KMT +
SemanticGraph → Alignment → Verify/DebVerify → Evidence + receipts), Alignment as a
first-class advisory producer, and verify/deb-verify semantics beyond architecture.
A5 still owns the hardening programme and the two production receipts.

## 5. Roadmap exit criteria

| Criterion | Status | Evidence |
|---|---|---|
| deterministic graph rebuild/freshness fixtures | **PASS** | `acceptance_rebuild_is_equivalent_and_keeps_specified_by`, `acceptance_clock_is_excluded_from_semantic_identity_not_hidden`, `acceptance_knowledge_projection_is_deterministic_and_rebuildable`, card freshness tests |
| Alignment-like advisory payload never changes `EffectiveInstructions` | **PASS** | `acceptance_advisory_context_does_not_change_instruction_identity` (against the real compiler) |

Clock-sensitive differences are **not hidden**: `acceptance_clock_is_excluded_from_semantic_identity_not_hidden`
records where the clock lives and proves the relation structure is clock-independent,
rather than canonicalising the difference away.

## 6. What the closeout implemented vs found

**Already implemented (verified, not rewritten):** K1–K6, K8, the AC1/AC2/AC3 core,
and all of AC4–AC8.

**Implemented by this closeout:** K7 (nodes), P1, P2, P3, A1, A2, A3, A4/A5/A6 pins,
K8's rebuild/freshness proof at the AC2 layer.

**Found and recorded:** the dead core relation vocabulary, the `SpecifiedBy` name
collision, and a **correction to an A3-S15 follow-up** — the AC2 overlay *does* have a
pinned rebuild (`architecture_graph::rebuild`, REQ-AC2-006); A3-S15's probe built a
fresh overlay instead of using it, so that probe tested a weaker property than
intended.

## 7. Follow-ups (open, not MUSTs for A3)

| id | P | What |
|---|---|---|
| `FU-A3-S15-1` | **P1 if required by generic Verify/DebVerify, else P2** | `evidence → observes → software_relation` — **A4 provenance/evidence substrate candidate.** Benefits WHY, generic Verify, generic DebVerify, CogniCode and Chronos, so it must not be implemented as a WHY-specific patch. |
| `FU-A3-S15-2` | P3 | **Corrected.** `SemanticGraphProjection::rebuild_from_canonical` has no callers, but the AC2 overlay has its own pinned `rebuild()`. The original wording overstated the gap. |
| `FU-A3-S15-3` | P3 | `VerifiedBy` targets a spec node rather than evidence (no evidence node kind). |
| `FU-A3-S15-4` | P3 | Pin generated explanation text against its condition, not its existence. |
| `FU-A3-CO-1` | P2 | Canonical relation encoding for `KnowledgePayload::Relation`, then cross-tree edges. |
| `FU-A3-CO-2` | P2 | Resolve the dead core `ContractedBy`/`SpecifiedBy` vocabulary (emit or remove). |
| `FU-A3-CO-3` | P3 | Disambiguate the two `SpecifiedBy` relations by name. |
| `ASC-MA-1` | P3 | Root help still calls `architecture` "emit the architecture-conformance receipt" although six read surfaces and `findings` share the namespace. |
