# Changeset — Dynamic Workflows & SDD Adaptive

Date: 2026-08-19

## Why
Refine SDDK after studying recent dynamic/programmatic agent workflow approaches and evaluating whether the current SDD pipeline can be made more compact without losing guarantees.

## Architectural result

```text
Template → Compiler → IR → Validator → Runtime → Dynamic Graph Revisions
```

The Supervisor proposes/replans; the deterministic runtime remains authority.

## SDD result
Add experimental:

```text
SHAPE → BUILD ⇄ CONVERGE → INTEGRATE
```

SDD quality becomes a set of invariants represented by `ChangeContract`, not a requirement to execute every legacy document as a separate agent phase.

## Compatibility
- A-full is retained as reference/baseline.
- No previous documents are deleted by this change.
- Existing UAT/failover/Cockpit/supply-chain architecture remains compatible.
- `Phase/CyclePath` migration is still gradual.

## New primary docs
- ADR-037, ADR-038, ADR-039.
- SPEC-037, SPEC-038, SPEC-039, SPEC-040.
- SDD-ADAPTIVE-WORKFLOW.md.
- WORKFLOW-PATTERNS.md.
- SPIKE-006 and SPIKE-007.
