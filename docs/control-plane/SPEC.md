# Spec — Control plane local de telemetría (milestone CP-2026-08)

**Estado:** draft
**Fecha:** 2026-08-07
**ADR:** [ADR-0009](../sddk-stabilization-plan/adr/ADR-0009-telemetry-control-plane.md), [ADR-0010](../sddk-stabilization-plan/adr/ADR-0010-telemetry-dashboard-html.md)
**Alcance:** sin componente MCP. Solo CLI + SQLite central + dashboard HTML estático.

---

## 1. Objetivo

Agregar en un único SQLite local la telemetría de ciclos de **todos** los proyectos adoptados en el host, exponerla por CLI (ingest/aggregate/status) y presentarla en un dashboard HTML autocontenido. Cerrar antes los gaps de datos (costos, coherence, context quality) para que el análisis tenga señal.

## 2. No objetivos

- Multi-host / push remoto (disparador de reevaluación en ADR-0009).
- Componente MCP server (excluido explícitamente).
- Migración de datos del artifact-registry (solo enlace por `change_name`, no integración).
- Backend analítico masivo (LadybugDB, ADR-0008): no se reactiva.

## 3. Layout de almacenamiento

```
~/.local/share/sddk/control-plane/
├── control-plane.sqlite   # store central (reconstruible)
└── dashboard.html         # salida por defecto de `sddk telemetry dashboard`
```

El directorio se crea bajo el XDG data root (mismo árbol que `projects/`). El store es una **proyección derivada**: se borra y reconstruye con `sddk telemetry ingest`.

## 4. Esquema SQLite (v1)

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
    project_id   TEXT PRIMARY KEY,          -- p-<hex>
    display_name TEXT NOT NULL,
    scope        TEXT NOT NULL DEFAULT '.',
    remote_url   TEXT,
    first_seen   TEXT NOT NULL,             -- RFC3339
    last_seen    TEXT NOT NULL              -- RFC3339
);

CREATE TABLE IF NOT EXISTS cycles (
    cycle_id                  TEXT PRIMARY KEY,  -- <project_id>/<slug>
    project_id                TEXT NOT NULL REFERENCES projects(project_id),
    path                      TEXT NOT NULL DEFAULT 'unknown',  -- a-full|a-lite|a-min|b-direct
    context_quality           TEXT NOT NULL DEFAULT 'C2',
    phase_durations_sec       TEXT NOT NULL DEFAULT '{}',  -- JSON map phase->sec
    coherence_scores          TEXT NOT NULL DEFAULT '[]',  -- JSON array
    correction_cycles         INTEGER NOT NULL DEFAULT 0,
    tokens_used               INTEGER NOT NULL DEFAULT 0,
    cost_estimate_usd         REAL NOT NULL DEFAULT 0.0,
    costs                     TEXT NOT NULL DEFAULT '{}',   -- JSON map L1-L6 -> usd
    first_pass_success        INTEGER NOT NULL DEFAULT 0,
    verify_verdict            TEXT NOT NULL DEFAULT 'UNKNOWN',  -- PASS|PW|FAIL|UNKNOWN
    merged_to_main            INTEGER NOT NULL DEFAULT 0,
    tag_version               TEXT,
    lead_time_hours           REAL,
    teleological_coherence_pct REAL,
    recorded_at               TEXT NOT NULL,   -- RFC3339
    UNIQUE (project_id, cycle_id)
);

CREATE TABLE IF NOT EXISTS aggregates (
    window_days   INTEGER NOT NULL,          -- 7 | 30
    computed_at   TEXT NOT NULL,             -- RFC3339
    payload_json  TEXT NOT NULL              -- MetricsAggregate serializado
);

