---
id: INC-AUDIT-S14-NO-STRUCTURED-LOGGING
title: "sddk-cli no tiene logging estructurado: 0 tracing en todo el crate"
status: open
severity: medium
priority: P2
created: 2026-09-27
discovered_by: session-14 audit (code-based, no documentado previamente)
cluster_id: CL-OBSERVABILITY
fingerprint: "sddk_cli_no_tracing_v1"
---

## Qué es

`crates/sddk-cli/src/` contiene **0 ocurrencias de `tracing`**. El
diagnóstico de ejecución depende de `println!`/`eprintln!` sin estructura.

## Evidencia (verificada)

```
$ grep -rc 'tracing' crates/sddk-cli/src/ | grep -v ':0' | wc -l
0
$ grep -rn 'tracing' crates/ --include='*.rs' | wc -l
8        # los 8 son deps/manifest, ningún uso real
```

Contraste: `sddk-gateway/src/runner_receipt.rs` ya tiene un modelo de
receipt con `RedactionLocation`, `RedactionMarker` y `detect_canaries` —
la capacidad de observar y redactar ya existe en el dominio, simplemente
no se emiten eventos.

## Por qué importa

SDDK promete un ledger append-only de decisiones. Pero cuando una
ejecución falla, no hay forma de correlacionar:

- qué `cycle_id` produjo el fallo
- qué capability se intentó
- en qué punto de la pipeline

Los strings de stderr no son consultables, no tienen nivel, y no se
correlacionan con los eventos del ledger. Para un sistema cuyo valor
propuesto es ser auditable, la opacidad en runtime es una deficiencia
de diseño, no una carencia de tooling.

## Opciones

- **(a) `tracing` + `tracing-subscriber`** en el boundary de
  `cycle`/`ledger`/`verify`, con `cycle_id` y `request_id` como campos
  estructurados. Empezar por `cycle` (el path más consultado en
  incidentes). Coste: medio, aditivo.
- **(b) `tracing` solo en error paths**, sin migrar el happy path. Coste:
  bajo, valor acotado.

## Riesgo de no hacerlo

Bajo por ahora: el sistema es local-first y single-process, así que el
operador puede reproducir localmente. El coste sube si C5 (J8/J9,
operaciones host avanzadas y un segundo host real) se activa, que es
justo cuando la correlación deja de ser trivial.

## Estado

Abierto. No es quick win. No se ejecuta en este ciclo: tocar 4 crates
para observabilidad es un WorkItem propio con su propio baseline de
tests.

## Matiz añadido en session-18 (no cambia el veredicto, acota la solución)

El título dice "0 tracing" y eso es literalmente cierto, pero tomarlo
literalmente lleva a la solución equivocada. Verificado en el código:

```text
crates/sddk-cli/src/metrics.rs       -> 15 usos fuera del propio módulo
crates/sddk-cli/src/telemetry.rs     ->  6 usos fuera del propio módulo
crates/sddk-cli/src/analytics.rs     ->  2 usos fuera del propio módulo
```

**Ya existe una capa de observabilidad propia, cableada y en uso.** Añadir
`tracing` sin integrarlo con esto produce dos sistemas de telemetría
paralelos que no se correlate, que es un problema peor que el actual:
no es "no observo", pasa a "observo dos veces en formatos que no se
cruzan".

Por tanto la aceptación de este INC **no** es "meter la dependencia
`tracing`". Es, en orden de preferencia:

1. Integrar `metrics`/`telemetry` con correlación por `cycle_id` y
   `capability` (que es lo que el INC pide de verdad: poder correlacionar
   un fallo con su ciclo), o
2. Si se elige `tracing`, hacerlo **como fachada** sobre los módulos
   existentes, no como sistema paralelo.

Decisión de diseño, no de aplicación. Sin owner asignado.
