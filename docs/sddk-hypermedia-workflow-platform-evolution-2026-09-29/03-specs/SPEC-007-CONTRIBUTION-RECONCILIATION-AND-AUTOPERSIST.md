# SPEC-007 — Typed contributions, reconciliation and automatic persistence

## Intent

Eliminar de los prompts la obligación de recordar `sddk plan evidence attach`, `decision record`, transitions y persistencia de outputs.

## Contribution contract

Un agent/executor devuelve contributions tipadas, no comandos internos de persistencia.

```json
{
  "step_run":"SR-12",
  "attempt":"A-2",
  "basis":"CTX-91",
  "outcome":"succeeded",
  "contributions":[
    {"kind":"decision","payload":{}},
    {"kind":"evidence","payload":{}},
    {"kind":"finding","payload":{}},
    {"kind":"artifact","payload":{}},
    {"kind":"context_delta","payload":{}}
  ]
}
```

## Requisitos

### CON-001 — Validate before persist

Toda Contribution se valida contra StepDefinition output contract + current basis + authority.

### CON-002 — Reconciler owns persistence orchestration

El reconciler transforma contributions admitidas en APIs existentes: planning, evidence, decisions, semantic relations, artifact CAS, event log.

### CON-003 — No direct agent DB writes

Agentes no escriben SQLite/CAS por bypass. Usan affordances/contribution endpoint.

### CON-004 — Idempotency

Cada contribution tiene stable idempotency key derivable de attempt + output identity. Retry no duplica decisions/evidence.

### CON-005 — Atomicity boundary

Si varias contributions forman un contract indivisible, el reconciler MUST persistir transaccionalmente o registrar typed partial failure con recovery action. No ocultar partial success.

### CON-006 — Rendering is projection

`proposal.md`, `design.md`, `tasks.md`, `verify-report.md` pueden mantenerse, pero deben poder renderizarse desde structured entities o quedar ligados por digest como artifacts; no son la única memoria.

### CON-007 — Tasks become WorkItems

El software pack default debe proyectar task decomposition a WorkItems/dependencies. `tasks.md` es human view.

### CON-008 — Decisions become DecisionRecords

Alternativa seleccionada, defer/reject/escalate con rationale debe quedar persistida cuando el StepDefinition declare decision output.

### CON-009 — Evidence auto-attach

Tests, provider observations, gate outputs y artifacts que cumplen contract se adjuntan automáticamente a work item/decision/goal según mapping del pack.

### CON-010 — Completion gate

StepRun sólo puede `Succeeded` cuando required contributions y evidence contract estén satisfechos.

### CON-011 — Human override

Un humano puede aportar/editar una contribution mediante affordance con provenance `human`, sujeto a mismas validaciones.

### CON-012 — Explain

`why` debe poder recorrer StepRun → Contribution → Evidence/Decision/Observation → source.

## Acceptance

CON-UAT-001..014.
