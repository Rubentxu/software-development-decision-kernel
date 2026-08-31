# ADR-029-UAT-BOUNDED-CONTEXT — Extract UAT as a bounded context and workflow pack

**Status:** Accepted


## Context
The current UAT model is already sophisticated but is too large to remain a generic kernel module.

## Decision
Preserve UAT semantics while extracting `uat-domain`, `uat-app` and adapters/views. Model campaign, scenario run, defect, retest, waiver, acceptance and release sign-off explicitly.

## Consequences
UAT becomes the strongest proof that the generic kernel can host a complex non-SDD domain without contaminating core semantics.