CREATE INDEX IF NOT EXISTS idx_cycles_project ON cycles(project_id);
CREATE INDEX IF NOT EXISTS idx_cycles_recorded ON cycles(recorded_at);
```

Regla de integridad: **un `cycle_id` solo se ingesta una vez**; re-ingest actualiza `last_seen` del proyecto y los campos del ciclo (upsert por `cycle_id`).

## 5. Comandos CLI

### `sddk telemetry ingest`

Escanea `~/.local/share/sddk/projects/*/` y por cada proyecto con `workspaces/*/adoption.json`:

1. Registra/actualiza la fila `projects` (display_name, scope, remote_url, first/last_seen).
2. Lee `metrics/metrics.jsonl` y hace upsert de cada registro en `cycles`.
3. Si un ciclo no tiene registro métrico pero el ledger (`~/.local/state/sddk/projects/<id>/ledger.sqlite`) contiene eventos para él, deriva un registro pobre con `derive_from_events` (misma lógica que `metrics backfill`).
4. Reporta: proyectos vistos, ciclos ingesados, ciclos derivados, duplicados omitidos.

Flags: `--dry-run` (muestra el plan sin escribir), `--format json|text`.

Idempotencia: ejecutar dos veces produce el mismo estado final (no duplica filas).

### `sddk telemetry aggregate --window 7d|30d`

1. Lee `cycles` del store central.
2. Computa `MetricsAggregate` cross-proyecto (reutiliza `compute_aggregate` de `metrics.rs`, sin ventana por archivo).
3. Persiste en `aggregates` y escribe `aggregate.json` junto al store.
4. Emite también el bloque F3 (`tuning_from_aggregate`) → `tuning.md` del control plane.

El aggregate cross-proyecto incluye: `sample_size`, `first_pass_success_rate`, `median_lead_time_hours`, `median_cost_usd`, `top_bottleneck_phase`, `path_distribution`, `verdict_distribution` — todo agregado sobre TODOS los proyectos.

### `sddk telemetry status`

Tabla resumen: proyecto, nº ciclos, nº con costos, nº con coherence, fecha último ingest. Sirve para detectar gaps de datos visualmente (¿cuántos ciclos tienen `cost_estimate_usd = 0`?).

### `sddk telemetry dashboard --output <path>`

Genera HTML autocontenido (ADR-0010):

- KPIs: sample, first-pass rate, lead time medio, costos medios/totales, bottleneck global.
- Tabla de ciclos (todos los proyectos), ordenada por `recorded_at` desc.
- Comparativa 7d vs 30d.
- Distribuciones paths/verdicts.
- Bottleneck por proyecto.
- Señales F3 del tuning vigente.
- Datasets embebidos como JSON inline; **cero URLs externas, cero CDN, cero fetch**.
- Determinista: mismo store → mismo HTML.

## 6. Gaps de datos (prerrequisito)

Antes de que el dashboard sea útil, se cierran estos gaps en la **captura automática** (`capture_cycle_metrics`):

| Gap | Hoy | Target |
|-----|-----|--------|
| Costos/tokens | `tokens_used: 0`, `cost_estimate_usd: 0.0` siempre | Estimar por modelo: `estimate_cost(tokens, model)` con rates por modelo; persistir `tokens_used` y `costs` (L1-L6) si el ledger/manifiesto los expone |
| Teleological coherence | `teleological_coherence_pct: null` siempre | Calcular pct desde artifacts del ciclo (spec ↔ resultado); si no hay artifacts suficientes, dejar `null` pero **contabilizarlo** en `telemetry status` como gap |
| Context quality | `C2` por defecto sin evidencia | Leer del `context.json` real (ya soportado por `metrics record --set-context`) y registrarlo en el ciclo |
| Verdict real | derivado solo de estado `RELEASED`/`REMEDIATING` | Completar con el receipt de verify cuando exista |

Criterio de cierre: tras implementar, `sddk telemetry status` muestra >0 ciclos con costos estimados y el dashboard no muestra columnas vacías salvo que el gap esté documentado.

## 7. Integración con el loop self-research

Sin MCP: los agentes `analytics-researcher`/`analytics-judge`/`analytics-reporter` consumen el **research packet cross-proyecto** emitido por `sddk analytics research` (ya existe) pero alimentado desde el store central cuando está presente:

- Fuentes del researcher: `control-plane.sqlite` (aggregates + cycles) + `ledger events` por proyecto.
- El `research packet` pasa a incluir `projects: [...]` con resumen por proyecto.

## 8. Criterios de aceptación

- [ ] `telemetry ingest` registra los proyectos adoptados del host y todos sus ciclos sin duplicados.
- [ ] Reconstruible: `rm control-plane.sqlite && ingest` produce el mismo estado.
- [ ] Idempotente: doble ingest no duplica.
- [ ] `telemetry aggregate --window 30d` > sample que `--window 7d` cuando hay ciclos de más de 7 días.
- [ ] `telemetry dashboard` genera HTML sin URLs externas (grep `https?://` y `src=` externos → vacío) que abre vía `file://`.
- [ ] Determinista: dos ejecuciones con el mismo store → mismo hash del HTML.
- [ ] Gaps cerrados: `telemetry status` evidencia costos y coherence poblados o gap documentado.
- [ ] 0 regresiones: `cargo test` + `act -j required` verdes.

## 9. Fases de implementación

1. **G1 — Gaps de datos** en `metrics.rs` (costos, coherence, context quality, verdict).
2. **G2 — Store + ingest** (`sddk-telemetry` crate o módulo CLI + schema v1 + upsert + derive).
3. **G3 — Aggregate cross-proyecto** (reuso de `compute_aggregate` sobre el store).
4. **G4 — Dashboard HTML** (patrón `export_html` del vault + datasets embebidos).
5. **G5 — Research packet cross-proyecto** + actualización de agentes (sin MCP).
6. **G6 — Docs y CI**: README del control plane, roadmap cerrado, tests.

## 10. Tests

- Unit: upsert idempotente, derive_from_events, aggregate cross-proyecto (fixtures de 2+ proyectos), determinismo del HTML.
- CLI: ingest con `--dry-run`, status con gaps, dashboard sin red (parsear HTML y verificar cero URLs externas).
- Reconstrucción: delete + ingest == mismo contenido (hash de filas).

## 11. Referencias

- `crates/sddk-cli/src/metrics.rs` — `capture_cycle_metrics`, `derive_from_events`, `compute_aggregate`, `tuning_from_aggregate`.
- `crates/sddk-cli/src/analytics.rs` — `ResearchPacket`, `run_analytics_research`.
- `crates/sddk-vault/src/export.rs` — patrón `export_html` autocontenido.
- `docs/sddk-stabilization-plan/adr/ADR-0009-telemetry-control-plane.md`, `ADR-0010-telemetry-dashboard-html.md`.
- ROADMAP milestone CP-2026-08; BACKLOG épica E11 (SDDK-1101..1106).
