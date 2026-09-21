---
id: arch-spec-038-architecture-mutation-and-counterfactual-probes
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-038 — Architecture Mutation and Counterfactual Probes

## Intent

Critical architecture guards SHALL be falsifiable, and proposed structural changes MAY be evaluated before repository mutation.

## Requirements

- AC-038-001: mutation probes run only in disposable sandbox/worktree.
- AC-038-002: each mutation names the expected contract/guard and records whether it detected the injected violation.
- AC-038-003: initial critical mutations cover provider-type leak, Alignment→Governance/Instruction, workbook canonical write and second canonical writer.
- AC-038-004: counterfactual candidate graph is EPHEMERAL and cannot become canonical state.
- AC-038-005: counterfactual results report affected contracts, unknowns, new boundary cycles/conflicts and required compatibility facades.
- AC-038-006: LLM-generated graph deltas must pass deterministic schema/identity validation.

## Acceptance

AC-UAT-010, 040, 042.
