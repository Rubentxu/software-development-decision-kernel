# Open questions y spikes

## SP-01 — Resource URI identity

**Pregunta:** ¿qué IDs ya existentes pueden reutilizarse y qué grammar evita duplicar identity?  
**Output:** ADR amendment + parser tests.  
**No decidir:** URL HTTP concreta.

## SP-02 — Persistent context storage shape

Evaluar:

1. canonical event + projection;
2. table projection + CAS payload;
3. CAS only + index.

Criterios: replay, query, migration, retention, write amplification, one-authority.

## SP-03 — StepDefinition vs OperatorContract overlap

Inspeccionar `operator_contract.rs`, `SkillDefinition` y TypedChildOutput antes de crear schemas. Objetivo: StepDefinition referencia/reutiliza Operator input/output schemas en vez de duplicarlos.

## SP-04 — WorkflowDefinition syntax

YAML es UX, no domain. Evaluar schema mínimo y si el current WorkflowTemplate puede ser la normalized domain representation directa.

## SP-05 — Affordance derivation boundary

¿Engine application service o agent-experience module? Criterios: access a authority/policy/capability state sin invertir dependencies.

## SP-06 — Capability ontology

Definir naming rules:

```text
<domain>.<noun-or-action>
code.dependencies
runtime.trace
knowledge.read
artifact.publish
```

Evitar explosion de capabilities equivalentes por provider.

## SP-07 — Augmentation precedence

Resolver conflictos entre:

```text
StepDefinition
SkillDefinition
AugmentationProfile
Workflow override
User/project policy
```

Propuesta a falsar:

- policy/authority puede restringir todo;
- Step required no puede ser rebajado por skill/profile;
- workflow instance puede elevar optional→preferred/required sólo si policy lo admite;
- provider override no cambia capability semantics.

## SP-08 — Provider observation reuse policy

Determinar staleness key por capability. Static graph no tiene la misma basis que runtime trace. No usar una universal cache key simplista.

## SP-09 — Markdown projections

Decidir qué artifacts actuales son:

- canonical structured entity rendered to Markdown;
- human-authored artifact persisted by digest;
- transitional legacy authority.

No intentar estructurar toda prosa indiscriminadamente.

## SP-10 — Hypermedia MCP UX

Comparar:

A. 4 tools genéricas (`get`, `invoke`, `expand`, `validate`);  
B. dynamic tools generated per action;  
C. hybrid common porcelain + generic fallback.

Criterios: token footprint, discoverability, schema fidelity, compatibility.

## SP-11 — Book domain provider strategy

Para C7, no necesitamos un buscador universal. Seleccionar un source provider controlable y reproducible o fixtures + una fuente real limitada. Objetivo es arquitectura, no breadth de research.

## SP-12 — Step semantic memoization (defer C8)

Medir primero repeated read-only calls. Sólo abrir si provider calls/context recomputation son coste material. Nunca side effects.

## Decisiones explícitamente diferidas

- marketplace remoto;
- arbitrary third-party mutation controllers;
- pack signatures/trust federation;
- HTTP server permanente;
- daemon obligatorio;
- cloud orchestration;
- generic distributed scheduler;
- vector DB obligatoria;
- renombrar el producto por salir de software.
