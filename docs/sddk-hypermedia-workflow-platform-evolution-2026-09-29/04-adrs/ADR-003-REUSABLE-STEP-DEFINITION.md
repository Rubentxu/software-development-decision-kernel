# ADR-003 — StepDefinition as reusable workflow unit

**Status:** Proposed

## Context

Phase prompts are tied to specific software lifecycle locations and files, reducing reuse. Future user workflows need stable units whose contracts do not depend on neighbors.

## Decision

Introduce versioned `StepDefinition` with task kind, typed input/output contracts, context requirements, capability requirements, evidence contract, authority class and augmentation profiles.

`StepRun` is the runtime instance; it owns attempts, context basis and typed outputs.

## Consequences

- steps become reusable across workflows;
- workflow compiler can type-check wiring;
- skills/providers can be selected semantically;
- file-based handoffs become projections rather than interfaces.

## Rejected

- reuse whole prompt files without contracts;
- make every skill a step;
- generic god-steps such as an unconstrained `core.execute`.

## Guardrail

StepDefinition never grants authority and does not bind concrete providers by default.
