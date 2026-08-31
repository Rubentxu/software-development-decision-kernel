# Schema Examples

## Agent Capability Manifest

```yaml
id: agent://sddk-security-reviewer
version: 1
provides:
  - security.review
  - security.threat-model
inputs:
  schema: schema://security-input/v1
outputs:
  schema: schema://security-findings/v1
context:
  must_have:
    - source.changed
    - architecture.relevant
side_effect_class: none
tools:
  allow:
    - artifact.read
    - graph.query
model_policy: security-deep
```

## Model routing policy

```yaml
id: security-deep
candidates:
  - selector: provider=anthropic,tier=premium
    priority: 100
  - selector: provider=openai,tier=premium
    priority: 95
  - selector: provider=local,tier=capable
    priority: 60
hard_constraints:
  data_classification: approved
failover:
  max_attempts: 3
  on_quota_exhausted: reroute
```

## OrchestratorSignal

```yaml
id: sig-123
type: workflow.replan.required
trigger_event: evt-456
correlation_id: wf-10
summary:
  reason_codes: [verification_repeatedly_failed]
state_refs:
  workflow: wf-10
  node: nr-4
alternatives: []
allowed_decisions:
  - retry_with_changed_strategy
  - request_human_input
  - abort_node
```

## Receipt

```yaml
id: receipt-123
capability: git.apply-patch
proposal_hash: sha256:...
policy_hash: sha256:...
actor: agent://sddk-implementer
scope:
  paths: ["crates/sddk-engine/**"]
verification:
  status: passed
  evidence_refs: [evidence:test-123]
```
