# SPEC-006 — Observation, evidence and knowledge ingestion

## Intent

Normalizar resultados de CogniCode, Chronos y futuros providers sin convertirlos en autoridad de verdad ni llenar SDDK de DTOs externos.

## Observation

Modelo conceptual mínimo:

```rust
struct Observation {
    observation_id: ObservationId,
    capability: CapabilityId,
    provider: ProviderBasis,
    subject: SubjectRef,
    subject_basis: ContentDigest,
    completeness: Completeness,
    status: ObservationStatus,
    claims: Vec<ObservedClaim>,
    raw_ref: Option<ExternalOrCasRef>,
    produced_at: LogicalOrRecordedTime,
}
```

## Requisitos

### OBS-001 — Provider anti-corruption

Wire DTOs terminan en adapter. Domain/verification/context usan Observation SDDK-owned.

### OBS-002 — Provenance

Toda Observation MUST conservar provider id/build/protocol/capability snapshot/config/analyzer/scenario basis cuando aplique.

### OBS-003 — Subject basis

Static observation MUST ligarse a source/content revision. Runtime observation MUST ligarse a executable/scenario/instrumentation basis.

### OBS-004 — Epistemic status

Closed set inicial:

```text
Complete
Partial
Unavailable
Incompatible
TimedOut
Cancelled
Failed
```

Nunca normalizar failure a empty success.

### OBS-005 — Evidence projection

Una Observation MAY materializar EvidenceRef/relations cuando un consumer la usa para justificar/verify/observe. No duplicar raw provider payload pesado si basta ref+digest.

### OBS-006 — Contradiction preservation

Dos observations contradictorias se conservan con provenance distinta. No universal score ni last-writer-wins.

### OBS-007 — Staleness

Observation queda stale cuando cambia cualquier basis relevante: subject, provider capability snapshot, analyzer config o scenario contract según policy.

### OBS-008 — Reuse

Si basis sigue vigente, ContextCompiler SHOULD reutilizar observation persistida antes de relanzar provider, salvo policy `fresh_required`.

### OBS-009 — Provider non-authority

Provider no puede emitir DecisionRecord, Gate PASS o Authority approval directamente.

### OBS-010 — Context projection

ContextCapsule recibe summaries/refs relevantes, nunca dump masivo del graph/trace provider.

## Mapping inicial

```text
CogniCode
  code.structure      → static Observation
  code.usages         → static Observation
  code.dependencies   → static Observation
  code.boundaries     → static Observation

Chronos
  runtime.trace       → runtime Observation
  runtime.summary     → runtime Observation
  runtime.behavior    → runtime Observation
```

## Acceptance

OBS-UAT-001..012.
