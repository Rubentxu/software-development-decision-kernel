# SPEC-002 — WorkflowDefinition, compilation and single execution authority

## Intent

Eliminar la coexistencia de varias autoridades de workflow convergiendo authored intent, IR y runtime.

## Modelo

```text
WorkflowDefinition (authored/versioned)
        ↓ validate
WorkflowTemplate / normalized AST
        ↓ compile
WorkflowIR (content-addressed executable topology)
        ↓
WorkflowRuntime
```

## Requisitos

### WF-001 — WorkflowDefinition

Debe expresar:

- metadata/name/version;
- typed inputs/outputs;
- StepDefinition refs;
- dependency edges;
- guards;
- sequence/parallel/map/choice/loop/gate/wait/subworkflow composition;
- workflow-level budgets;
- invariants;
- allowed expansion permissions;
- augmentation profile refs;
- failure/retry policy defaults.

### WF-002 — No embedded infrastructure recipes

WorkflowDefinition MUST NOT necesitar comandos SDDK, provider wire calls, shell recipes de persistence o prompt handoff protocols.

### WF-003 — Compile-time contract checking

El compiler MUST detectar:

- StepDefinition inexistente/incompatible;
- input contract sin productor/provider;
- output→input type mismatch;
- cycles no permitidos;
- operator no estable para schema version;
- capability REQUIRED imposible de satisfacer sin fallback permitido;
- authority requirement imposible bajo workflow policy;
- budget fuera de hard limits;
- subworkflow incompatible;
- expansion permission no concedido.

### WF-004 — Content-addressed IR

WorkflowIR MUST identificar digest de definición, StepDefinitions, policies y compilation basis. Mismo input normalizado + mismas versiones => mismo digest.

### WF-005 — Runtime authority

Sólo WorkflowRuntime puede avanzar StepRuns. Un prompt/orchestrator externo puede solicitar una affordance; no puede declarar internamente un step completado sin receipt/transition admitida.

### WF-006 — Dynamic mutation via plan revision

Discover/Replan/Map expansion MUST producir revisión tipada/auditable. Nunca modificar silenciosamente el WorkflowDefinition original.

### WF-007 — Static workflow compatibility

El manifest actual puede actuar temporalmente como input/compat decoder. Durante migración, un equivalence compiler MUST comparar topología/gates/outputs antes de cutover.

### WF-008 — Generated workflows

LLMs MAY proponer WorkflowDefinition. La propuesta MUST pasar compiler/admission. El LLM no ejecuta directamente YAML inventado.

### WF-009 — User-defined workflows

Un usuario MAY construir workflows únicamente a partir de contracts públicos registrados. No se exige modificar prompts internos de SDDK.

### WF-010 — No second scheduler

Packs, skills y providers no pueden crear schedulers paralelos. Subworkflows continúan bajo WorkflowRuntime.

## Ejemplo mínimo

```yaml
apiVersion: sddk.dev/v1alpha1
kind: Workflow
metadata:
  name: investigate-and-review
  version: 1.0.0
spec:
  inputs:
    subject: SubjectRef
  steps:
    - id: investigate
      use: core.research@v1
    - id: review
      use: core.review@v1
      needs: [investigate]
  outputs:
    result: review.result
```

## Acceptance

WF-UAT-001..012.
