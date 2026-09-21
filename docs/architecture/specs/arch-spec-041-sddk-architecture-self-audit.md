---
id: arch-spec-041-sddk-architecture-self-audit
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-041 — SDDK Architecture Self-Audit

## Intent

Before the Architecture Conformance capability contributes to BASE_PRODUCTION_READY, SDDK SHALL dogfood it against its own accepted architecture.

## Requirements

- AC-041-001: self-audit runs at an exact named revision and contract-set digest.
- AC-041-002: native analysis reproduces a representative set of historical A0/A1 classes: duplicate/shadow authority, dependency boundary, bounded compatibility, projection-only, negative/bypass coverage.
- AC-041-003: unresolved mandatory contradictions/unknowns prevent the architecture receipt from claiming PASS unless governed waiver criteria explicitly allow them.
- AC-041-004: delete/rebuild of graph/workbooks preserves equivalent findings.
- AC-041-005: self-audit runs in Base mode without CogniCode/Chronos/LLM.
- AC-041-006: enhanced providers may strengthen evidence without changing the Base authority model.

## Acceptance

AC-UAT-016 plus A5 `ARCHITECTURE-CONFORMANCE-RECEIPT`.
