---
id: arch-spec-A3-S4-paradigm-lens-profile
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-4-paradigm-lens-profile
source: arch-spec-035-paradigm-lens-system + ADR-0114-PARADIGMS-AS-ALIGNMENT-LENSES
based_on: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/04-PARADIGM-LENSES.md
---

# arch-spec-A3-S4 — ParadigmProfile + Lens Selection as SemanticGraph Overlay

## Intent

AC3 of the Architecture Conformance track (per
`docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/09-ROADMAP.md`):

> Implement `ParadigmProfile`/lens selection and graph relations without
> judging code yet. Exit: profiles can differ by bounded
> context/software unit and survive rebuild.

This is the **cycle-bounded spec** that supersedes nothing historical but
inherits `arch-spec-035-paradigm-lens-system.md` (which remains
`status: proposed` upstream — the canonical narrative).

## Resolved open questions

(Q1, blocking) Anchor coverage: **Option C** — `paradigm_profile` adds two
new namespaced kinds (`ac3_node_project_intent`, `ac3_node_bounded_context`)
and re-uses `ac2_node_software_unit` for software-unit anchors. AC2
substrate is untouched.

(Q2, blocking) `ProjectIntent` lives as a graph node (Path 2). One
`ProjectIntent` per cycle; acts as the paradigm-profile root.

(Q3, blocking) Assessments exist as `UNKNOWN` data only (`basis == DECLARED`,
`status` is part of the closed vocabulary but **defaults to UNKNOWN for every
profile in this cycle**). Verdict logic belongs to AC7.

## Scope (must)

- Declare `ParadigmProfileKind` as a closed enum of 11 variants.
- Declare `ParadigmLensKind` as a closed enum of 11 variants.
- Declare `LensStatus` as a closed enum of 7 variants.
- Declare `EvidenceBasis` as a closed enum of 5 variants.
- Project paradigm profiles as nodes in the same `SemanticGraphProjection`
  used by AC2 (no second projection authority).
- Three anchor shapes: `ProjectIntent`, `BoundedContext`, `SoftwareUnit`.
- One overlay relation `ac3_rel_uses_paradigm` (anchor → paradigm) and
  `ac3_rel_evaluated_by` (anchor → lens).
- `ParadigmProfileOverlay` (mirror of `ArchitectureGraphOverlay`) with
  `rebuild()`, deterministic digest, query ADT.
- `ParadigmQuery` ADT (closed set, 3 variants):
  - `ProfilesForAnchor(ParadigmAnchorRef)`
  - `AnchorsForParadigm(ParadigmProfileKind)`
  - `AssessmentsForAnchor(ParadigmAnchorRef)`
- Each `Assessment` has `status` defaulting to `UNKNOWN` and `basis` defaulting
  to `DECLARED` until AC7 plugs in observation.

## Scope (must NOT)

- NO lens evaluation; assessments carry observed evidence as `None`.
- NO write to `EffectiveInstructions`; no capability grant.
- NO Authority bypass; lens evaluation cannot gate AuthorityEngine.
- NO scoring of code; no "passes OO"/"fails FP" verdicts.
- NO second projection authority; everything goes through the existing
  `SemanticGraphProjection`.

## Requirements (REQ-AC3-NNN format)

### Profile & lens vocabulary

- **REQ-AC3-001** — `ParadigmProfileKind` is a closed enum with exactly 11
  variants matching `04-PARADIGM-LENSES.md` §ParadigmProfile:
  `ObjectOriented | Functional | FunctionalPure | DataOriented | EventDriven |
  Reactive | Hexagonal | DDD | Pipeline | ActorLike | Custom`.
  Pin test: `ParadigmProfileKind::ALL.len() == 11` (compile-time).

- **REQ-AC3-002** — `ParadigmLensKind` mirrors `ParadigmProfileKind` (same
  11 variants). The lens kind is the *evaluating* side of the same
  paradigm namespace.
  Pin test: `ParadigmLensKind::ALL.len() == 11`.

- **REQ-AC3-003** — `LensStatus` is a closed enum with exactly 7 variants
  (`ALIGNED | TENSION | MISALIGNED | ACCEPTED | REVIEW_DUE | UNKNOWN |
  NOT_APPLICABLE`).
  Pin test: `LensStatus::ALL.len() == 7`.

- **REQ-AC3-004** — `EvidenceBasis` is a closed enum with exactly 5 variants
  (`DECLARED | OBSERVED | VERIFIED | MUTATION_VERIFIED | RUNTIME_CORROBORATED`)
  taken from `11-FITNESS-RECEIPTS.md` §Proof levels.
  Pin test: `EvidenceBasis::ALL.len() == 5`.

### Anchor coverage

