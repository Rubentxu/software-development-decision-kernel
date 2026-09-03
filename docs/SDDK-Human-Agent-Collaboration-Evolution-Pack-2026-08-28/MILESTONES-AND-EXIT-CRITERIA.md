# Milestones y criterios de salida

## M-HX0 — State Truth

- [ ] Source-of-truth matrix única.
- [ ] `status-query.md` alineado.
- [ ] CurrentRunView reconstruible desde fixtures.
- [ ] 0 fallos de authority contract test.
- [ ] cold-start ambiguity devuelve blocked explícito, nunca estado inventado.

## M-HX1 — Interaction Contracts

- [ ] schemas draft-07/2020-12 validados según convención repo.
- [ ] 10 event types mínimos.
- [ ] backward/forward version strategy documentada.
- [ ] 100% domain tests.

## M-HX2 — Never Lost

- [ ] phase started/completed/blocked report.
- [ ] resume report.
- [ ] breadcrumb estable.
- [ ] UAT comprehension >= 90%.
- [ ] report <= 140 palabras por fase en novice salvo blocker/decision.
- [ ] resume <= 150 palabras.
- [ ] no lifecycle calls desde renderer.

## M-HX3 — Human Authority

- [ ] risk policy implementada.
- [ ] required decisions generan receipt.
- [ ] 100% irreversible/high-risk acciones requieren autoridad.
- [ ] <= 1 approval innecesaria/ciclo.
- [ ] `/why` no expone chain-of-thought; usa rationale/evidence.
- [ ] Reframe material siempre visible.

## M-HX4 — Persona

- [ ] novice/standard/expert/audit.
- [ ] wisecracking_robot.
- [ ] safety tone suppressor.
- [ ] 100% semantic parity en golden dataset.
- [ ] personality no altera command routing/gates.

## M-HX5 — Memory

- [ ] profile XDG atomic.
- [ ] candidate -> learned promotion.
- [ ] edit/forget/pin/export.
- [ ] provenance en 100%.
- [ ] correction rate < 10% tras 20 ciclos; target <5% para auto-learning avanzado.

## M-HX6 — Friction/F3

- [ ] taxonomy.
- [ ] metrics ingest.
- [ ] dashboard.
- [ ] F3 recommendations.
- [ ] no auto-mutation de safety policy.
- [ ] `where_am_i_queries_per_cycle < 0.2` en ventana dogfood >=20 ciclos.

## M-HX7 — Release

- [ ] dogfood >= 20 ciclos.
- [ ] >= 5 resume-from-cold-start.
- [ ] >= 5 blocker/recovery.
- [ ] >= 5 decision-required.
- [ ] UAT >= 90% global.
- [ ] 0 P0 abiertos.
- [ ] docs/inventory/manifest actualizados.
