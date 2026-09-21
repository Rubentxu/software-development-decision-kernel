# RECEIPT — AIW-S8 X06 — Storage schema version guard

> **Slice:** `p-63676b11dc0ef88f/aiw-s8-x06-storage-schema-versioning`
> **Date:** 2026-09-21
> **Base HEAD:** 358686c9453f
> **Closes:** SCOPE-CONTRACT §S8-STOP-3 (X06)

## What was built

- `crates/sddk-storage/src/schema_guard.rs` (NEW): schema version guard.
  `COMPILED_SCHEMA_VERSION = LATEST_SCHEMA_VERSION` (import del módulo
  `pub(crate) migrations` dentro del mismo crate, sin fricción);
  `MIN_SUPPORTED_SCHEMA_VERSION = 1`. `classify(i32)` determinista;
  `check_compatibility(&Storage)`; `assert_compatible(&Storage)` con
  `GuardError::{CheckFailed, NewerSchema, TooOldSchema}` — fail-closed.
- `crates/sddk-storage/src/lib.rs`: `pub mod schema_guard;` + re-exports.
- `crates/sddk-storage/tests/aiw_s8_x06_schema_versioning.rs` (NEW):
  4 integration tests.

## Verification (OBSERVED)

| Gate | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p sddk-storage --all-targets -- -D warnings` | 0 warnings |
| `cargo test -p sddk-storage --lib schema_guard` | 5 passed; 0 failed |
| `cargo test -p sddk-storage --test aiw_s8_x06_schema_versioning` | 4 passed; 0 failed |
| `cargo build --release -p sddk-storage` | Finished `release` profile |

Total new tests: **9** (5 unit + 4 integration).

## §6 — Honest adjustments vs. task template

1. `StorageError::UnsupportedSchemaVersion` no existe; la variante real es
   `SchemaVersion { actual, expected }`. El guard define su propio
   `GuardError` con `CheckFailed(#[from] StorageError)` para no acoplar
   semántica de "read-only open" a la del guard.
2. El test `simulated_older_schema_rejected` del template fue reemplazado.
   Un `Storage` handle migrado no puede tener versión distinta de
   `LATEST` (open migra siempre), y escribir `user_version` a mano
   requeriría fabricar un esquema falso. En su lugar:
   - `classify(on_disk: i32)` es `pub` y cubre determinísticamente las
     ramas `NewerThanSupported` y `TooOld` con 4 unit tests + 1 de
     mensaje de error.
   - `MIN_SUPPORTED_SCHEMA_VERSION` es `pub const`, así un binario viejo
     puede clasificar honestamente.
   No hay ningún `#[ignore]`.
3. Los tests del template pasaban `dir.path()` (directorio) a
   `Storage::open`; corregido a `dir.path().join("ledger.sqlite")`
   (patrón de `sqlite_storage.rs`).
4. `MIN_SUPPORTED_SCHEMA_VERSION` añadido como constante explícita para
   que `TooOld` sea una rama alcanzable y testeable, no solo teórica.

## Notes

- `Storage::open_read_only` ya rechazaba mismatch con
  `StorageError::SchemaVersion` (fail-closed en ese camino). El guard
  extiende la misma disciplina a consumidores programáticos del handle.
