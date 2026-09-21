# AIW-ADR-02 — Agenda y árboles como vistas sobre contratos existentes

**Estado: Proposed · Ámbito:** Planning, Secretary, Decision, Execution, Context.

## Contexto

La metáfora del jefe y sus secretarios tiende a generar `Agenda`, `WorkAssignment`, `DecisionGraph`, `BehaviourTree`, `WorkHandoff`, etc. El repositorio ya posee WorkItem, dependencia, DecisionRecord, IR/revisión, attempts, evidencia y ContextCapsule. Crear copias durables crea dos autoridades e inconsistencias; la proyección `project_next` actual **no** es un scheduler multiobjetivo y puede fallar con varios Active.

## Decisión propuesta

- «Agenda» = consulta/proyección **read-only y explicable** sobre estado Planning identificado, claim/evidence gaps, ejecución y decisiones. El Secretary propone; no persiste una agenda autoritativa ni ejecuta comandos por esa vista.
- «Decision Tree/Graph» = vista de relaciones existentes (incluyendo alternativas de Decision Memory/Decision Lab cuando estén disponibles). No crear un segundo grafo dueño de decisiones.
- «Behaviour Tree» = patrón declarativo sobre operadores realmente implementados y reglas reactivas existentes. No implementar un segundo scheduler ni confundir `Choice` guard con Selector por fallback.
- Handoff = composición de `AgentContributionEnvelope` + outputs/receipts de attempts + `ContextCapsule`; mapear primero arch-spec-007 a los tipos Rust. No crear nuevas clases por coincidencia de nombre.
- Ningún atributo genérico «metadata: JSON» debe esconder invariantes reales. Si una relación indispensable no cabe en los contratos actuales, abrir un UAT que demuestre el hueco y una decisión de diseño mínima (ADR-03).

## Alternativas rechazadas

Una BD de decisiones independiente, agenda Markdown bidireccional, WorkItem automático por finding, crear tipos idénticos al protocolo histórico sin mapeo. La consulta Planning candidata jamás otorga ejecución admitida ni anula Authority.

## UAT

Agenda reconstruida dos veces sobre misma base da mismos campos semánticos; input stale/missing/multiple Active/dep missing nunca reporta READY falso; decisión y handoff guardan refs tras reinicio sin transcript; ningún nuevo owner persistente de planificación. Compatibilidad JSON/texto desde misma proyección.
