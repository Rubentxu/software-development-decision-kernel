# SPEC-024 — Alignment Lenses

## Core universal concerns

- responsibility clarity;
- cohesion/coupling/change locality;
- dependency/boundary integrity;
- state/effect management;
- invariants;
- duplication/semantic ownership;
- testability/observability/failure isolation;
- temporal/concurrency coupling;
- knowledge freshness and uncertainty.

## Lens contract

```text
AlignmentLens
  id
  version
  applies_when
  concerns[]
  required_evidence[]
  heuristics[]
  workbook_contributions[]
  suggestion_rules[]
```

Initial candidate lenses:

`hexagonal`, `ddd`, `object-oriented`, `functional`, `data-oriented`, `event-driven`, `actor`, `ecs`, `rust`, `kotlin`.

Auto detection may recommend lenses; only config/policy activates normative expectations. Lens results are advisory unless backed by explicit contracts.
