# Software Alignment domain model

## Ubiquitous language

### UniversalConcern
Una preocupación transversal que no prescribe paradigma: `ResponsibilityClarity`, `Cohesion`, `Coupling`, `BoundaryIntegrity`, `ChangeLocality`, `StateSafety`, `EffectVisibility`, `DependencyDirection`, `FailureIsolation`, `ConcurrencySafety`, `SemanticOwnership`, `Duplication`, `TemporalCoupling`, `Testability`, `KnowledgeFreshness`.

### AlignmentLens
Interpretación opcional sobre concerns. Ejemplos namespaced:

- `core.boundaries`
- `architecture.hexagonal`
- `ddd.bounded-context`
- `oo.solid-grasp`
- `functional.effects-and-adt`
- `rust.type-state`
- `data-oriented.layout`
- `event-driven.message-boundaries`

### AlignmentAssessment
Resultado consultivo sobre un subject + concern + lens.

```text
ALIGNED | TENSION | MISALIGNED | ACCEPTED | REVIEW_DUE | UNKNOWN | NOT_APPLICABLE
```

### ContractViolation
Sólo existe si se contradice un constraint explícito `MUST/MUST_NOT`. No se usa para heurísticas.

### AlignmentTension
Hay fuerzas contrapuestas o indicios que merecen atención; puede ser intencional.

### ImprovementOpportunity
Direcciones posibles con tradeoffs, no acción requerida.

### ArchitecturalIntentSnapshot
Proyección compilada desde decisiones/config/invariantes/contracts; no sustituye sus fuentes.
