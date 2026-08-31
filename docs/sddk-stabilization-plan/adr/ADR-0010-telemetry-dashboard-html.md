# ADR-0010 — Dashboard HTML autocontenido de telemetría

**Estado:** aceptada
**Fecha:** 2026-08-07
**Milestone:** CP-2026-08 — Control plane local de telemetría

## Contexto

El control plane (ADR-0009) centraliza la telemetría en SQLite, pero sin una capa de presentación los datos siguen siendo solo consultables por CLI. El framework ya tiene el patrón de exportación HTML autocontenido (`export_html` en `crates/sddk-vault/src/export.rs` genera HTML standalone sin CDN), y el PRD exige "informes HTML sin dependencias CDN" como métrica de éxito.

El usuario necesita un dashboard que presente datos útiles — los mismos que hoy están vacíos en el JSONL: costos, teleological coherence, bottleneck, first-pass rate, lead time, distribución de paths/verdicts — agregados cross-proyecto.

## Decisión

Añadir **`sddk telemetry dashboard`**: genera un **HTML estático autocontenido** (CSS + JS inline, cero CDN, cero red) a partir del store central (ADR-0009), siguiendo el patrón `export_html` existente del vault.

Características:

1. **KPIs cross-proyecto**: first-pass rate, lead time medio, costos totales/medios, sample size, bottleneck global.
2. **Desglose por proyecto**: tabla de ciclos por proyecto con verdict, path, merged, tag, lead time, costos.
3. **Tendencias 7d vs 30d**: first-pass rate y lead time comparados por ventana.
4. **Distribuciones**: paths (a-full/a-lite/a-min/b-direct) y verdicts (PASS/PW/FAIL).
5. **Bottleneck por proyecto**: fase con mayor duración media.
6. **Señales F3**: path_bias, recommended_lens/skip/deepen del tuning actual.
7. **Datasets embebidos como JSON** en el HTML: el dashboard se renderiza con JS local (sin fetch, sin red).
8. **Salida**: `--output <path>` (default `dashboard.html` en el directorio del control plane) e impresión de la ruta generada.

## Consecuencias positivas

- Presentación inmediata y portable: se abre con cualquier navegador, se comparte por fichero.
- Sin servidor, sin deploy, sin dependencias de runtime: encaja con local-first.
- Reutiliza el patrón probado `export_html` del vault (misma estrategia de datos embebidos).
- El dashboard solo muestra datos reales: los gaps de datos (costos, coherence) se cierran antes (ADR-0009), no se maquillan en el HTML.

## Consecuencias negativas

- HTML estático: sin interactividad avanzada (filtros, drill-down) sin regenerar o añadir JS local.
- El HTML embebe los datos del momento de generación; no es un live view. Se mitiga con la regeneración implícita tras cada ciclo (mismo mecanismo que el ingest del ADR-0009).

## Alternativas rechazadas

### Servidor web local (e.g. `sddk telemetry serve`)

Añade un proceso vivo, puertos y ciclo de vida; el ADR-0009 ya evita servidores. Revisar si el static no cubre la necesidad.

### Dashboard en el artifact-registry MCP

Excluido: el milestone prohíbe componente MCP; además acoplaría presentación a un store de otro dominio.

### Gráficas por CLI (texto/ASCII)

Útil para consola (`analytics report` ya existe) pero no satisface la presentación visual de datos útiles.

## Criterios de cumplimiento

- `sddk telemetry dashboard` genera un HTML autocontenido que se abre sin red (verificado con `file://` y con inspección de que no hay URLs externas ni CDN).
- El HTML contiene datasets JSON embebidos con los agregados del store central.
- Se regenera idéntico (determinista) para el mismo estado del store.
- Documentado en el README de telemetría del control plane.

## Referencias

- ADR-0009 (control plane local, store central SQLite).
- `crates/sddk-vault/src/export.rs` (patrón `export_html`).
- PRD §11 métricas de éxito: "El informe HTML se genera sin dependencias CDN".
- Spec: `docs/control-plane/SPEC.md`.
