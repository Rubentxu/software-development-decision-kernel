# A3-S2-AC1-RECEIPT

## Identity

- **Cycle:** `p-63676b11dc0ef88f/a3-2-architectural-contract`
- **Path:** A-min
- **Base commit:** `32047cd` (post v1.169.22 release)
- **Final commit:** `<populated at archive time>`
- **SDDK workspace version:** `1.169.22` (release bump will be 1.169.23)
- **Date:** 2026-09-15
- **Author:** orchestrator (direct implementation, no swarm spawn)
- **Spec:** `docs/architecture/specs/arch-spec-A3-S2-architectural-contracts.md`
- **Source of truth:** ADR-0112 (proposed) + arch-spec-032 (proposed)
- **Exploration report:** `.sddk/cycles/p-63676b11dc0ef88f-a3-2-architectural-contract/exploration-report.md`

## Verdict

`PASS` — A3-S2 delivers AC1: typed `ArchitecturalContract` + `ArchitectureClaim`
satisfy REQ-A3S2-001..022 with no S3+ leakage, no A4 imports, no provider SDK
imports, and no `ContextCapsule` modification. Module is split across 7 files
(1901 → 1367 product LoC, 734 LoC tests). `CoreNodeKind` baseline moves
18 → 19 (+ArchitecturalContract); `CoreRelationKind` baseline moves 14 → 16
(+ContractedBy, +SpecifiedBy); both deltas are pinned by
`s2_post_ac1_corenodekind_baseline` in `knowledge.rs` and
`core_relation_kinds_have_16_entries_after_ac1_relations` in `semantic_kind.rs`.
The historical S1 anchor `s1_does_not_introduce_new_corenodekind_variants`
is preserved as `#[ignore]` (fails by design after A3-S2) — its presence is
enforced by `s1_anti_encroachment_anchor_is_preserved_in_knowledge`.

## Scope delivered

### Files added

| Path | LoC | Purpose |
|---|---|---|
| `crates/sddk-engine/src/architectural_contract/mod.rs` | 60 | module entry, state-class doc, re-exports |
| `crates/sddk-engine/src/architectural_contract/types.rs` | 321 | `ContractId`, `Revision`, `ComponentRef`, `EntityRef`, `DecisionRef`, `SpecRef`, `ContractKindRef` |
| `crates/sddk-engine/src/architectural_contract/error.rs` | 82 | `ContractError` enum |
| `crates/sddk-engine/src/architectural_contract/payload.rs` | 215 | `ContractKind`, `BoundaryKind`, `ContractPayload`, `ContractExtensionValue` |
| `crates/sddk-engine/src/architectural_contract/hashing.rs` | 109 | canonical basis-hash derivation |
| `crates/sddk-engine/src/architectural_contract/contract.rs` | 283 | `ArchitecturalContract` OBJECT + typed declare_* constructors |
| `crates/sddk-engine/src/architectural_contract/claim.rs` | 297 | `EvidenceRef`, `EvaluatorRef`, `ClaimOutcome`, `ArchitectureClaim` (PROJECTION), `ContractEvaluation` (EPHEMERAL) |
| `crates/sddk-engine/src/architectural_contract/tests.rs` | 734 | 23 tests inline (11 acceptance + 4 negative + 4 anti-encroachment + 4 bonus) |

**Product code:** 1367 LoC across 7 files (largest 321 LoC).
**Test code:** 734 LoC in `tests.rs`.

### Files modified

| Path | Change |
|---|---|
| `crates/sddk-engine/src/lib.rs` | `+pub mod architectural_contract;` (alphabetical position) |
| `crates/sddk-engine/src/semantic_kind.rs` | +1 `CoreNodeKind::ArchitecturalContract` (ALL 18→19), +2 `CoreRelationKind::{ContractedBy, SpecifiedBy}` (ALL 14→16), updated domain_tag() matches, renamed test (14→16 entries), added `_had_14_entries_before_ac1` historical anchor test |
| `crates/sddk-engine/src/knowledge.rs` | exposed `BasisHash::from_digest` as `pub(crate)` (needed by sibling module), renamed `s1_does_not_introduce_new_corenodekind_variants` with `#[ignore = "historical anchor"]`, added `s2_post_ac1_corenodekind_baseline` companion test |
| `crates/sddk-engine/src/architectural_contract/tests.rs` | includes new module-level state-class audit covering all 6 production submodules |

### Files explicitly NOT touched

