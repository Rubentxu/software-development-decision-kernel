# SPEC-020 — Capability Registry

**Status:** Proposed

## Purpose
Decouple workflow intent from concrete agents/models/tools.

## Capability ID
Use stable semantic IDs:

```text
architecture.design
architecture.review
security.threat-model
testing.plan
testing.execute
uat.execute
incident.triage
git.apply-patch
```

## Provider manifest

```yaml
provider_id: agent://sddk-architect
provides:
  - capability: architecture.review
    quality_tier: high
    side_effects: none
    expected_output: schema://architecture-review/v1
context:
  requires:
    - architecture.current
    - decisions.active
tools:
  allowed:
    - graph.query
    - artifact.read
routing:
  model_policy: architecture-deep
```

A capability can have implementations by:
- logical agent;
- deterministic tool;
- human;
- remote service.

## Resolution
Registry returns eligible implementations. Execution Router chooses route using policy/health/budget/history.

## Validation
On pack load:
- unique provider IDs;
- all workflow-required capabilities resolve or are explicitly external;
- schemas exist;
- side-effect classification is present;
- required permissions are satisfiable.

## Non-goal
Capability registry does not itself decide runtime availability; it declares semantic supply. Provider Health/Execution Router apply operational state.
