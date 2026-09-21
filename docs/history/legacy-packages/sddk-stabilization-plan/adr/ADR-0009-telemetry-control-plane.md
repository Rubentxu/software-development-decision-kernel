# ADR-0009 — Control plane local de telemetría (SQLite central + ingest)

**Estado:** aceptada
**Fecha:** 2026-08-07
**Milestone:** CP-2026-08 — Control plane local de telemetría

## Contexto

El framework genera telemetría real de ciclos (`metrics.jsonl`, `aggregate.json`, `tuning.md` por proyecto) pero vive fragmentada:

- Un store por proyecto en `~/.local/share/sddk/projects/<project_id>/metrics/`.
- Los agregados (`sddk analytics report/trends/research`) requieren `--root/--scope` y solo ven un proyecto.
- No existe una vista cross-proyecto: first-pass rate, lead time, bottleneck, costos y señales F3 de TODOS los proyectos en un solo lugar.
- Los agentes self-research (`analytics-researcher` → `analytics-judge` → `analytics-reporter`) consumen archivos por proyecto; sin agregación central su análisis es ciego a la flota.
- Gaps de datos conocidos: `cost_estimate_usd` siempre 0.0, `tokens_used` siempre 0, `teleological_coherence_pct` siempre `null`, `context_quality` fijo en C2 por defecto.

No existe control plane. La infraestructura MCP centralizada disponible (artifact-registry, agents-workflows) no recibe telemetría de ciclos y no se integra en este ADR.

## Decisión

Crear un **control plane local-first** sin servidor ni componente MCP:

1. **SQLite central reconstruible**: `~/.local/share/sddk/control-plane/control-plane.sqlite` agregando las métricas de todos los proyectos adoptados del host. Es una proyección derivada — se puede borrar y reconstruir desde los JSONL locales, siguiendo el precedente del vault-index reconstruible (ADR-0002).
2. **`sddk telemetry ingest`**: escanea `~/.local/share/sddk/projects/*/metrics/metrics.jsonl` + `adoption.json` + `ledger.sqlite` y puebla el store central de forma idempotente (por `cycle_id`).
3. **`sddk telemetry aggregate --window 7d|30d`**: computa agregados cross-proyecto (first-pass rate, lead time medio, bottleneck por proyecto y global, distribución de paths/verdicts, costos, señales F3) y los persiste en el store central.
4. **Cierre de gaps de datos** como prerrequisito del milestone: estimación de costos/tokens en la captura automática, teleological coherence, y context quality real.

## Consecuencias positivas

- Vista unificada de toda la flota de proyectos en una máquina.
- Los agentes self-research consultan un solo store determinista en vez de N JSONL.
- Reconstruible e idempotente: sin estado distribuido que reparar.
- Respeta la filosofía local-first del framework (ADR-0001, modo local-first con act): cero red, cero cloud.
- Trazable: el store central puede enlazarse por `change_name` con el artifact-registry (sin migrar sus datos).

## Consecuencias negativas

- No agrega entre máquinas (multi-host queda fuera del alcance; es un disparador futuro).
- Un directorio más que mantener (`~/.local/share/sddk/control-plane/`).
- El dashboard (ADR-0010) depende de que el ingest se ejecute tras cada ciclo; se mitiga con ingest implícito al cerrar ciclo.

## Alternativas rechazadas

### Extender artifact-registry MCP como control plane

Ya centraliza 329 artifacts en SQLite, pero pertenece a otro dominio (fases de cambio), está poblado por varios proyectos y mezclar telemetría de ciclos lo acoplaría a un componente MCP — excluido explícitamente de este milestone.

### Servidor remoto / push telemetry

Violaría el ADR local-first y añade infraestructura sin demanda medida. Se documenta como disparador de reevaluación.

### Reutilizar solo el CLI analytics por proyecto

No resuelve la vista cross-proyecto ni alimenta el loop self-research con una única fuente.

## Criterios de cumplimiento

- `sddk telemetry ingest` sobre el host actual produce un store con los 3 proyectos adoptados y todos sus ciclos, sin duplicados (`cycle_id` único).
- `sddk telemetry aggregate` produce agregados cross-proyecto con sample = suma de ciclos de todos los proyectos.
- El store central se reconstruye idéntico tras `rm control-plane.sqlite && sddk telemetry ingest`.
- El ingest es idempotente: ejecutado dos veces no duplica filas.
- Los agentes self-research consumen exclusivamente el store central (contracto `research packet` cross-proyecto).

## Disparadores de reevaluación

- Más de una máquina reportando (multi-host → evaluar push/aggregator remoto).
- Más de 50 proyectos adoptados (evaluar particionado o índice FTS del control plane).
- Consulta cross-proyecto > 1s sostenida (evaluar proyecciones materializadas).

## Referencias

- ADR-0001 (local-first, runtime determinista), ADR-0002 (proyecciones reconstruibles), ADR-0008 (complejidad analítica aplazada hasta que las métricas la justifiquen).
- `crates/sddk-cli/src/metrics.rs` (captura, agregación, F3), `crates/sddk-cli/src/analytics.rs` (research packet).
- Spec: `docs/control-plane/SPEC.md`.
- Roadmap: milestone CP-2026-08 en `docs/sddk-stabilization-plan/ROADMAP.md`.
