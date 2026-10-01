---
id: ADR-0148-DYNAMIC-EXPANSION-ROOT-MODULE
status: accepted
proposed_at: 2026-10-01
accepted_at: 2026-10-01
accepted_by_cycle: c3l
date: 2026-10-01
deciders: ["operador (gates pre-aprobados, session-55)", "orchestrator"]
related:
  - ADR-0097-COMMON-REVISION-SUBSTRATE
  - ADR-0145-DURABLE-CONTEXT-STORE-ROOT-MODULES
---

# dynamic_expansion: un módulo root para la vertical de expansión

## Contexto

`AIW-S4` (expansión dinámica por evidencia) se declaraba entregado con un test
que llamaba `Engine::cycle_replan` directamente. La verificación de C3l.3
encontró seis huecos, dos de ellos estructurales:

- `cycle_replan` toma su `event_id` del caller, así que **no deriva identidad
  del trigger**: dos expansiones idénticas son dos revisiones. El test W02
  incluso *fijaba el defecto como contrato*.
- No existe un concepto de **"nodo nuevo"** en ninguna parte, porque la
  ejecución incremental no existe todavía.

Un intento de coser esto como composición de test era imposible: el exit gate
exige identidad estable del trigger, revisión N+1 con lineage y ejecución
incremental, y **ninguna de las tres cosas tenía superficie**.

## Decisión

Nuevo módulo root `crates/sddk-engine/src/dynamic_expansion.rs`.

1. **La identidad del trigger es un fingerprint content-addressed** que viaja en
   el payload del evento canónico `cycle.expansion.applied`. La idempotencia se
   apoya en que el ledger **ya es** la autoridad append-only (AGENTS.md §2.7):
   no se crea una tabla nueva ni un registro paralelo. El guard de replay corre
   **antes de cualquier mutación**, lo que cierra por construcción la divergencia
   entre el evento deduplicado y la proyección reescrita.

2. **El padre de cada revisión es el tip real del ledger**, no un recálculo
   local. Motivo: `WorkflowManifest` no contiene un `WorkflowIR` (es un
   manifiesto de transiciones, no un DAG), así que el substrate de plan-revision
   nunca estuvo anclado al ciclo. Anclar el linaje al ledger lo hace
   reconstruible y resistente a restart.

3. **La selección incremental se calcula contra el plan base que declara el
   propio trigger** (`base_ir` → `proposed_ir`).

## Límite declarado

`executed_node_ids` es un ledger de **selección y contabilidad** de nodos
despachados, **no** evaluación de operadores. Evaluar operadores es
DW-RUNTIME-003/004/005, explícitamente fuera de scope del propio
`execution_graph_compiler`, que es *compile-only* por diseño declarado.

Por tanto este ADR **no** habilita `AIW-S4` como VERIFIED. La frontera de
certificación es la de C3l.0: la vertical existe y es falsificable, la
ejecución de operadores no.

## Alternativas descartadas

- **Cosido como test de composición** — imposible: la superficie no existía.
- **Tabla de identidades de trigger aparte** — crea una segunda autoridad para
  un dato que el ledger ya puedeAssociar.
- **`PlanRevisionLineageV1` para el linaje** — su campo `revisions` es privado y
  no hay constructor desde un vector; `derive` además exige el IR completo. Se
  usa `PlanRevisionV1::new` con el padre del ledger, que da lo mismo y es
  reconstruible.

## Consecuencias

- Un test de contrato (`crates/sddk-cli/tests/context_fitness.rs::
  no_new_root_level_context_module_without_adr`) exige que todo módulo root
  nuevo en `sddk-engine/src` esté nombrado en un ADR. Este ADR lo satisface.
- La idempotencia por contenido hace que dos triggers con distinto
  `expected_base_revision` pero mismo contenido compartan fingerprint: es lo
  correcto, porque la base es un pin de concurrencia, no parte de la identidad
  del request.
