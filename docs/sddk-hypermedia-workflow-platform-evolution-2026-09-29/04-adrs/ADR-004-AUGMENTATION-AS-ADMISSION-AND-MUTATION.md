# ADR-004 — Step augmentation as admission + deterministic mutation

**Status:** Proposed

## Context

A workflow author should not manually wire context loading, SDDK persistence commands, provider calls, evidence capture and common skills into every step.

## Decision

Introduce a deterministic Step Admission/Augmentation pipeline inspired by Kubernetes admission, but owned by SDDK:

```text
StepDefinition
→ validate/admit
→ resolve context
→ select skills
→ aggregate capabilities
→ resolve providers
→ apply policy/authority/budgets
→ ExecutableStep
```

Augmentation modifies the execution plan for a StepRun, not the authored WorkflowDefinition.

## Consequences

- default best practices become systemic;
- custom workflows inherit SDDK semantics automatically;
- explainability/provenance of injected behavior is required;
- registry snapshots become part of execution basis.

## Rejected

- copy common prompt fragments into every workflow;
- arbitrary user mutation webhooks in first version;
- LLM-based provider/capability resolution for deterministic cases.

## Initial scope

Only built-in deterministic augmentors in C6. Third-party augmentors are deferred until stable capability/authority contracts exist.
