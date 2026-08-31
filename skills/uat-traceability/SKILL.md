---
name: uat-traceability
description: "Trigger: uat-traceability, matriz de trazabilidad, traceability matrix. Build requirement → feature → scenario → status chains for the UAT report."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: sddk-framework
  version: "1.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `uat-reporter`.

## Purpose

Keep the traceability chain intact: `requirement_ref → feature → scenario → session → verdict`. This is what makes a UAT report defensible to an architect.

## Chain rules

1. Every `feature` with a `requirement_ref` must map to an existing PRD/requirement identifier (e.g. RF-016).
2. Every `scenario` must belong to exactly one feature.
3. Every session result must reference a `scenario_id` that exists in the plan — orphan results are validation errors.
4. Rollups: feature coverage = (non-fail + non-blocked scenarios) / total scenarios. Report coverage = covered scenarios / total.

## Report shape

```yaml
features:
  - id: F-01
    name: Crear proyecto
    coverage_pct: 100
    scenarios:
      - scenario_id: S-1
        status: PASS
        executor: human
```

## Commands

```bash
sddk uat report --release <tag> --plan <plan> --sessions <session-files...>
```

## References

- `agents/uat-reporter.md` — the report contract
- ADR-012 §4 (Feature → Scenario → Session → Evidence → Verdict)