- `crates/sddk-engine/src/semantic_graph.rs`, `semantic_node.rs` — overlay integration deferred to A3-S3 (AC2)
- `crates/sddk-engine/src/context_capsule.rs` — `advisory_context` deferred to A3-S5
- `crates/sddk-engine/src/decision_memory.rs` — wiring deferred to S3+
- `crates/sddk-engine/src/authority_engine.rs`, `crates/sddk-engine/src/paradigm_lens.rs` (if exists) — A4 / AC7 deferred to A4 / AC7
- Any provider/host SDK module — anti-encroachment enforced
- Any alignment/verify/debverify module — anti-encroachment enforced

## Acceptance tests (REQ-A3S2-NNN)

| REQ | Test name | Status |
|---|---|---|
| REQ-A3S2-002 | `contract_identity_includes_id_kind_payload_refs_revision` | PASS |
| REQ-A3S2-002 | `basis_hash_is_deterministic_for_same_inputs` | PASS |
| REQ-A3S2-002 | `basis_hash_differs_on_payload_change` | PASS |
| REQ-A3S2-002 | `basis_hash_differs_on_decided_by_change` | PASS |
| REQ-A3S2-005 | `kind_payload_coupling_is_type_safe` | PASS |
| REQ-A3S2-006/007 | `extension_payload_uses_namespaced_kind_and_btreemap` | PASS |
| REQ-A3S2-016 | `extension_payload_rejects_provider_namespace` | PASS |
| REQ-A3S2-012 | `claim_outcome_is_unknown_when_evidence_is_empty` | PASS |
| REQ-A3S2-013 | `claim_outcome_is_stale_when_now_precedes_declared_at` | PASS |
| REQ-A3S2-014 | `claim_outcome_is_stale_when_deprecated_after_elapsed` | PASS |
| REQ-A3S2-014 | `bounded_compatibility_with_replacement_is_verified_past_window` | PASS |
| REQ-A3S2-004 | `serde_roundtrip_preserves_contract_identity_and_basis_hash` | PASS |

**11 / 11 acceptance tests PASS.**

## Negative fixtures

| Fixture | REQ | Status |
|---|---|---|
| `cannot_build_contract_with_empty_component_ref` | REQ-A3S2-010 (typed construction path) | PASS |
| `cannot_build_contract_with_empty_revision` | REQ-A3S2-010 (typed construction path) | PASS |
| `cannot_claim_verified_with_empty_evidence` | REQ-A3S2-012 | PASS |
| `extension_payload_rejects_non_namespaced_kind` | REQ-A3S2-006 | PASS |

**4 / 4 negative fixtures PASS.**

## Anti-encroachment probes (REQ-A3S2-021..022)

| Probe | REQ | Mechanism | Status |
|---|---|---|---|
| `s2_module_has_no_a4_or_provider_imports` | REQ-A3S2-021 | include_str! + grep on all 6 submodules | PASS |
| `s2_does_not_grant_capabilities` | REQ-A3S2-015 | grep for `grants`, `CapabilityId`, `granted_by` | PASS |
| `s2_markdown_is_not_runtime_authority` | REQ-A3S2-022 | grep for `parse_markdown`, `from_str_md`, etc. | PASS |
| `s2_post_ac1_corenodekind_baseline` | REQ-A3S2-017..020 | counts 19/16 pinned via include_str! | PASS |

**4 / 4 anti-encroachment probes PASS.**

### State-class discipline

- `architectural_contract_module_state_class_annotations_complete` PASS:
  - All 12 production types are present in source (ContractId, Revision,
    ComponentRef, EntityRef, DecisionRef, SpecRef, ContractKindRef,
    ContractKind, BoundaryKind, ContractPayload, ContractExtensionValue,
    ArchitecturalContract).
  - All 4 claim types (EvidenceRef, EvaluatorRef, ClaimOutcome,
    ArchitectureClaim) are present in `claim.rs`.
  - `# State class:` discipline declared in every submodule doc-comment.

## Bonus fixtures (added beyond the 19-test plan)

- `bounded_compatibility_with_replacement_is_verified_past_window` —
  documents that replacement contracts keep the old contract active
  past its window.
- `extension_value_canonical_form_is_deterministic` — pins insertion-
  order independence for nested `Object`/`Array` extension values.
- `provider_boundary_contract_preserves_decision_ref` — confirms
  `ProviderBoundary` contracts round-trip Decision/Spec provenance.
- `architectural_contract_module_state_class_annotations_complete` —
  state-class discipline audit.

**4 bonus fixtures PASS (23 total in module).**

## Crosswalks (per spec §Crosswalks)

