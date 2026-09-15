---
id: ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY
status: accepted
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-3-architecture-graph-overlay
implementation_evidence:
  - "crates/sddk-engine/src/architecture_graph/mod.rs (ArchitectureGraphOverlay entry; state-class doc)"
  - "crates/sddk-engine/src/architecture_graph/types.rs (SoftwareUnit, UnitKind, DecisionRef, SpecRef, TestRef, UatRef, CompatibilityPathRef, ArchitectureOverlayNodeKind, ArchitectureOverlayRelationKind)"
  - "crates/sddk-engine/src/architecture_graph/overlay.rs (ArchitectureGraphOverlay::add_unit/add_claim/add_relation/add_contract_metadata/find_*_contracted_by/find_contracts_for_unit/attach_claim_to_unit/traverse_finding_to_software/traverse_decision_to_software; Query ADT with 3 closed variants)"
  - "crates/sddk-engine/src/architecture_graph/rebuild.rs (sorted rebuild for determinism; REQ-AC2-006)"
  - "crates/sddk-engine/src/architecture_graph/tests.rs (16 acceptance + 4 anti-encroachment + bonus type pins)"
  - "docs/architecture/specs/arch-spec-A3-S3-architecture-semantic-graph-overlay.md (cycle-bounded spec, 22 REQs)"
superseded_by: []
related_adrs:
  - "ADR-0098-ONE-SEMANTIC-GRAPH"
  - "ADR-0100-UNIVERSAL-EVIDENCE"
stale_after: 2027-03-14
---

# ADR-0113 — Architecture as a SemanticGraph Overlay

## Context

Authority maps, ownership maps, dependency maps and architecture workbooks are graph-shaped. Creating a dedicated architecture database would duplicate the existing SemanticGraphProjection and create a second projection authority.

## Decision

Represent architecture knowledge as typed nodes/relations and assessment objects inside the one rebuildable `SemanticGraphProjection`.

Architecture-specific vocabulary is layered; the shared graph vocabulary stays intentionally small. Raw observed relations (DEPENDS_ON, WRITES, EMITS, OWNS, IMPLEMENTS, DERIVES_FROM) are distinguished from assessment/status objects.

Architecture workbooks and behavior maps are derived projections over this graph and canonical evidence/decisions.

## Consequences

- WHY/WHY-NOT can traverse decision→contract→evidence→software relations;
- static/runtime providers can enrich the same graph without owning truth;
- delete/rebuild remains a mandatory invariant;
- no separate graph authority may be introduced for architecture convenience.

## Replaces

This decision replaces proposals for an independent ArchitectureGraph persistence subsystem.
