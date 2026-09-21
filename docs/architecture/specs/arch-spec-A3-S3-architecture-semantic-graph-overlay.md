---
id: arch-spec-A3-S3-architecture-semantic-graph-overlay
status: proposed
proposed_at: 2026-09-15
source: ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY, arch-spec-033-architecture-semantic-graph-overlay
cycle: p-63676b11dc0ef88f/a3-3-architecture-graph-overlay
supersedes_history: false
---

# arch-spec-A3-S3 — Architecture SemanticGraph Overlay (cycle-bounded)

This is the **cycle-bounded spec** for A3-S3 / AC2. The canonical spec lives in
`docs/architecture/specs/arch-spec-033-architecture-semantic-graph-overlay.md`
(status: proposed). This file pins the concrete scope, REQs, acceptance,
negative fixtures and anti-encroachment probes for the cycle
`p-63676b11dc0ef88f/a3-3-architecture-graph-overlay`.

## Intent

Project the existing `ArchitecturalContract` (OBJECT) and `ArchitectureClaim`
(PROJECTION) substrate — together with `SoftwareUnit`, `DecisionRef`, `SpecRef`,
`TestRef`, `UatRef` and the new typed overlay relations (`OWNS`, `DEPENDS_ON`,
`WRITES`, `READS`, `EMITS`, `CONSUMES`, `PROJECTS`, `DERIVES_FROM`, `IMPLEMENTS`,
`DECIDED_BY`, `VERIFIED_BY`, `CONTRADICTS_BY`, `SUPERSEDES_BY`) — into the
**one canonical** `SemanticGraphProjection`, so:

1. There is no second architecture-graph persistence authority
   (REQ-AC2-001 / AC-033-001).
2. Delete-and-rebuild yields equivalent queries and digest
   (REQ-AC2-007 / AC-033-006).
3. The architecture traceable chain `finding → claim → contract → decision/spec →
   evidence → software unit` is traversable (REQ-AC2-008 / AC-033-007).

## Source of truth

- ADR-0113 (proposed) — `docs/architecture/adrs/ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY.md`
- arch-spec-033 (proposed) — `docs/architecture/specs/arch-spec-033-architecture-semantic-graph-overlay.md`
- A3-S2 substrate — `docs/architecture/specs/arch-spec-A3-S2-architectural-contracts.md`
  (`ArchitecturalContract`, `ArchitectureClaim`, `BasisHash`, `EvidenceRef`)
- arch-spec-005 — `docs/architecture/specs/arch-spec-005-semantic-graph-and-why.md`
  (`SemanticGraphProjection`, `SemanticNode`, `SemanticRelation`)
- Architecture Graph Model — `docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/02-ARCHITECTURE-GRAPH-MODEL.md`
- Contracts and Claims — `docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/03-ARCHITECTURAL-CONTRACTS-AND-CLAIMS.md`

## State classes (per ADR-0095)

- `ArchitectureGraphOverlay` — **PROJECTION** (rebuildable from canonical
  facts + contract substrate + claim substrate).
- `SoftwareUnitRef`, `DecisionRef`, `SpecRef`, `TestRef`, `UatRef`,
  `CompatibilityPathRef` — **typed newtypes** (no persistence, pure references).
- `ArchitectureOverlayRelationKind` — **closed enum** of overlay relations
  (14 variants). Domain tags are namespaced (`ac2.owns`, `ac2.depends_on`, …)
  so the relation kind can be parsed through the existing `RelationKind::Extension`
  path without growing `CoreRelationKind`.
- `ArchitectureOverlayNodeKind` — **closed enum** of overlay node kinds
  (8 variants: SoftwareUnit, BoundedContext, DecisionRef, SpecRef, TestRef,
  UatRef, CompatibilityPath, ArchitectureClaim). Domain tags are namespaced
  (`ac2.software_unit`, `ac2.bounded_context`, …).

## Type contract

```text
ArchitectureGraphOverlay        // PROJECTION, lives in sddk-engine
├── add_contract(contract: &ArchitecturalContract)
├── add_claim(claim: &ArchitectureClaim)
├── add_unit(unit: &SoftwareUnit)
├── add_relation(rel: ArchitectureOverlayRelation)
├── projection(): &SemanticGraphProjection  // reuses canonical graph
├── rebuild(contracts, claims, units) -> () // delete-and-rebuild
├── find_units_contracted_by(contract_id) -> Vec<NodeId>
├── find_contracts_for_unit(unit_id)        -> Vec<NodeId>
├── traverse_finding_to_software(claim_id) -> Vec<NodeId>
├── traverse_decision_to_software(decision_ref) -> Vec<NodeId>
├── digest(): Digest                          // == projection.digest() bytes
└── query(Q): Vec<NodeId>                     // bounded query surface

ArchitectureOverlayNodeKind (closed enum, namespaced domain_tag)
├── SoftwareUnit
├── BoundedContext
├── DecisionRef
├── SpecRef
├── TestRef
├── UatRef
├── CompatibilityPath
├── ArchitectureClaim
└── EvidenceRef                  // A4-S15R: projection of evidence_ref::EvidenceRef

ArchitectureOverlayRelationKind (closed enum, namespaced domain_tag)
├── Owns
├── DependsOn
├── Writes
├── Reads
├── Emits
├── Consumes
├── Projects
├── DerivesFrom
├── Implements
├── DecidedBy
├── VerifiedBy
├── ContradictsBy
├── SupersedesBy
└── ArchitectureClaimedBy   // ArchitectureClaim → Contract (1:N from claim)

SoftwareUnit (PROJECTION input, lives in sddk-engine)
├── id: SoftwareUnitRef
├── locator: String                  // e.g. "crates/sddk-engine/src/semantic_graph.rs"
├── kind: UnitKind                   // Module | Crate | Function | Endpoint | Service | Schema | Table
└── module_path: Vec<String>         // bounded context chain
```

