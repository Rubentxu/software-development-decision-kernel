# SPEC-034 — Pack Contract

**Status:** Proposed

## Pack manifest

```yaml
id: sddk-incident
version: 1.0.0
requires:
  kernel: ">=2.0 <3"
provides:
  workflows:
    - incident-response
  capabilities:
    - incident.triage
    - incident.diagnose
  behaviors:
    - incident.severity-escalation
  views:
    - incident.timeline
integrates_with:
  - sddk-uat
conflicts_with: []
```

## Pack contents
- workflow definitions;
- capability/provider declarations;
- agents;
- schemas;
- policies;
- behaviors;
- projections/lenses;
- docs/examples;
- migrations if needed.

## Isolation
A pack cannot:
- write ledger bypassing validation;
- execute effects outside CapabilityExecutor;
- mutate kernel projections directly;
- introduce hidden recursive delegation.

## Discovery
Filesystem/workspace discovery can remain compatible with existing direction, but validation occurs before activation.

## Composition
Workflows can call pack workflows as subworkflows. Capability IDs form the semantic integration surface.
