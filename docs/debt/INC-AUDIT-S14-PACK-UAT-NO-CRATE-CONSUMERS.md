---
id: INC-AUDIT-S14-PACK-UAT-NO-CRATE-CONSUMERS
title: "sddk-pack-uat: 0 consumidores como crate, pero es un pack de la arquitectura — no es código muerto"
status: open
severity: low
priority: P3
created: 2026-09-27
discovered_by: session-14 audit (code-based)
cluster_id: CL-PACK-BOUNDARY
fingerprint: "sddk_pack_uat_no_crate_consumers_v1"
---

## Observación

`crates/sddk-pack-uat/` (250 LOC: `lib.rs` 52 + `conformance.rs` 198)
está declarado como workspace member, compila, tiene 0 tests propios, y
**ningún `Cargo.toml` del workspace lo declara como dependencia**
(verificado: la única coincidencia de `sddk-pack-uat` es su propia
entrada en el `[workspace]` de `Cargo.toml`).

## Por qué NO es código muerto (contra la intuición inicial)

La primera lectura fue "placeholder compilado, 0 consumidores → borrar".
**Esa conclusión es incorrecta**, y volver a repetirla sería el mismo
error que cometí con `test_ports.rs`.

Este crate es un **pack de la arquitectura**, no una librería auxiliar:

- `ADR-0104` (Pack extension boundary) establece que los packs son
  unidades de primera clase con su propia boundary.
- El doc comment de `lib.rs` declara explícitamente el propósito:
  "the SDDK UAT pack: guided runner, form DSL, scenario management, and
  evidence collection", con 5 capabilities nombradas
  (`uat.plan.create`, `uat.plan.execute`, `uat.plan.approve`,
  `uat.evidence.collect`, `uat.receipt.emit`).
- Declara la frontera: los tipos UAT viven aquí, mientras el modelo
  universal de evidencia permanece en `sddk-domain`. El pack depende de
  `sddk-domain` y re-exporta para acceso ergonómico.
- Está en fase de **extracción Phase 4**: los tipos se re-exportan
  todavía desde `sddk_domain::uat` por compatibilidad, y el doc dice que
  "once all consumers are migrated, the canonical location for UAT types
  will be this crate".

Los packs se consumen vía **bundle**, no como dependencia de crate Rust.
Que 0 crates lo dependan es la forma normal de un pack, no una señal de
abandono.

## Por qué sigue siendo deuda (low/P3, no más)

- 250 LOC compilados, sin tests propios, cuya ubicación canónica aún no
  es efectiva (los tipos siguen en `sddk_domain::uat`).
- La fase de migración depende de migrar consumidores, y el criterio de
  "todos los consumidores migrados" no está definido ni medido.
- Riesgo real: dos ubicaciones canónicas de los tipos UAT durante la
  transición. El doc lo reconoce, pero nada lo hace verificable.

## Recomendación

**No borrar.** Medir el criterio de completitud en vez de specular:

- contar los consumidores que aún importan `sddk_domain::uat`
- cuando llegue a 0, completar la migración y decidir si el crate
  permanece (como boundary de pack) o se disuelve

Añadir un test que afirme la frontera (p. ej. que el pack no importa
tipos de otro pack) daría teeth a ADR-0104 sin depender de la
migración.

## Estado

Abierto. Documental/deuda de proceso, no de código. No requiere acción
de implementación hasta que la migración de consumidores avance.
