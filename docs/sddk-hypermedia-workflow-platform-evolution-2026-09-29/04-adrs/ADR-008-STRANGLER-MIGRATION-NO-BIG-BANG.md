# ADR-008 — Strangler migration; no big-bang workflow rewrite

**Status:** Proposed

## Context

The repository already has production-relevant lifecycle behavior, tests, receipts and compatibility obligations. Replacing prompts, manifests, runtime and context simultaneously would make failures hard to attribute.

## Decision

Migrate one authority at a time:

1. fix recovery contradictions;
2. add durable context/hypermedia read path;
3. introduce StepDefinition/WorkflowDefinition beside legacy;
4. compile and compare, no effects;
5. cut over one default workflow slice;
6. move persistence recipes into ContributionReconciler;
7. remove obsolete prompt/control-flow only after UAT.

## Consequences

- more temporary adapters;
- longer period of dual representation;
- lower release risk and better falsifiability.

## Rules

- never dual-execute side effects for shadow comparison;
- receipts state which authority executed;
- legacy code removal requires observed consumers migrated;
- rollback path maintained per slice until certification.
