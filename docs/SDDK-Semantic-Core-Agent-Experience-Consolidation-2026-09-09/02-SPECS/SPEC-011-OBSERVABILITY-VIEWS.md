# SPEC-011 — Observability, metrics, analytics and views

## Consolidated model

- Telemetry: raw operational observations/events.
- Metrics: deterministic aggregations/projections.
- Analytics: higher-level queries/trends over metrics/facts.
- Views: presentation/read models.

These are layers, not independent authorities.

## CLI direction

Porcelain exposes `sddk status`, `sddk doctor`, and optional `sddk observe ...`; existing `metrics`, `analytics`, `telemetry`, `explore` commands remain plumbing/compat until migration.

## Requirements

Observability cannot mutate workflow state except by producing an ActionProposal that passes Authority Engine. Views reference provenance and support `--format json` for automation.
