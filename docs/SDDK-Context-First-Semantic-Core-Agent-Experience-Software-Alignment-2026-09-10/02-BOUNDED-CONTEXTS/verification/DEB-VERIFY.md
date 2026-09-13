# Deb-Verify — whole-project reconciliation

## Única responsabilidad

Responder: **¿el baseline acumulado describe todavía correctamente el proyecto completo y dónde hay deuda, drift, contradicciones o conocimiento insuficiente?**

## Owned outputs

```text
DebVerifyPlan
ProjectKnowledgeBaselineReceipt
DebtClassificationDelta
GlobalEvidenceGap
ReconciliationSummary
```

## Pipeline

```text
latest KnowledgeBasis
  -> source manifest reconciliation
  -> KMT branch validation/rebuild
  -> global/stratified static provider analysis
  -> expected/observed context+dependency recomputation
  -> workbooks global refresh candidates
  -> risk × uncertainty ranking
  -> progressive LLM review by focus
  -> selected Chronos/golden scenarios
  -> stale decision/tradeoff/invariant detection
  -> contradiction reconciliation
  -> new ProjectKnowledgeBaselineReceipt
```

## Debt taxonomy

- Implementation Debt
- Architecture/Design Debt
- Knowledge Debt

Debt no significa automáticamente “arreglar”. Puede ser `ACCEPTED` con DecisionRef y revisit trigger.

## No es

`verify --all`. Puede descubrir conocimiento incorrecto en áreas sin cambios recientes y rebaselinar análisis.
