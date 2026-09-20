# SCOPE-CONTRACT — a8-s2-counterfactual-planning (AC13, ADR-0116)

## Goal

UAT del planning contrafactual sobre deltas efímeros (AC13): evaluar
candidatos de refactor sobre clones de sandbox sin tocar la base.

| Req | Test | Estado |
|---|---|---|
| CF-1 aislamiento: la base no muta al evaluar el delta | t_cf1 | ✅ |
| CF-2 delta violador → detectado con evidencia del guard | t_cf2 | ✅ |
| CF-3 delta compatible → viable (`detected=false` no es error) | t_cf3 | ✅ |
| CF-4 determinismo: misma base+delta → idéntico resultado | t_cf4 | ✅ |

## Notas

- Reutiliza `run_mutation_probe` (AC6) como motor de evaluación
  contrafactual: el sandbox interno ya es un clon efímero
  (`let mut candidate = sandbox.clone()`), lo que AC13 formaliza.
- La superficie de "candidate graph deltas" (grafo semántico, no
  archivos) queda como refinamiento posterior; el invariante
  (evaluación pura, aislada, con evidencia) queda pinado.

## Evidence

- `cargo test -p sddk-engine --test a8_s2_counterfactual_planning` → 4 PASS / 0 FAIL.
- Test-only; sin cambios en src.
