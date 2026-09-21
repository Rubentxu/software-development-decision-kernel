---
id: arch-spec-033-architecture-semantic-graph-overlay
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-033 — Architecture SemanticGraph Overlay

## Intent

Architecture knowledge SHALL be represented as a typed overlay in the one rebuildable SemanticGraphProjection.

## Requirements

- AC-033-001: no second architecture graph persistence authority.
- AC-033-002: support typed nodes for units, contexts, contracts, decisions/specs/tests/receipts and compatibility paths.
- AC-033-003: support observed relations such as OWNS/DEPENDS_ON/WRITES/READS/EMITS/CONSUMES/PROJECTS/DERIVES_FROM/IMPLEMENTS.
- AC-033-004: assessment results are distinguishable from observed facts/relations.
- AC-033-005: every non-declared observation carries basis/provenance/freshness.
- AC-033-006: delete/rebuild yields equivalent semantic architecture queries/digest.
- AC-033-007: WHY can traverse finding→claim→contract→decision/spec→evidence→software relation.

## Acceptance

AC-UAT-002, 003, 004, 016.