## Requirements (18 REQs)

### Single authority

- **REQ-A3S2-001-equivalent (REQ-AC2-001)** — there is no second
  `ArchitectureGraph` persistence authority. The overlay **delegates** to the
  canonical `SemanticGraphProjection`; it never owns bytes.
- **REQ-AC2-002** — `ArchitectureGraphOverlay::projection()` returns a
  reference to the canonical graph; mutating the overlay mutates that graph
  in-place (no copy-on-write, no shadow store).

### Typed overlay kinds (no free strings)

- **REQ-AC2-003** — overlay node kinds route through the existing
  `NodeKind::Extension(NamespacedKind)` path with domain prefix `ac2.*`.
  No new `CoreNodeKind` variants in this cycle.
- **REQ-AC2-004** — overlay relation kinds route through the existing
  `RelationKind::Extension(NamespacedKind)` path with domain prefix `ac2.*`.
  No new `CoreRelationKind` variants in this cycle.
- **REQ-AC2-005** — domain tags are deterministic, lowercase, snake_case,
  ASCII-only. Closed set is pinned by tests (`all_overlay_node_tags_unique`,
  `all_overlay_relation_tags_unique`).

### Rebuild equivalence

- **REQ-AC2-006** — `rebuild(contracts, claims, units)` clears the overlay and
  re-projects from inputs. Two rebuilds over the same inputs yield identical
  `digest()` bytes (REQ-AC2-007 / AC-033-006).
- **REQ-AC2-007** — `digest()` equals
  `projection().canonical_bytes()` (the overlay does not invent a parallel
  digest surface).
- **REQ-AC2-008** — `rebuild` is O(N+M) where N is contract count + claim
  count and M is unit count; no quadratic blow-up, no hidden cross-references.

### Query surface (bounded, typed)

- **REQ-AC2-009** — `find_units_contracted_by(contract_id)` returns the
  set of `SoftwareUnit` node IDs whose `ArchitectureClaimedBy` relation
  points at `contract_id`. Empty set when no claim exists.
- **REQ-AC2-010** — `find_contracts_for_unit(unit_id)` returns the
  set of `ArchitecturalContract` node IDs connected to `unit_id` via the
  inverse `ArchitectureClaimedBy` projection.
- **REQ-AC2-011** — `traverse_finding_to_software(claim_id)` traverses
  `ArchitectureClaim → Contract → DecisionRef → SpecRef → SoftwareUnit` and
  returns the visited `SoftwareUnit` IDs in document order.
- **REQ-AC2-012** — `traverse_decision_to_software(decision_ref)` is the
  forward half of REQ-AC2-011 and returns `SoftwareUnit` IDs reachable from
  the decision via the overlay relation set.
- **REQ-AC2-013** — every query helper has a bounded input (one NodeId or
  one typed ref) and a bounded output (Vec<NodeId>). No global scans
  exposed to callers.

### Traceability chain (AC-033-007)

- **REQ-AC2-014** — the overlay emits, for each accepted contract, an
  `ArchitectureClaim → Contract` (ArchitectureClaimedBy) relation whenever
  a matching claim exists, plus a `Contract → DecisionRef` (DecidedBy)
  relation, a `Contract → SpecRef` (SpecifiedBy) relation (always emitted;
  declared intent), and one `Contract → EvidenceRef` (VerifiedBy)
  relation per typed-unique `EvidenceRef` in the contract's evidence set
  (A4-S15R). VerifiedBy is **not** emitted for an empty evidence set.
  `SpecifiedBy` and `VerifiedBy` are **separate provenance axes**:
  declared intent vs. verification evidence.
- **REQ-AC2-015** — contradiction claims (`outcome = Contradicted`) emit a
  `Claim → Contract` `ContradictsBy` relation instead of `ArchitectureClaimedBy`.
  This keeps assessment results distinguishable from observed facts
  (REQ-AC2-016 / AC-033-004).
- **REQ-AC2-016** — stale claims (`outcome = Stale`) are excluded from the
  projection (no relation emitted) — they must not pollute the canonical
  graph with stale assertions.
