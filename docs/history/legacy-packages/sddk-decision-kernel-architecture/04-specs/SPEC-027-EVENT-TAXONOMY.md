# SPEC-027 — Canonical Event Taxonomy

**Status:** Proposed

## Naming
Lowercase dot-separated past-tense facts where possible:

```text
workflow.run.started
workflow.node.ready
agent.execution.started
provider.quota.exhausted
provider.circuit.opened
model.route.selected
```

## Categories

### Workflow
`workflow.run.*`, `workflow.node.*`, `workflow.join.*`, `workflow.wait.*`

### Execution
`agent.execution.*`, `attempt.*`, `tool.execution.*`, `host.session.*`

### Routing/provider
`model.route.*`, `provider.health.*`, `provider.circuit.*`, `provider.quota.*`

### Context
`context.capsule.*`, `context.object.read`, `context.object.stale`

### Governance
`proposal.*`, `policy.*`, `approval.*`, `capability.*`, `receipt.*`

### Evidence/UAT
`evidence.*`, `uat.plan.*`, `uat.run.*`, `uat.defect.*`, `uat.signoff.*`

### Human
`human.review.*`, `human.decision.*`

### Pack/runtime
`pack.*`, `behavior.*`, `supervisor.signal.*`, `supervisor.decision.*`

## Severity is separate from type
An event type does not encode INFO/WARN/ERROR. Journal severity is a projection policy.

## Versioning
Envelope version and payload schema version are explicit. Consumers ignore unknown additive fields; breaking payload changes require a new schema version/type migration.

## No log strings as domain events
A raw log line may be evidence/diagnostic metadata. Domain events contain typed meaning extracted by adapters/classifiers.
