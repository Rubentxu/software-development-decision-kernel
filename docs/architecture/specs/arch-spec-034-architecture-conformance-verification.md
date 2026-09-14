---
id: arch-spec-034-architecture-conformance-verification
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-034 — Architecture Conformance Verification

## Intent

Verification SHALL evaluate explicit architectural claims against reproducible evidence rather than assign an opaque quality score.

## Requirements

- AC-034-001: Verify maps changed software units to affected contracts and minimal probes.
- AC-034-002: DebVerify independently challenges global assumptions and is not `verify --full`.
- AC-034-003: results preserve VERIFIED/CONTRADICTED/UNKNOWN/STALE/NOT_EVALUATED semantics.
- AC-034-004: deterministic probes run without LLM/provider where possible.
- AC-034-005: LLM assessments are INFERRED with model/input/output provenance.
- AC-034-006: Base mode works with CogniCode and Chronos absent.
- AC-034-007: receipts include exact revision/contract/knowledge/graph basis.
- AC-034-008: no universal weighted architecture score is emitted by the kernel.

## Acceptance

AC-UAT-007..010, 016, 043.
