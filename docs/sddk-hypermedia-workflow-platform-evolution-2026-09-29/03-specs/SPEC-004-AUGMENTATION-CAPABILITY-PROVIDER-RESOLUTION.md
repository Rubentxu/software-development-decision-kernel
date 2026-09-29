# SPEC-004 — Step augmentation, capability resolution and provider selection

## Intent

Hacer que los workflows de usuario obtengan context, skills e intelligence integrations sin codificar esas decisiones en prompts.

## Pipeline

```text
StepDefinition
 → Step Admission
 → Context requirement resolution
 → Skill selection
 → Capability aggregation
 → Provider resolution/negotiation
 → Governance/authority
 → ExecutableStep
```

## Requisitos

### AUG-001 — Deterministic admission

Con misma StepDefinition, ContextBasis, registry snapshots y policy digest, admission MUST producir mismo resultado.

### AUG-002 — Skill selection

Reutilizar `SkillRegistry::select_for(task_kind, context_keys)`. Skills aportan expertise/instruction fragments y capability expectations; nunca authority.

### AUG-003 — Capability merge

Capabilities del step + skills + augmentation profiles se combinan con reglas deterministas y provenance. `required` domina `preferred`, que domina `optional`.

### AUG-004 — Provider registry

Un ProviderDescriptor declara capabilities, lifecycle, protocol versions y health/capability snapshot.

### AUG-005 — Capability negotiation

Resolver por capability, no por product version. Product build/version permanece metadata reproducible.

### AUG-006 — Provider selection

Algoritmo inicial:

1. filtrar READY + protocol compatible;
2. filtrar capability exacta;
3. aplicar explicit override si válido;
4. aplicar local policy/preference;
5. ordenar determinísticamente por priority + provider id;
6. registrar selection receipt.

No usar un LLM para seleccionar provider básico.

### AUG-007 — Required/preferred semantics

- REQUIRED sin provider => StepRun Blocked con recovery affordances.
- PREFERRED sin provider => ejecutar degradado sólo si StepDefinition permite Base path; generar EvidenceGap.
- OPTIONAL => no bloquear.

### AUG-008 — Automatic provider observations

Si un capability provider se ejecuta, su output pasa por normalizador a Observation antes de consumo/persistencia.

### AUG-009 — Augmentation profiles

Profiles son composición declarativa de defaults, no bundles monolíticos de prompts.

Ejemplos:

```text
core.context
core.evidence
software.static-intelligence
software.runtime-intelligence
research.sources
content.consistency
```

### AUG-010 — Runtime discover

Un step MAY solicitar capability adicional durante ejecución sólo si WorkflowIR concede `Discover` y policy lo admite. Se registra plan revision/receipt.

### AUG-011 — No hidden mutation

El Augmentor no cambia el desired workflow. Produce `ExecutableStep`/resolved execution plan para un StepRun concreto.

### AUG-012 — Explainability

Debe existir una explicación estructurada de por qué una capability/skill/provider fue añadida o rechazada.

## Acceptance

AUG-UAT-001..014.
