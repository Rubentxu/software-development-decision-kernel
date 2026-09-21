# SCOPE-CONTRACT — AIW-S8 X07 — Second-binary (read-only) integration

> **Slice id:** `p-63676b11dc0ef88f/aiw-s8-x07-second-binary-integration`
> **Parent:** `aiw-s8-cli-host-evaluation/SCOPE-CONTRACT.md` §S8-STOP-4
> **Status:** ✅ IMPLEMENTED
> **HEAD base:** 6136f2f

## §1 Goal

Close X07 (ARCH): `ninguna lógica de dominio en nuevo main. Prueba de
dependencia/fitness + segundo consumidor real.`

This slice delivers the **second consumer**: a second `Storage` handle
opened in read-only mode over the same database file written by a first
writer process, consuming the operational data without disturbing the
writer.

## §2 In scope

- `crates/sddk-storage/tests/aiw_s8_x07_second_binary_integration.rs`
  (NEW): 4 integration tests exercising the writer→reader handoff over
  one shared ledger path:
  - `writer_and_reader_share_one_storage_path`
  - `reader_cannot_write`
  - `reader_schema_matches_writer`
  - `reader_handles_empty_ledger_gracefully`

## §3 Contract pinned

- `Storage::open_read_only` succeeds over an existing writer-created
  ledger and exposes the canonical read surface (`get_project`,
  `list_events`, `get_cycle`, `schema_version`).
- Writes through the read-only handle are rejected by the underlying
  SQLite `SQLITE_OPEN_READ_ONLY` connection.
- Reader schema version equals writer's (20 = LATEST_SCHEMA_VERSION,
  post MIGRATION_20 canonical redirect).
- Empty ledger lists as an empty vec, never a panic.

## §4 Out of scope

- A literal second OS process (the read-only handle over the same file
  is the publish-grade integration the parent contract requested).
- Wiring the reader into the CLI binary itself (fitness-test territory,
  `crates/sddk-cli/tests/context_fitness.rs` remains the ARCH gate for
  no-domain-logic-in-main).
