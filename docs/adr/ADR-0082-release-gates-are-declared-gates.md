# ADR-0082: Los gates del release son los declarados en `gates:`; los receipts son artifacts

## Estado

Accepted (2026-09-30, ciclo C3k/W4, report agent-secretless S3.1/S3.2)

## Contexto

El prompt de la fase release instruía ejecutar
`sddk cycle evaluate-gate --gate release-receipt` (y en una sección,
`--gate {release-receipt|no-pending-effects}`). En el workflow canónico
(`workflow/workflow.yaml`) `release-receipt` y `merge-receipt` están
declarados como **artifacts** (sección `artifacts:`, requeridos por la
transición `release.complete`), no como gates. Los únicos gates reales
de esa transición son `no-pending-effects` y `release-uat-approved`
(bloque `gates:`). Reproducido en v2.2.37: el comando del prompt falla
con `ENGINE_UNREGISTERED_EVALUATOR`, porque un gate no declarado nunca
recibe evaluadores (el engine registra `sddk.cli` por defecto solo
para `workflow.gates.keys()`).

La alternativa considerada fue registrar `sddk.cli` también sobre los
receipts, tratándolos como gates materializados. Se rechaza: un gate es
una pregunta binaria evaluada con evidencia; un receipt es un artefacto
con productor, consumidor y binding SHA. Duplicarlos como gates rompe
la separación artifact/gate del workflow (ADR-0080, clasificación de
gates) y crearía dos fuentes de verdad para el mismo hecho.

## Decisión

1. **Los gates evaluables son exactamente los del bloque `gates:` del
   workflow manifest.** El engine registra `sddk.cli` por defecto para
   todos ellos; no hay registro ad hoc desde el CLI.
2. **El prompt de release (fase y skill) instruye evaluar solo
   `no-pending-effects` y `release-uat-approved`;** los receipts
   (`release-receipt`, `merge-receipt`) se pasan como `--artifact` en la
   transición, nunca como `--gate`.
3. **El error `UnregisteredEvaluator` incluye un hint** que explica la
   distinción artifact/gate y señala el ejemplo canónico
   (`release-receipt`/`merge-receipt`), para que un agente que siga un
   prompt viejo pueda autocorregirse.
4. Los prompts son **texto derivado del workflow**, no arquitectura
   (AGENTS.md §2.8): si el workflow y un prompt discrepan, gana el
   workflow y el prompt se corrige.

## Consecuencias

- `sddk cycle evaluate-gate --gate release-receipt` sigue fallando
  (correcto), pero con un mensaje accionable.
- El flujo de release del prompt ejecutado al pie de la letra produce
  receipts reales (test: `crates/sddk-cli/tests/release_gate_contract_e2e.rs`).
- Cualquier prompt futuro que nombre un gate debe verificarlo contra el
  bloque `gates:` del manifest; el test de contrato anterior falla si el
  prompt vuelve a instruir artifacts como gates.

## Falsación

- `cargo test -p sddk-cli --test release_gate_contract_e2e` (2 casos).
- `grep -n "gate release-receipt" prompts/sddk/phases/release.md` no
  debe producir instrucciones de evaluate-gate sobre artifacts.