- **REQ-AC3-005** — Three anchor kinds: `ProjectIntent`, `BoundedContext`,
  `SoftwareUnit`. SoftwareUnit is shared with AC2 substrate
  (`ac2_node_software_unit`); the other two are local to AC3
  (`ac3_node_project_intent`, `ac3_node_bounded_context`).
  Pin test: `ParadigmAnchorKind::ALL.len() == 3`.

- **REQ-AC3-006** — Anchor identity uses a typed newtype:
  `ProjectIntentRef(String)`, `BoundedContextRef(String)`,
  `SoftwareUnitRef(String)` (re-used from AC2). All three carry `Cow<'static, str>`
  semantics preferred.

- **REQ-AC3-007** — A `ParadigmAnchorRef` enum dispatches per anchor kind;
  `Display`/`canonical_payload` are implemented for hashing. Each
  newtype's `canonical_payload` is `pub(crate)`, mirroring the AC2 convention.

### Overlay projection

- **REQ-AC3-008** — `ParadigmProfileOverlay` wraps the existing
  `InMemorySemanticGraph` (no second projection authority); exposes
  `add_anchor()`, `add_profile()`, `add_assessment()`, `add_lens()`,
  `find_profiles_for_anchor()`, `find_anchors_for_paradigm()`,
  `find_assessments_for_anchor()`.
  Pin test: `paradigm_profile.digest() == projection().canonical_bytes()`.

- **REQ-AC3-009** — Overlay relation kinds (3 namespaced closed kinds):
  `ac3_rel_uses_paradigm`, `ac3_rel_evaluated_by`, `ac3_rel_lens_target`.
  Pin test: `ParadigmOverlayRelationKind::ALL.len() == 3`.

- **REQ-AC3-010** — A profile attaches to one anchor via exactly one
  `ac3_rel_uses_paradigm` edge. Multiple `uses_paradigm` edges per anchor
  are allowed (poly-paradigm).
  Pin test: `acceptance_anchor_supports_multiple_paradigms`.

- **REQ-AC3-011** — A lens selects one anchor (or one paradigm profile) via
  `ac3_rel_lens_target`. AC3 implements only the data model; AC7 plugs
  observation/verdict.

- **REQ-AC3-012** — An `Assessment` references (anchor_ref, lens_kind)
  with `status` defaulting to `UNKNOWN` and `basis == DECLARED`.
  Pin test: `acceptance_assessment_default_is_unknown_decared`.

### Build/rebuild determinism