- **REQ-AC2-017** — every non-declared observation carries basis/provenance/
  freshness metadata. The overlay reuses `EvidenceRef` from A3-S2 (and the
  universal `EvidenceRef` from A4-0 / `evidence_ref::EvidenceRef`); each
  `VerifiedBy` edge in the projection points at a typed `EvidenceRef`
  projection node (A4-S15R). The node's identity derives from
  `EvidenceRef::ordering_key()` (sha256 over kind | locator | cas), so
  typed-equal refs produce the same node and dedup is by content.
- **REQ-AC2-018** — `query` accepts a `Query` ADT with three closed variants
  (`ContractsForUnit`, `UnitsContractedBy`, `TraverseDecisionToSoftware`).
  Unknown query variants fail closed (return empty, do not panic).

## Anti-encroachment (REQ-AC2-019..022)

- **REQ-AC2-019** — `architecture_graph` module imports nothing from
  `sddk-pack-uat`, `sddk-jcode`, `sddk-gateway`, `provider_*`, `host_*`
  packages.
- **REQ-AC2-020** — overlay types do **not** carry capability grants, do
  not write to canonical storage, and do not invoke authority admission
  (the overlay is a pure projection builder).
- **REQ-AC2-021** — overlay never parses Markdown / YAML / TOML as runtime
  authority. Inputs come from typed Rust values only.
- **REQ-AC2-022** — no second digest / canonical-bytes surface. The overlay
  delegates to `SemanticGraphProjection::canonical_bytes` and asserts byte
  equality in tests.

## Acceptance (per arch-spec-033)

- AC-033-001 → REQ-AC2-001, REQ-AC2-002
- AC-033-002 → REQ-AC2-003
- AC-033-003 → REQ-AC2-004, REQ-AC2-005, REQ-AC2-014, REQ-AC2-015
- AC-033-004 → REQ-AC2-015, REQ-AC2-016
- AC-033-005 → REQ-AC2-017
- AC-033-006 → REQ-AC2-006, REQ-AC2-007, REQ-AC2-008
- AC-033-007 → REQ-AC2-011, REQ-AC2-012, REQ-AC2-014

## UAT

AC-UAT-002, 003, 004, 016 (per arch-spec-033 acceptance line).

## Test plan (16 tests planned)

1. **acceptance/rebuild_equivalence** — rebuild twice, identical digest.
2. **acceptance/rebuild_clear_then_reproject** — `rebuild` after adding nodes
   yields only the re-projected set (no carryover).
3. **acceptance/digest_equals_projection_bytes** — overlay.digest() ==
   projection().canonical_bytes() byte-for-byte.
4. **acceptance/find_units_contracted_by_returns_unit_set** — query with a
   single contract returns the unit IDs it constrains.
5. **acceptance/find_units_contracted_by_empty_when_no_claim** — query
   without matching claim returns empty (not panic).
6. **acceptance/find_contracts_for_unit_is_inverse** — for a unit with N
   contracts, the inverse query returns all N.
7. **acceptance/traverse_finding_to_software_visits_chain** — visits
   ArchitectureClaim → Contract → DecisionRef → SpecRef → SoftwareUnit
   in document order.
8. **acceptance/traverse_decision_to_software_forward_half** — returns
   unit IDs reachable from a decision through the overlay relations.
9. **acceptance/contradicted_claim_emits_contradicts_by** — claim with
   outcome `Contradicted` emits `ContradictsBy`, not `ArchitectureClaimedBy`.
10. **acceptance/stale_claim_excluded_from_projection** — claim with
    outcome `Stale` does not emit any relation.
11. **acceptance/relation_attaches_evidence_refs** — every emitted overlay
    relation carries `evidence` with at least one EvidenceRef.
12. **negative/unknown_query_variant_returns_empty** — query variant not in
    the closed set returns empty Vec, does not panic.
13. **anti-encroachment/no_a4_or_provider_imports** — same pattern as S2.
14. **anti-encroachment/no_capability_or_authority_side_effects** — overlay
    contains no `CapabilityId`, no `grants`, no `admission` calls.
15. **anti-encroachment/no_markdown_parsing** — no `pulldown_cmark`,
    `serde_yaml`, `toml::from_str` for contract ingestion.
16. **anti-encroachment/no_second_digest_surface** — assert
    `overlay.digest().as_bytes() == projection().canonical_bytes()`.

## Out of scope (deferred)

- No new `CoreNodeKind`/`CoreRelationKind` variants in this cycle
  (overlay uses `Extension(NamespacedKind)` exclusively).
- No integration with `active_graph` (different projection; separate cycle).
- No provider/CogniCode/Chronos ingest (A6/A7).
- No WHY query surface (covered by `cockpit_views` + future work).
- No mutation probes (AC6, A4+).
- No paradigm lens evaluation (AC7, A4+).

## Carry-over debt awareness

- `C4_LEGACY_ALLOWLIST_M1` line numbers will shift by +1 per `pub mod X;`
  insertion; this cycle adds `pub mod architecture_graph;` in `lib.rs`.
  Maintain the allowlist in the same commit (anti-brittleness, not new debt).
- The `surface.briefness.*` checks in `dev doctor` are pre-existing P3
  misses; not a regression, no action in this cycle.
