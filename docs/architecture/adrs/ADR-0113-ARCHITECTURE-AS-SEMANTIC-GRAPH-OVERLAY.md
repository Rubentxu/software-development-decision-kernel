---
id: ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY
status: proposed
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: null
accepted_by_cycle: null
implementation_evidence: []
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
