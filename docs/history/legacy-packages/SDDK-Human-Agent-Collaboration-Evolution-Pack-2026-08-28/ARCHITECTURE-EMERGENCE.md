# Arquitectura emergente

La intención es evitar un gran diseño especulativo. Cada decisión futura tiene un **trigger medible**.

## AE-01 — ¿Nuevo crate `sddk-presentation`?

**Default:** no.  
**Spike:** después de HX2.  
**Crear si:** existen >= 3 adapters reales que duplican render logic o aparecen dependencias cíclicas entre CLI/engine/domain.  
**No crear si:** módulos internos mantienen boundaries claros.

## AE-02 — Persistencia de preferencias

**Default:** fichero XDG versionado (`interaction-profile.json`) con write-atómico.  
**Evaluar SQLite si:** >10k observaciones/perfil, queries longitudinales complejas o necesidad cross-project.  
**No usar control-plane.sqlite como authority:** sólo proyección/analytics.

## AE-03 — Engram

**Default:** adapter opcional.  
**Promoción a supported adapter si:** dogfood demuestra que búsquedas cross-session aportan valor y puede conservar provenance/confidence.  
**Nunca:** authority de ciclo o facts críticos.

## AE-04 — Persistencia de InteractionEvents

**Default:** derivar de ledger/artifacts y persistir sólo telemetría mínima.  
**Crear journal específico si:** se necesitan eventos humanos que no pueden reconstruirse y su ausencia rompe UX/auditoría.

## AE-05 — HTML/TUI

**Default:** Markdown/chat + CLI text/json.  
**HTML:** reutilizar control plane cuando HX2 esté estable.  
**TUI:** sólo con evidencia de uso frecuente y tareas de navegación temporal.

## AE-06 — Modelo de personalidad

**Default:** rules/config deterministic + LLM surface renderer opcional.  
**No hacer:** LLM libre reinterpretando facts.  
**Evaluar template compiler:** si golden tests muestran divergencia entre adapters.

## AE-07 — Automatic preference learning

**Default:** promoción conservadora por repetición + confidence.  
**Automatizar más si:** false-positive correction rate < 5% durante >= 30 ciclos dogfood.  
**Fail-safe:** usuario puede inspect/edit/forget/pin.

## AE-08 — Friction adaptation

**Default:** F3 emite recomendaciones, no muta políticas.  
**Auto-tuning permitido sólo para:** verbosity, timing de summaries y formato no crítico.  
**Nunca auto-tune:** security gates, approval thresholds, evidence requirements.

## AE-09 — `/why` implementation

**Default:** explicación basada en Decision/Reframe/Assumption records + evidence links.  
**No:** chain-of-thought.  
**Spike:** medir si el material existente cubre >= 90% de preguntas; si no, añadir `rationale_summary` estructurado a phase outputs.

## AE-10 — UAT shared human protocol

HX3 debe ofrecer un `HumanDecisionPort` reutilizable por `sddk-pack-uat`.  
Si el Guided Runner necesita campos nuevos, extender schema de forma versionada; no bifurcar el modelo.

## Architecture Fitness Functions

- no dependency from domain -> CLI/render;
- renderer cannot invoke lifecycle commands;
- persona transform preserves normalized facts;
- current-run view reconstruction is deterministic;
- every persisted preference has provenance;
- every required human decision has receipt;
- schema migrations are backward compatible.
