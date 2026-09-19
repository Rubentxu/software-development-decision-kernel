# AIW-SPEC-03 — Generación y revisión de workflow sin segundo runtime

**Estado: Proposed.**

- **AIW-REQ-W01 MUST** mantener workflows SDD estáticos ejecutables; dinámica es opt-in por plantilla/permisos de expansión y caso de uso. Ambos compilan al IR canónico y usan el mismo runtime.
- **W02 MUST** recibir trigger real (observación+claim gap o blocker) con revision/basis y consumidor concreto. Evento duplicado se deduplica; ausencia de trigger no autoriza expansión por recomendación narrativa vaga.
- **W03 MUST** separar `propose` Secretary, elección Orchestrator, validación Authority, compiler/validator y commit durable de nueva revisión. Secretary MUST NOT modificar grafo ni efectos por su propia propuesta.
- **W04 MUST** comprobar capacidad negociada, side effects/permiso, schema/guards, budget, DAG, worktree conflicts, concurrency, graph depth, gates, idempotencia y parent revision antes de commit.
- **W05 MUST** conservar resultados/nodos/receipts completos al añadir rama; replay/reboot MUST NOT repetir efecto completado ni llamar a un proveedor cognitivo para reconstruir el mismo plan aceptado.
- **W06 MUST** elegir primer caso expresable por `Task/Sequence/Choice/Map/Parallel` **solo tras caracterización de operator.rs**. `Join/Race/Loop/Gate/Wait/SubWorkflow/Compensate` MUST NOT declararse disponibles por su presencia en `workflow_ir.rs`.
- **W07 MUST** diferenciar guarded `Choice` de selector con fallback-on-failure; Retry sobre side effects exige idempotencia o compensación caracterizada.
- **W08 SHOULD** hacer crecer patrones BT declarativos dentro del IR existente solo cuando UAT de comportamiento real no se pueda satisfacer con una composición más simple. No nuevo scheduler ni tablas BT.
- **W09 MUST** ante budget agotado, deny, stale basis, incompatible provider o check faltante, conservar estado anterior, registrar razón y escalar/abstener sin falso PASS.
