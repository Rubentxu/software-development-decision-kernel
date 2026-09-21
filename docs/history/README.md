# Archivo histórico de SDDK — no es backlog activo

**Autoridad de continuación:** [../roadmap/README.md](../roadmap/README.md) → [../roadmap/ROADMAP.md](../roadmap/ROADMAP.md). Este directorio conserva contexto y decisiones antiguas; NO promueve specs/roadmaps/recibos por sí solo.

## Traslado físico en la propuesta 2026-09-21

| Ruta original (queda un stub de compatibilidad) | Contenido trasladado |
| --- | --- |
| `docs/evolutivo-continuidad-sesiones-delegacion-deliberacion.md` | [continuidad, delegación y memoria](legacy-evolutivos-2026/evolutivo-continuidad-sesiones-delegacion-deliberacion.md) |
| `docs/evolutivo-correcciones-flexibilidad.md` | [ideas de flexibilidad diferidas](legacy-evolutivos-2026/evolutivo-correcciones-flexibilidad.md) |
| `docs/evolutivo-workflows-dinamicos-STATUS.md` | [estado de workflows de v1.70.0](legacy-evolutivos-2026/evolutivo-workflows-dinamicos-STATUS.md) |
| `docs/evolutivo-workflows-dinamicos-integracion-roadmap.md` | [dossier antiguo de integración de workflows](legacy-evolutivos-2026/evolutivo-workflows-dinamicos-integracion-roadmap.md) |

Los textos íntegros se conservan en sus rutas nuevas **sin reescritura**. Las rutas originales pasan a ser stubs enlazados para que las referencias existentes no den 404. Los enlaces relativos internos del documento archivado son históricos: no se declaran reparados automáticamente. Antes de citar un enlace histórico, comprobar destino; nunca ejecutar su viejo roadmap como vigente.

## Congelados in situ: NO mover a ciegas

Se conservan en sus rutas originales porque AGENTS, specs, ADRs, recibos o tests pueden referenciarlos con paths estables: `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/`, `docs/sddk-2.0-architecture-consolidation/`, `docs/sddk-complete-evolution-2026-08-23/`, `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/`, `docs/sddk-decision-kernel-architecture/`, `docs/architecture/a5/`, `docs/handoff/`, `docs/research/`, `tests/cycle-artifacts/`. Son HISTÓRICOS o evidencias fechadas, no roadmaps activos.

**Para un traslado posterior:** generar inventario de inbound links (MD, YAML, TOML, shells, tests y manifests), crear mapa old→new, actualizar los consumidores y los generadores, ejecutar lints/contratos y preservar stub para cualquier enlace externo conocido; mover por lote acotado en commit independiente. No mover ni editar evidencia certificada salvo operación de traslado aprobada con trazabilidad de hash/SHA.

## No archivar

[../architecture/README.md](../architecture/README.md) (arquitectura normativa), ADRs/specs aceptados, los gates originales [PRODUCTION-READY-GATE](../SDDK-Production-Readiness-Alignment-2026-09-14/03-PRODUCTION-READY-GATE.md), los receipts UAT, `AGENTS.md`, `docs/RELEASING.md` y el nuevo directorio `docs/roadmap/`. No ocultar deuda real archivando sus incidencias.

## Semántica

`HISTORICAL` = contexto congelado; `SUPERSEDED` = no fuente normativa; `DEFERRED` = intención aún no aprobada/ejecutada; `CERTIFIED` = recibo histórico vinculante **solo para su SHA/perfil**. Una futura reconciliación abre un WorkItem nuevo y enlaza origen, decisión y receipt, no cambia retrospectivamente el pasado.
