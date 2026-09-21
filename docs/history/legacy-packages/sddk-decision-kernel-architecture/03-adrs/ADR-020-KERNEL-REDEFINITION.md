# ADR-020-KERNEL-REDEFINITION — Redefine SDDK as Software Development Decision Kernel

**Status:** Accepted


## Context
The original name coupled the product identity to Specification-Driven Development. The architecture now includes generic workflow orchestration, UAT, incident response, provider routing, event sourcing, reactive behaviors, graph projections, governance and operational analytics.

## Decision
Keep the SDDK brand and redefine it as **Software Development Decision Kernel**. Treat Specification-Driven Development as the `sddk-sdd` pack.

## Consequences
- The kernel vocabulary must not assume SDD phases.
- Documentation separates product semantics from the SDD pack.
- Existing SDD workflows remain supported through compatibility compilation.
- New features should be justified as kernel capability, generic pack capability, or specific pack capability.

## Rejected
Renaming the repository/product: unnecessary migration cost and loss of continuity.
