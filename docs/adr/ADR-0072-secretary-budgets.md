# ADR-0072 — Secretary Budgets — per-call and cycle composition

**Status:** accepted
**Type:** decisions
**Created:** 2026-08-30
**Created in cycle:** [[p-52b95ef55999f9de/secretary-orchestrator]]
**Supersedes:** none
**Suppressed:** none
**Renamed from:** none
**Amends:** none

---

## Context

El secretary Runtime necesita budgets para sus decisiones, compuestos a partir de ADR-0068 (Bounded Execution: `max_wall_ms`, `no_progress_threshold`) y ADR-0070 (Sizing Budgets Advisory). El secretary NO crea un dominio `Budgets.agent` separado — eso introduciría connascence Name entre dominios distintos.

---

## Decisión

### Composición de budgets

El secretary usa **composición** de budgets existentes, no un budget nuevo:

**Per-call budgets:**
- `per_call_wall_ms`:wall-clock time máximo por evaluación de un evento individual
- `per_call_tokens`:tokens máximo por llamada LLM (si aplica en Stage 1+)

**Cycle-budget composition:**
- Heredado de `Budgets.cycle` (definido en ADR-0068 y ADR-0070)
- El secretary no tiene budget independiente — compone el cycle-budget con su per-call advisory

**Estructura de advisory:**
```yaml
metric: per_call_wall_ms | per_call_tokens
forecast: <N>
budget: <M>
recommendation: "within_budget" | "approaching_limit" | "exceeded"
rationale: "advisory; never blocks per ADR-0070 separation_invariant"
```

### Naming collision note

> **ADR-0072** ≠ **`docs/adr/ADR-0050-true-concurrent-parallel.md`**
>
> `ADR-0072` en vault (`~/.sddk-knowledge/sddk-framework/adrs/ADR-0072-secretary-budgets.md`) y `docs/adr/ADR-0050-true-concurrent-parallel.md` son archivos **distintos** con numeración diferente. El slot `ADR-0072` en vault es la superficie canónica del secretary; `ADR-0050` en `docs/adr/` es un ADR de concurrencia del proyecto público，两者 no colisionan.

---

## Consequences

### Positive

- Sin connascence Name: no se introduce un dominio `Budgets.agent` nuevo
- Reutiliza ADR-0068 y ADR-0070 tal cual — zero new semantics
- Advisory record estructurado permite monitoring sin bloqueo

### Negative

- El secretary no tiene budget propio hard — todo es advisory
- Si `Budgets.cycle` no está configurado, el advisory es no-op

### Neutral

- Stage 0 no implementa runtime behavior — los budgets son documentación
- Stage 1+ decide cómo consumir el advisory record

---

## References

- [[ADR-0068-bounded-execution]]
- [[ADR-0070-sizing-budgets-advisory]]
- [[SPEC-042-secretary-runtime]]
