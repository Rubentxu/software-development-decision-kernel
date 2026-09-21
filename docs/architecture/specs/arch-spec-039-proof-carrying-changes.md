---
id: arch-spec-039-proof-carrying-changes
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-039 — Proof-Carrying Changes

## Intent

A structured software change MAY carry reproducible conformance evidence for the architectural contracts it affects.

## Requirements

- AC-039-001: envelope includes change basis, affected contract ids, receipt/evidence refs, unknowns and compatibility delta.
- AC-039-002: envelope is evidence, not merge/release authority.
- AC-039-003: Governance may explicitly require a receipt for selected operations.
- AC-039-004: receipt identity binds exact source/contract/knowledge/provider bases.
- AC-039-005: invalid/missing receipt cannot be silently treated as verified.
- AC-039-006: Agentic structured work may return the envelope through typed Contribution/receipt contracts.

## Acceptance

AC-UAT-023, 041, 042.
