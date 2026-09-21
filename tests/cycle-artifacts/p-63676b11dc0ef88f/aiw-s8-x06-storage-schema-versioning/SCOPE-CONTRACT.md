# SCOPE — AIW-S8 X06 — Storage schema versioning

> **Slice:** `p-63676b11dc0ef88f/aiw-s8-x06-storage-schema-versioning`
> **Parent:** `aiw-s8-cli-host-evaluation/SCOPE-CONTRACT.md` §S8-STOP-3
> **Base HEAD:** 358686c9453f
> **Date:** 2026-09-21

## Question

Un CLI compilado contra una versión de esquema antigua puede leer datos
escritos por un esquema nuevo sin comprobación explícita de compatibilidad.

## Contract

Compatibilidad o fallo explícito documentado, nunca interpretación falsa.

## In scope

- `crates/sddk-storage/src/schema_guard.rs` (NEW): guard de versión de
  esquema que falla cerrado (error claro) cuando el mismatch no es
  reconciliable. `classify(version)` expone determinísticamente las cuatro
  ramas (`Exact`, `Migratable`, `NewerThanSupported`, `TooOld`);
  `check_compatibility(&Storage)` clasifica el esquema on-disk;
  `assert_compatible(&Storage)` falla cerrado con `GuardError`.
- Re-export en `crates/sddk-storage/src/lib.rs`.
- Tests de integración `tests/aiw_s8_x06_schema_versioning.rs`.

## Out of scope

- Código de migración en producción. `Storage::open` ya migra forward;
  el guard solo clasifica y decide.
- Cambios al esquema o a `migrations.rs` (solo lectura de
  `LATEST_SCHEMA_VERSION`, `pub(crate)`, importable dentro del crate).

## Fail-closed rule

`assert_compatible` devuelve `Err` en `NewerThanSupported` y `TooOld`;
jamás interpreta datos de un esquema no soportado como si lo fueran.
