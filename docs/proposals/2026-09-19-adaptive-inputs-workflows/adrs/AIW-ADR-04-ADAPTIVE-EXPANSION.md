# AIW-ADR-04 — La replanificación es una propuesta que modifica IR mediante autoridad existente

**Estado: Proposed · Ámbito:** Secretary L0/L1/L2, Orchestrator, compiler, validator, workflow-run.

## Contexto

ADR-037 documenta expansión dinámica, pero la presencia de variantes IR no prueba la ejecución de Join/Race/Loop/Gate/Wait/SubWorkflow/Compensate en `build_operator`. Tampoco se ha probado que Secretary consuma Planning/Knowledge en CLI/skills productivos. Crear runtime BT paralelo duplicaría scheduler, lock, receipts y recuperación.

## Decisión propuesta

- Evento **factual y pertinente** + claim/gap/dependency produce propuesta del Secretary con trigger ref, basis, objetivo, capacidad elegible, budget y delta de plan. No crear work item ni mutar IR en etapa de preparación.
- Orchestrator decide política estratégica; Authority valida permisos de invocación/observación y de efecto; compiler/validator comprueban invariantes, DAG, allowlist, budgets/concurrencia y operator support. Commit de revisión padre→hija atómico con evento/idempotencia.
- Runtime conserva completed nodes/receipts; reevalúa solo ramas nuevas; replay consume revisión aceptada, no vuelve a consultar LLM/Jev ni reejecuta efectos.
- Primer caso **un Task de verificación añadido** desde una evidencia estática real, con `Task/Sequence/Choice` solo si sus semánticas existentes se comprueban. No implementar toda álgebra BT por simetría.

## Alternativas rechazadas

Scheduler privado del Secretary; ejecutar scripts de workflow arbitrarios generados por LLM; recompilar desde cero y resetear trabajos ya completados; `selector=Choice` sin prueba fallback-on-failure; nueva BD de PlanRevision.

## UAT

Disparador duplicado, basis stale, reinicio entre proposal y commit, deny, cap absent, budget agotado, nodo completado, trabajo paralelo conflictivo y cancelación: 0 efectos duplicados y resultado anterior intacto. Cuando falta el operador requerido, el compilador rechaza la expansión explícitamente.