- **REQ-AC3-013** — `ParadigmProfileInputs` (mirror of AC2's `RebuildInputs`)
  plus `rebuild(overlay, &inputs)` produce identical canonical bytes
  across two invocations on identical inputs (REQ-AC3-013 ≡ REQ-AC2-006
  re-stated).
  Pin test: `acceptance_rebuild_equivalence`.

- **REQ-AC3-014** — Sorted iteration on every collection to guarantee
  determinism. The same `NamespacedKind` 2-segment domain-tag convention
  applies (`ac3_node_*`, `ac3_rel_*`).

- **REQ-AC3-015** — `rebuild` followed by `clear()` followed by `rebuild`
  must yield identical bytes (no carryover).
  Pin test: `acceptance_rebuild_clear_then_reproject`.

### Query ADT

- **REQ-AC3-016** — `ParadigmQuery` is a closed ADT with exactly 3
  variants enumerated above. `query()` dispatches and returns a
  deterministic, sorted result.
  Pin test: `bonus_query_advertises_three_closed_variants`.

- **REQ-AC3-017** — `query(ParadigmQuery::ProfilesForAnchor(anchor))`
  returns all paradigm profiles attached to that anchor (sorted by
  `ParadigmProfileKind::domain_tag()`).

- **REQ-AC3-018** — `query(ParadigmQuery::AnchorsForParadigm(kind))`
  returns all anchors that declare the given paradigm (sorted by anchor id).

- **REQ-AC3-019** — `query(ParadigmQuery::AssessmentsForAnchor(anchor))`
  returns all assessments for that anchor (sorted). All assessments for
  freshly-built data return `status == UNKNOWN` (REQ-AC3-012 default).

### Anti-encroachment pins (compile-time + source-grep)

- **REQ-AC3-020** — `paradigm_profile/` source does NOT `use` any of the
  forbidden neighbour crates from A3-S1's REQ-A3S1-040 anti-encroachment:
  `alignment`, `verify`, `debverify`, `authority_engine`,
  `completion_provider_router`, `agent_host`, `provider`, `host_sdk`.

- **REQ-AC3-021** — `paradigm_profile` does NOT import `EffectiveInstructions`,
  `AuthorityEngine`, or any capability-bearing type. Pin test:
  `anti_encroachment_no_authority_or_capability_side_effects`.

- **REQ-AC3-022** — `paradigm_profile.digest() == projection().canonical_bytes()` —
  the function `digest()` on the overlay must delegate to
  `projection().canonical_bytes()` and not maintain a parallel digest
  field. Pin test: `anti_encroachment_no_second_digest_surface`.

- **REQ-AC3-023** — `paradigm_profile` does NOT depend on `architecture_graph`
  import for **types** (it may share `InMemorySemanticGraph` through the
  substrate layer but the `ArchitectureGraphOverlay` is not imported —
  pin test: `anti_encroachment_no_architecture_graph_reexport`).
  Rationale: AC4 may want to traverse from a paradigm profile back to
  contracted units, and that traversal must go through the substrate, not
  through a direct cross-module import.

### Acceptance closure

- **REQ-AC3-024** — ADR-0114 is promoted from `proposed` to `accepted`
  with `implementation_evidence` listing the new module
  (`crates/sddk-engine/src/paradigm_profile/`). Acceptance mirrors to
  `~/.sddk-knowledge/sddk-framework/adrs/ADR-0114-*.md`.

- **REQ-AC3-025** — `arch-spec-035` upstream remains `status: proposed`
  (per cycle-bounded convention from A3-S3); the cycle-bounded spec lives
  at `docs/architecture/specs/arch-spec-A3-S4-paradigm-lens-profile.md`.

## Acceptance tests (planned = ~16)

1. `acceptance_rebuild_equivalence` (REQ-AC3-013) ✓
2. `acceptance_rebuild_clear_then_reproject` (REQ-AC3-015) ✓
3. `acceptance_digest_equals_projection_bytes` (REQ-AC3-008) ✓
4. `acceptance_anchor_supports_multiple_paradigms` (REQ-AC3-010) ✓
5. `acceptance_assessment_default_is_unknown_decared` (REQ-AC3-012) ✓
6. `acceptance_paradigm_profile_kind_unique` (REQ-AC3-001) ✓
7. `acceptance_paradigm_lens_kind_unique` (REQ-AC3-002) ✓
8. `acceptance_lens_status_unique` (REQ-AC3-003) ✓
9. `acceptance_evidence_basis_unique` (REQ-AC3-004) ✓
10. `acceptance_find_profiles_for_anchor_sorted` (REQ-AC3-017) ✓
11. `acceptance_find_anchors_for_paradigm_sorted` (REQ-AC3-018) ✓
12. `acceptance_find_assessments_for_anchor_returns_unknown` (REQ-AC3-019) ✓
13. `acceptance_paradigm_overlay_relation_kind_unique` (REQ-AC3-009) ✓
14. `acceptance_relation_attaches_evidence_refs` (REQ-AC3-008/11) ✓
15. `anti_encroachment_no_authority_or_capability_side_effects` (REQ-AC3-021) ✓
16. `anti_encroachment_no_second_digest_surface` (REQ-AC3-022) ✓
17. `anti_encroachment_no_architecture_graph_reexport` (REQ-AC3-023) ✓
18. `anti_encroachment_no_a4_or_provider_imports` (REQ-AC3-020) ✓
19. `bonus_query_advertises_three_closed_variants` (REQ-AC3-016) ✓
20. `bonus_paradigm_profile_kinds_all_eleven_have_unique_tags` (REQ-AC3-001) ✓
21. `bonus_paradigm_lens_kinds_all_eleven_have_unique_tags` (REQ-AC3-002) ✓
22. `bonus_lens_status_all_seven_have_unique_tags` (REQ-AC3-003) ✓

## Companion changes

- `crates/sddk-engine/src/lib.rs` — add `pub mod paradigm_profile;`
  between `authority` and `knowledge` (alphabetical).
  This will tick INC-A3-S1-C4-LINE-SHIFT to its **4th instance**.
- ADR-0114 — promote from `proposed` to `accepted` in a follow-up commit.

## Out-of-scope (explicit non-goals)

- Lens evaluation (AC7)
- ArchitectureConformanceVector.paradigm_alignment final value (AC4/AC5)
- Mutation probes (AC6)
- Provider/host SDK adapters (ADR-0114 §initial lenses only)
- CLI surface for `sddk architecture paradigms` (deferred to CLI UX cycle)

## Carry-over debt

- INC-A3-S1-C4-LINE-SHIFT will tick to 4th instance; follow-up AST-based
  allowlist refactor remains open.

## References

- ADR-0114 (cycle's target for promotion)
- docs/.../04-PARADIGM-LENSES.md (canonical narrative)
- docs/.../05-ADT-FP-OO-DSL-MODELING.md (modelling guidance)
- docs/.../11-FITNESS-RECEIPTS.md (convergence point)
- arch-spec-A3-S3 (cycle-bounded spec recipe to mirror)
- arch-spec-035 (proposed upstream spec, do not modify in this cycle)
