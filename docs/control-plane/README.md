# SDDK Control Plane (local)

Telemetría centralizada de **todos** los proyectos adoptados en el host, sin
servidor ni componente MCP. ADR-0009 (store SQLite local), ADR-0010 (dashboard
HTML autocontenido).

## Layout

```text
$SDDK_DATA_DIR/control-plane/        # ~/.local/share/sddk/control-plane
├── control-plane.sqlite             # store central (projects, cycles, aggregates)
└── dashboard.html                   # salida de `sddk telemetry dashboard`
```

El store es una **proyección derivada**: se puede borrar y reconstruir con
`sddk telemetry ingest`.

## Comandos

| Comando | Qué hace |
|---------|----------|
| `sddk telemetry ingest` | Escanea `projects/*/` (adoption.json + metrics.jsonl + ledger) y puebla el store central. Idempotente y reconstruible. `--dry-run` para plan. |
| `sddk telemetry aggregate --window 7d\|30d` | Agregados cross-proyecto (first-pass, lead time, bottleneck, paths, verdicts, costos) + señales F3 persistidas. |
| `sddk telemetry status` | Tabla por proyecto: nº ciclos, cobertura de costos/coherence, último ingest. Detecta gaps de datos. |
| `sddk telemetry dashboard --output <path>` | Genera HTML autocontenido (sin CDN, sin red) con KPIs, tendencias, distribuciones y señales F3. Determinista. |
| `sddk analytics research --all-projects` | Research packet cross-proyecto (contrato de entrada del loop self-research). Requiere ingest previo. |

## Uso típico

```bash
# 1. Ingerir la telemetría de todos los proyectos adoptados
sddk telemetry ingest

# 2. Ver cobertura y gaps de datos
sddk telemetry status

# 3. Agregados cross-proyecto (7d y 30d)
sddk telemetry aggregate --window 7d
sddk telemetry aggregate --window 30d

# 4. Dashboard HTML autocontenido
sddk telemetry dashboard            # → $SDDK_DATA_DIR/control-plane/dashboard.html

# 5. Research packet para el loop self-research
sddk analytics research --all-projects --root <repo> --scope <scope> --format json
```

## Gaps de datos

`telemetry status` evidencia gaps (costos/coherence en 0). El enriquecimiento
de un ciclo cerrado se hace con `sddk metrics record` (upsert por cycle_id):

```bash
sddk metrics record --root <repo> --scope <scope> --cycle <cycle_id> \
  --tokens 200000 --model mini-m2.7 --coherence 88 --costs '{"L1": 0.4}'
```

## Criterios

- Ingest idempotente: doble ejecución no duplica (`cycle_id` único).
- Reconstruible: `rm control-plane.sqlite && sddk telemetry ingest`.
- Dashboard sin URLs externas (verificado en tests), abrible vía `file://`.
- Determinista: mismo store → mismo HTML.

## Referencias

- [ADR-0009](../sddk-stabilization-plan/adr/ADR-0009-telemetry-control-plane.md)
- [ADR-0010](../sddk-stabilization-plan/adr/ADR-0010-telemetry-dashboard-html.md)
- [SPEC](SPEC.md)
- Roadmap: milestone CP-2026-08.