| # | Crosswalk | Outcome |
|---|---|---|
| 1 | ADR-0112 ↔ `ArchitecturalContract` fields | 1:1 mapping of decision concepts (id, kind, payload, refs, revision, declared_at) |
| 2 | arch-spec-032 AC-032-001..007 ↔ REQ-A3S2-001..022 | every AC-NNN has a corresponding REQ-A3S2-NNN with at least one test; no gaps |
| 3 | A3-S1 `BasisHash` ↔ A3-S2 `ArchitecturalContract::basis_hash` | same SHA-256 primitive (`pub(crate) from_digest`), distinct domain prefix (`sddk.architectural_contract.v1\n`) prevents cross-substrate collisions |
| 4 | CoreNodeKind baseline 18 → 19 | `ArchitecturalContract` variant added; pinned by `s2_post_ac1_corenodekind_baseline` |
| 5 | CoreRelationKind baseline 14 → 16 | `ContractedBy`, `SpecifiedBy` variants added; pinned by `core_relation_kinds_have_16_entries_after_ac1_relations` |

## Domain separation verification

`ArchitecturalContract::basis_hash()` derivation uses the prefix
`sddk.architectural_contract.v1\n` (REQ-A3S2-002 crosswalk #3), distinct
from `KnowledgeAssertion::basis_hash()` (`sddk.knowledge.assertion.v1\n`).
A `cargo test -p sddk-engine --lib knowledge:: architectural_contract::`
run (53 tests) shows both modules coexist with no domain collision.

## Risks and known constraints

- **Pause CHECK constraint** (pre-existing P3 operational debt): unrelated to S2.
- **Swarm stall** on long-context M2.7-highspeed startup (P3 operational
  debt, tracked from A3-S1 handoff). S2 was implemented orchestrator-direct
  as a result; no swarm spawn was attempted.
- **`storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`**
  flake (P3 pre-existing) was not encountered in this cycle's scoped test
  run (architectural_contract tests are pure, no SQLite).
- **Public API expansion**: `pub use architectural_contract::*` exposes 12
  types. Downstream consumers can already see them via `sddk_engine::architectural_contract::*`.
  No breaking change to existing public surface (knowledge, semantic_kind,
  lib all preserved).

## Verification (independent path)

Run from CWD `~/Proyectos/agentesIA/sddk-framework`:

```bash
cargo test -p sddk-engine --lib -- knowledge:: architectural_contract:: semantic_kind::
# 53 passed; 0 failed; 1 ignored (S1 historical anchor)

cargo fmt --check
# clean

cargo clippy -p sddk-engine --lib --tests -- -D warnings
# clean
```

Full workspace profile (`cargo test --workspace`) is reserved for the
release flow gate (per AGENTS.md §5).

## Out of scope (deferred, anti-encroachment confirmed)

- ✗ SemanticGraph overlay integration of ArchitecturalContract (A3-S3 / AC2)
- ✗ ParadigmProfile metadata + lens selection (A3-S4 / AC3)
- ✗ ContextCapsule.advisory_context + instruction-hash separation (A3-S5)
- ✗ Verify / DebVerify / ConformanceDelta semantics (A4, AC4, AC5)
- ✗ Paradigm lenses (AC7)
- ✗ Mutation probes (AC6)
- ✗ Markdown ingestion CLI (`sddk ingest contract.md`)
- ✗ Provider SDK types in payload (anti-encroached via `provider.<x>` reject)
- ✗ Capability grants (REQ-A3S2-015 forbids; ADR-0102 owns capability semantics)
- ✗ Promoting ADR-0112 / arch-spec-032 to `accepted` (separate cycle)

## Decisions taken

1. **Split to `mod.rs` + 6 submodules** instead of single-file: original
   single-file draft reached 1901 LoC, exceeding the A3-S1 rule-of-thumb
   budget of 1200. Split keeps the largest production submodule
   (`types.rs`) at 321 LoC and isolates the hashing module from
   public-facing types.
2. **`Cow<'static, str>` for canonical tags** instead of `&'static str`:
   necessary because `ContractKind::Extension` carries a runtime
   `ContractKindRef`, which cannot satisfy the `'static` bound. The six
   closed variants return `Cow::Borrowed` (zero allocation); only
   `Extension` allocates.
3. **`BasisHash::from_digest` exposed as `pub(crate)`** rather than
   building a public `from_hash` constructor: preserves the existing
   construction discipline (S1 substrate owns derivation) while allowing
   the sibling module to compute its basis hash.
4. **S1 anti-encroachment test marked `#[ignore]`** instead of removed or
   updated to 19/16: preserves the historical anchor (verified separately
   by `s1_anti_encroachment_anchor_is_preserved_in_knowledge` in
   `architectural_contract/tests.rs`) while not failing the test suite.
5. **BoundedCompatibility with `replaced_by` does NOT mark Stale past
   window**: documented as a bonus test
   (`bounded_compatibility_with_replacement_is_verified_past_window`) so
   the migration semantics are explicit.

## Cycle ID

`p-63676b11dc0ef88f/a3-2-architectural-contract`
