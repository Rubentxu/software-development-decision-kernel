# SPEC-026 — Metrics, Charts and Attention

## Metrics

Measured values and derived values retain scope+basis+producer+version.

## Charts

- pie only for meaningful part-to-whole (fresh/stale/unknown);
- bar for categorical module/context comparisons;
- line for per-cycle trends;
- scatter for risk×uncertainty, churn×risk, distance×connascence;
- DSM/heatmap for dependency/coupling matrices;
- Pareto for persistent smells/debt.

## Attention model

A composite may rank **where to inspect next**:

`risk × impact × change-frequency × uncertainty × persistence`.

It MUST be labelled `AttentionPriority`, never `ArchitectureQualityScore`.
