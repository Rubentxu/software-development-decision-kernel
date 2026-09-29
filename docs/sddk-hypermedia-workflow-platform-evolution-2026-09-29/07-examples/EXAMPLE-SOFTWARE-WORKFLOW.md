# Ejemplo — Workflow software con steps aumentados

> Ejemplo conceptual. Los nombres exactos de schemas/fields se congelan en C6a.

```yaml
apiVersion: sddk.dev/v1alpha1
kind: Workflow
metadata:
  name: software-change
  version: 0.1.0

spec:
  augmentation_profiles:
    - core.context
    - core.evidence
    - software.static-intelligence
    - software.runtime-intelligence

  inputs:
    goal:
      type: GoalRef

  budgets:
    max_depth: 16
    max_nodes: 256
    no_progress_threshold: 3

  steps:
    - id: explore
      use: software.explore@v1
      with:
        goal: $inputs.goal

    - id: design
      use: software.architecture-design@v1
      needs: [explore]
      with:
        findings: $steps.explore.findings

    - id: tasks
      use: software.task-decomposition@v1
      needs: [design]

    - id: implement
      use: software.implement@v1
      needs: [tasks]
      map:
        over: $steps.tasks.work_items
        max_concurrency: 2

    - id: verify
      use: software.verify@v1
      needs: [implement]

  outputs:
    verification: $steps.verify.result
```

## Lo que el autor NO declara

No declara:

```text
cognicode-mcp command
chronos command
sddk plan evidence attach
sddk plan decision record
cycle transition recipe
ledger verify recipe
handoff prompt
session recovery recipe
```

## Resolved `design` StepRun posible

```yaml
step_definition: software.architecture-design@v1
step_digest: sha256:...
task_kind: software.design.architecture

resolved_context:
  - goal
  - exploration_findings
  - prior_decisions
  - active_work_items
  - architecture_facts

selected_skills:
  - core.architecture-review@v1
  - software.boundary-reasoning@v1

capabilities:
  required:
    - knowledge.read
  preferred:
    - code.structure
    - code.dependencies
    - code.boundaries

providers:
  code.structure:
    provider: cognicode
    capability_snapshot: sha256:...
  code.dependencies:
    provider: cognicode
    capability_snapshot: sha256:...

persistence:
  decisions: automatic
  evidence: automatic
  observations: automatic
  context_delta: automatic
```

## Ejemplo de contribution del agente

```json
{
  "outcome":"succeeded",
  "contributions":[
    {
      "kind":"decision",
      "payload":{
        "choice":"single durable context store over parallel session cache",
        "rationale":"preserves one canonical persistence boundary"
      }
    },
    {
      "kind":"artifact",
      "payload":{"contract":"ArchitectureDesignV1","body_ref":"cas:..."}
    }
  ]
}
```

ContributionReconciler crea DecisionRecord, ArtifactRef, semantic relations y ContextDelta. El prompt no contiene esas commands.
