# SPEC-008 — Packs, user workflows and reusable catalogs

## Intent

Permitir que SDDK evolucione más allá de software development sin diluir el kernel ni duplicar infraestructuras.

## Pack

Un pack MAY registrar:

- namespaced StepDefinitions;
- WorkflowDefinitions;
- skills;
- augmentation profiles;
- capability semantic mappings;
- renderers/projections;
- policies acotadas;
- schemas/contracts;
- UAT fixtures.

Un pack MUST NOT registrar:

- un scheduler paralelo;
- una fact log paralela;
- un authority engine alternativo;
- wildcard capabilities;
- provider DTOs en contratos del core.

## Requisitos

### PACK-001 — Namespacing

Ejemplos: `core.*`, `software.*`, `content.*`, `research.*`.

### PACK-002 — Versioned contracts

Step/workflow refs incluyen versión o compatible range resuelto a digest concreto al compilar.

### PACK-003 — Reuse

Un WorkflowDefinition puede usar StepDefinition de varios packs si contracts/capabilities son compatibles.

### PACK-004 — Software pack

Las fases actuales se migran progresivamente a `software.*`. Git/testing/release assumptions no deben filtrarse a `core.*`.

### PACK-005 — Core generic catalog

Candidatos iniciales, sólo si UAT demuestra reuse:

```text
core.research
core.review
core.approve
core.publish-artifact
core.wait-for-event
```

No promocionar `core.plan`/`core.execute` genéricos si se vuelven god-steps.

### PACK-006 — User-authored workflow

Usuario puede crear workflow declarativo sin editar prompts de SDDK ni implementar persistencia/handoff.

### PACK-007 — User-authored step

Custom StepDefinition necesita schema, task_kind, capabilities, evidence contract y execution binding registrado. No aceptar arbitrary shell como default trusted step.

### PACK-008 — Installation/discovery

Registry discovery MUST ser deterministic and scoped (framework/user/project) con precedence explícita y receipts de resolved versions.

### PACK-009 — Portability proof

Antes de declarar core domain-neutral, ejecutar segundo dominio end-to-end. Se selecciona `content.book` como prueba.

### PACK-010 — Compatibility

Pack incompatible no impide Base startup; sólo bloquea workflows que lo referencian.

## Acceptance

PACK-UAT-001..012.
