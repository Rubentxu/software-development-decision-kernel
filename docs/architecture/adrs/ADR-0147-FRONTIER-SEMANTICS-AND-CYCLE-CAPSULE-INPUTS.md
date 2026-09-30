---
status: Accepted
date: 2026-09-30
deciders: ["orchestrator (autonomía delegada, session-46)"]
related:
  - ADR-075-CURRENT-RUN-VIEW-SHAPE
  - ADR-0145-DURABLE-CONTEXT-STORE-ROOT-MODULES
  - ADR-0146-DURABLE-DELTA-STREAM-ROOT-MODULE
  - SPEC-005 (durable context, CTX-003)
  - INC-DEBT-039
  - INC-DEBT-042
---

# ADR-0147 — Frontier semantics without a workflow run, and cycle-level CapsuleInputs

## Context

INC-DEBT-039 bloquea CTX-003 paso 5 ("compilar capsule") desde session-42
con una pregunta declarada "de modelo, no de código": *qué es `frontier`
cuando `node_runs_v1` está vacía*. El operador delegó esta decisión a esta
sesión (2026-09-30: "queda aprobado todos los gates humanos... continua con
el roadmap hasta el final").

El estado observado del ledger real: `node_runs_v1: 0`,
`workflow_run_events_v1: 0`, `decision_records_v1: 0`, 81 work items de los
que solo 2 tienen `cycle_id` real. No existe todavía ruta de producción que
cree workflow runs del substrate (`node_runs_v1` solo se escribe desde el
graph store de ejecución). Mientras tanto, el único `CapsuleInputs` de
producción (`RecoveryCapsuleInputs`) exige un `RunStateView` que ninguna
fuente puede producir, así que `sddk context bootstrap` no puede compilar
capsule y reporta `no_capsule_source` (exit 4, INC-DEBT-042 opción a).

## Decision

**D1 — `frontier` solo se define para un run existente.** La ausencia de
una fila en `node_runs_v1` NO es "frontier vacío"; es "no hay vista".
La invariante de REQ-CurrentRunView-Shape ("frontier vacío iff el run es
terminal o ningún nodo está listo") es una invariante **condicional a la
existencia del run**: sin run no es evaluable, y reportar `frontier=[]`
para un run inexistente fabricaría un hecho (el defecto exacto que la
opción (b) de INC-DEBT-039 eliminó). Un frontier vacío sigue siendo un
hecho legítimo para un run real terminal o sin nodos listos.

**D2 — El bootstrap de contexto compila capsule a nivel CICLO, no a nivel
run.** `sddk context bootstrap` opera sobre identidad/adopción/ciclo
(SPEC-005 CTX-003 pasos 1-3); su capsule de bootstrap es la del ciclo. Un
nuevo `CapsuleInputs` de producción (`CycleLedgerCapsuleInputs`) alimenta
el compilador con hechos reales del ledger: goal del ciclo (del
`CycleManifest`), work items por estado, decisiones aceptadas/rechazadas y
work items bloqueados como `must_read`. Eso satisface el MUST de CTX-003
paso 5 sin necesitar `RunStateView`.

**D3 — La ruta run-level queda como recovery explícito.** Para un
`workflow_run` real del substrate, `RecoveryCapsuleInputs` + un futuro
`RunStateViewInputs` respaldado por ledger es EL camino (falsable con
runs reales cuando existan). No se implementa hoy: sin runs, un adaptador
así sería código muerto probado solo contra fakes, exactamente el tipo de
"verde" que este repo rechaza. INC-DEBT-039 opción (a) queda re-scoped a
"cuando exista el primer run real" con D1 como semántica ya decidida.

## Consequences

- CTX-003 paso 5 deja de estar bloqueado: `context bootstrap` compila
  capsule real del ciclo (o falla honesto si no hay ciclo activo).
- El estado del bootstrap pasa a `complete` solo cuando hay capsule
  compilada/recuperada; `no_capsule_source` sigue siendo el estado de
  "sin ciclo" y sigue saliendo con exit 4 (INC-DEBT-042).
- `RecoveryCapsuleInputs` no cambia; su insumo `RunStateView` queda
  gobernado por D1 (no existe frontier sin run).
- INC-DEBT-039 pasa de "bloquea CTX-003 paso 5" a "pendiente del primer
  run real del substrate" (P2, ya degradada en session-45b/c).
- Frontiers de capsule a nivel ciclo: los work items `ready`/`blocked`
  del ciclo son la representación operativa de "qué sigue"; se mapean a
  `must_read` (blockers) y metadata del capsule, no a `frontier` de
  `RunStateView`.
