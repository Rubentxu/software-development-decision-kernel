# SPEC-003 — Reusable StepDefinition and StepRun

## Intent

Convertir el step en unidad reutilizable independiente del workflow concreto.

## StepDefinition

### STEP-001 — Stable identity

`StepDefinition` MUST tener namespace, name, semantic version y content digest.

### STEP-002 — Task kind

MUST declarar un `task_kind` semántico utilizado por SkillRegistry/Augmentor.

Ejemplos:

```text
software.design.architecture
software.implement.change
research.investigate
content.write.section
content.review.consistency
```

### STEP-003 — Typed contracts

MUST declarar inputs y outputs por contract/schema ref, nunca únicamente por path de fichero.

### STEP-004 — Context requirements

MAY declarar context keys requeridas/preferidas, p. ej. `prior_decisions`, `active_work_items`, `architecture`, `source_evidence`.

### STEP-005 — Capabilities

MUST distinguir `required`, `preferred`, `optional`. Ausencia REQUIRED bloquea. Ausencia PREFERRED produce gap/degradation visible.

### STEP-006 — Evidence contract

MUST declarar tier mínimo y tipos de evidence que el reconciler espera antes de completion si aplicable.

### STEP-007 — Authority class

MUST declarar ceiling de side effects esperado; authority real se reevalúa por run/action.

### STEP-008 — No provider binding by default

Provider name no aparece en StepDefinition salvo `provider_override` explícito en una instancia/debug policy.

### STEP-009 — No workflow coordinates

Un reusable StepDefinition MUST NOT conocer IDs de fases o vecinos concretos. Sólo contratos.

## StepRun

### STEPRUN-001 — Identity and lineage

Ligado a WorkflowRun, workflow revision, StepDefinition digest, inputs digest y attempts.

### STEPRUN-002 — State

Closed set inicial:

```text
Pending
Ready
Admitted
Running
Waiting
Blocked
Succeeded
Failed
Cancelled
Compensated
```

No reutilizar strings de fases legacy como autoridad.

### STEPRUN-003 — Context basis

Cada attempt guarda ContextBasis/ContextLease usados.

### STEPRUN-004 — Typed outputs

Reutilizar/expandir `TypedChildOutput`; ningún output entra al downstream si viola output schema.

### STEPRUN-005 — Idempotency

Reconcile de un StepRun terminal con mismos receipts es no-op. Re-execution requiere nueva AttemptId o política de retry explícita.

### STEPRUN-006 — Reuse

Un StepDefinition puede instanciarse en varios workflows sin duplicar implementation prompt o provider recipes.

## Execution modes

Un StepDefinition MAY admitir:

```text
agent
pure_function
capability_call
subworkflow
human_gate
```

El mode concreto puede resolverse por policy si hay más de uno compatible.

## Acceptance

STEP-UAT-001..012.
