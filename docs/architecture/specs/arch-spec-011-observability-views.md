---
id: arch-spec-011-observability-views
package_local_id: SPEC-011
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-011-OBSERVABILITY-VIEWS.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-011 — OBSERVABILITY-VIEWS

> **Mirror of package SPEC `SPEC-011`.** Repository-native identifier is `arch-spec-011-observability-views` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-011` |
| Repository native | `arch-spec-011-observability-views` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-011-OBSERVABILITY-VIEWS.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

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
