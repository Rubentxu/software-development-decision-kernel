---
id: INC-DEBT-023-lints-advisory-no-expansion-cycle
title: "Lints advisory sin ciclo de expansión programado (transition_outcome_used, execution_outcome_as_synthesis)"
status: closed
severity: low
priority: P3
fingerprint: "b56d8742"
fingerprint_aliases: []
cluster_id: CL-05
created: 2026-09-13
created_by: sddk-debt-verify (conformance-closeout-2026-09-13)
owner: rubentxu74
---

# INC-DEBT-023-lints-advisory-no-expansion-cycle — Lints advisory sin ciclo de expansión programado

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`deprecated_patterns.toml` contiene dos lints en modo advisory sin un ciclo de
trabajo que planifique su expansión a deny o su remoción:

- `transition_outcome_used` — 24 hits activos, funciona como regression guard;
  no hay decisión registrada sobre si debe subir a deny ni cuándo.
- `execution_outcome_as_synthesis` — 0 hits, corpus pending de re-auditoría
  (re-auditado en WU-C2 del ciclo conformance-closeout-2026-09-13 sin cambio
  de estado).

La deuda es de docs/gobernanza: los lints existen, funcionan, pero no tienen
owner, deadline ni criterio de cierre.

## Rationale

Severity **low**: no afecta comportamiento en producción; los lints advisory no
bloquean y no hay regresiones asociadas. Priority **P3**: sin urgencia, pero
el requisito cycle-7b (Debt lifecycle, ADR-0047) exige INC-NNN para cualquier
hallazgo pre_existing con evidencia completa en un verdict
PASS_WITH_WARNINGS. Cluster **CL-05 (smells/docs)**.

Evidencia: `docs/architecture/lints/deprecated_patterns.toml:7-9`, conteo de
hits verificado por el cluster smells del debt-verify del ciclo
conformance-closeout-2026-09-13 (`debt-findings.json`, FIND-000007/D7,
fingerprint `b56d8742…`).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-13 | sddk-debt-verify | created | FIND-000007 (D7, pre_existing) from cycle conformance-closeout-2026-09-13 |
| 2026-09-20 | orchestrator (auto-run) | closed — decision recorded, no expansion cycle scheduled | R17 diagnostics cycle follow-up; evidence below |

## Resolution

**Decision (2026-09-20): cerrado por decisión registrada, sin ciclo de
expansión.** La deuda era de gobernanza (owner/deadline/criterio de
cierre ausentes), no de código. La decisión técnica ya vivía en el
propio `deprecated_patterns.toml`; este cierre la hace explícita y
final:

1. **`transition_outcome_used` queda como regression guard permanente
   (`advisory` by design).** El M9.2 audit (v1.168.40) lo re-categorizó
   `state_machine`: `TransitionOutcome` es el outcome autoritativo del
   gate evaluator, co-currente y NO intercambiable con
   `ExecutionOutcome`. Promoverlo a deny bloquearía maquinaria
   legítima del ciclo. No habrá ciclo de promoción — el lint existe
   para detectar una migración accidental que no debe ocurrir.
   Criterio de reapertura: si una ADR futura redefine el modelo de
   outcome de transición.

2. **`execution_outcome_as_synthesis` sigue `advisory` con criterio de
   expansión orgánico.** Baseline 0 hits verificado de nuevo hoy
   (v1.169.121: 0 hits, sin cambio de superficie). El unblock está
   definido en su propio campo `explanation`: re-auditar cuando llegue
   el 2º proveedor (AX-S6) o cuando la superficie de uso de
   `ExecutionOutcome` crezca; entonces estrechar el regex y evaluar
   deny. No se programa ciclo: sigue el crecimiento del uso.

3. **`asset_unregistered_cli_example`** (mismo clúster advisory, 0
   hits): regex unsafe-by-design; sin cambio.

Con esto el INC tiene owner (rubentxu74), decisión y criterio de
reapertura. La cláusula "sin ciclo programado" del título queda
resuelta: la respuesta es que no hay ciclo que programar — la decisión
es mantener advisory.

## References

- `docs/architecture/lints/deprecated_patterns.toml` (definición de ambos lints)
- `debt-findings.json` del ciclo conformance-closeout-2026-09-13
  (`~/.local/share/sddk/projects/p-63676b11dc0ef88f/cycle-artifacts/`)
- ADR-0047 (debt lifecycle), `docs/debt/SEVERITY.md`, `docs/debt/PRIORITY.md`
- Re-auditoría WU-C2: commit 6bbc104

> Filled by `sddk-archive` (cycle-8+); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.
