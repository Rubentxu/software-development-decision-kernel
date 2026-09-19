# SDDK — Adaptive Inputs & Workflows (AIW)

**Estado:** paquete de propuesta mergeable; NO es una ADR aceptada, un receipt, una certificación de UAT ni un cambio aplicado al repositorio. **Base contrastada:** `Rubentxu/software-development-decision-kernel` rama `main`, commit `da3751021d98092e85a434e2598d8c85e04ff6a3`, 2026-09-19. **Destino sugerido:** `docs/proposals/2026-09-19-adaptive-inputs-workflows/`. **Idioma:** castellano de España.

## Tesis ejecutiva

Hacer que SDDK obtenga primero los hechos por mecanismos deterministas y proveedores especializados, los transforme en evidencia tipada con alcance y procedencia, y los entregue a sus consumidores existentes. El Orchestrator conserva la dirección; el Secretary prepara **propuestas** de próximos trabajos sobre Planning + Knowledge + Verify + runtime; los agentes diseñan y ejecutan trabajo especializado, pero no convierten su narración en evidencia factual. Cuando el resultado exige un cambio de plan, se construye una revisión del Workflow IR que pasa por el compilador, validador y autoridad existentes. Sin nuevo modelo canónico de Agenda, WorkAssignment, WorkHandoff, DecisionGraph ni BehaviourTree por anticipado.

## Autoridad e integración — leer antes de aplicar

1. `docs/architecture/README.md` es la entrada normativa del repo. El roadmap ejecutable vigente está en `docs/architecture/a5/A5-CURRENT-ROADMAP.md`, junto a `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`. NO reemplazarlo automáticamente ni reinterpretar M0–M9 históricos como trabajo vivo.
2. A5-C se declara certificado en el roadmap de referencia. **A6 CogniCode / STATIC_ENHANCED** sigue reservado/pendiente como capacidad productiva; el spike CC-S0 ya dejó código y ADR-0137 aceptada. Distinguir CC-S0 (seam Fake/Null) de proveedor real + evidencia consumida por Verify. A7 (Chronos), A8 (integración), J2..J6 (JCode) y R11 (evaluación de separación de crates) conservan sus carriles.
3. Este paquete **consolida y propone sustituir como guía de diseño futura**, tras una decisión explícita de adopción, los seis documentos `docs/proposals/2026-09-19-*` enumerados en [integration/LEGACY-DISPOSITION.md](integration/LEGACY-DISPOSITION.md). No borra documentos ni cambia frontmatter, estado o autoridad de los históricos.
4. [integration/ROADMAP-OVERLAY.md](integration/ROADMAP-OVERLAY.md) contiene cambios **propuestos**, no un roadmap independiente. [integration/MERGE-PLAN.md](integration/MERGE-PLAN.md) prescribe la integración segura y los gates antes de promocionar una ADR o abrir un ciclo.

## Navegación

| Documento | Función |
|---|---|
| [01-CURRENT-STATE.md](01-CURRENT-STATE.md) | Estado real: documentación, código y adopción; huecos y límites comprobados. |
| [02-DECISIONS-AND-SCOPE.md](02-DECISIONS-AND-SCOPE.md) | Decisiones consolidadas, descartes y criterio anti-proliferación del dominio. |
| [architecture/01-OWNERSHIP-AND-FLOWS.md](architecture/01-OWNERSHIP-AND-FLOWS.md) | Arquitectura hexagonal, productores/consumidores, autoridad, proceso paralelo. |
| [architecture/02-INPUTS-AND-EXECUTION.md](architecture/02-INPUTS-AND-EXECUTION.md) | Mapa auto/directo/delegado y protocolo de captura de CLI externas. |
| [architecture/03-ADAPTIVE-ORCHESTRATION.md](architecture/03-ADAPTIVE-ORCHESTRATION.md) | Orchestrator–Secretary; handoff y expansión dinámica sin runtime paralelo. |
| [architecture/04-BOUNDARIES-AND-FUTURE-CLIS.md](architecture/04-BOUNDARIES-AND-FUTURE-CLIS.md) | JCode/CogniCode/Chronos; extracción futura de CLI basada en evidencia. |
| [adrs/README.md](adrs/README.md) | ADR candidatas y alternativas rechazadas. |
| [specs/README.md](specs/README.md) | Requisitos RFC 2119 y contratos UAT por capacidad. |
| [roadmap/MILESTONES.md](roadmap/MILESTONES.md) | Hitos acotados enlazados a A6/A7/A8/J2/R9/R11, dependencias y criterios de salida. |
| [uat/UAT-MATRIX.md](uat/UAT-MATRIX.md) | Pruebas positivas y negativas, condiciones de falsificación y recibos. |
| [uat/EXECUTION-PLAYBOOK.md](uat/EXECUTION-PLAYBOOK.md) | Preparación de fixtures, ejecución y clasificación de evidencias reales. |
| [integration/ROADMAP-OVERLAY.md](integration/ROADMAP-OVERLAY.md) | Qué actualizar en el roadmap canónico, sin reescribir ciclos cerrados. |
| [integration/MERGE-PLAN.md](integration/MERGE-PLAN.md) | Aplicación, conflict resolution y estrategia de reemplazo controlado. |
| [integration/LEGACY-DISPOSITION.md](integration/LEGACY-DISPOSITION.md) | Disposición explícita de los seis documentos origen y propuestas anteriores. |
| [examples/SCENARIO.md](examples/SCENARIO.md) | Ejemplo end-to-end con dos tareas, prueba estructurada, handoff y expansión. |
| [METRICS-AND-SPIKES.md](METRICS-AND-SPIKES.md) | Métricas, experimentos, TypeSafe/Jev y condiciones de descarte. |

## Definición de «implementado»

`Documentado` ≠ `compilado` ≠ `expuesto por CLI/API` ≠ `invocado en el workflow real` ≠ `evidencia verificada tras reinicio` ≠ `release aceptada`. Cada hito deberá mostrar explícitamente qué escalón alcanza con UAT fresco. Un Fake nunca cuenta como productor real. No se han ejecutado pruebas del workspace para redactar este paquete.
