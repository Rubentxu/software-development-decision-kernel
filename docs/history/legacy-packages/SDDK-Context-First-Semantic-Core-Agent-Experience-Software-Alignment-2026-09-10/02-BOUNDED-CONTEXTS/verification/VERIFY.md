# Verify — recent-change verification

## Única responsabilidad

Responder: **¿qué cambió recientemente, qué puede verse afectado y qué conocimiento/evaluaciones necesitan actualizarse antes de continuar?**

## Owned outputs

```text
VerifyScope
VerifyReceipt
EvidenceGap
DeepeningCandidate
VerificationObservationRef
```

## Pipeline definitivo

```text
previous KnowledgeBasis
      +
Git/worktree delta
      |
      v
fingerprint dimension diff
      |
      v
changed units
      |
      +--> base deterministic checks
      |
      +--> CodeIntelligencePort AnalyzeDelta (si disponible/útil)
      |
      v
impact closure + staleness candidates
      |
      v
retrieve current Knowledge Cards
      |
      v
Alignment AssessDelta
      |
      v
LLM targeted evaluator only for UNKNOWN/TENSION ambiguity
      |
      v
publish assertion/assessment updates
      |
      v
refresh affected projections
      |
      v
Governance evaluates explicit release policies
      |
      v
VerifyReceipt
```

## No hace

- full repository analysis;
- full LLM review;
- Chronos por defecto;
- reescribir decisiones/tradeoffs;
- convertir Alignment advice en gate.
