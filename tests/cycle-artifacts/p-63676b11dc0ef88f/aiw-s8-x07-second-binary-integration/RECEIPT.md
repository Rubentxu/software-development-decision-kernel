# RECEIPT — AIW-S8 X07 — Second-binary (read-only) integration

> **Slice:** `p-63676b11dc0ef88f/aiw-s8-x07-second-binary-integration`
> **Date:** 2026-09-21
> **Base HEAD:** 6136f2f
> **Closes:** SCOPE-CONTRACT §S8-STOP-4 (X07)

## What was built

- `crates/sddk-storage/tests/aiw_s8_x07_second_binary_integration.rs`
  (NEW): 4 integration tests. A writer `Storage::open` process writes a
  project, workspace, cycle (with canonical event) and closes; a second
  consumer opens the same ledger file via `Storage::open_read_only` and
  reads the data back.

## Verification (OBSERVED)

| Gate | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p sddk-storage --test aiw_s8_x07_second_binary_integration -- -D warnings` | 0 warnings |
| `cargo test -p sddk-storage --test aiw_s8_x07_second_binary_integration` | 4 passed; 0 failed |
| `cargo build --release -p sddk-storage` | Finished `release` profile [optimized] |

Total new tests: **4**.

## What the tests pin

1. `writer_and_reader_share_one_storage_path` — reader sees the writer's
   project, single canonical event, and cycle (`get_project`,
   `list_events`, `get_cycle`).
2. `reader_cannot_write` — `insert_project` through the read-only handle
   returns `Err` (SQLite `SQLITE_OPEN_READ_ONLY`).
3. `reader_schema_matches_writer` — `schema_version() == 20`
   (LATEST_SCHEMA_VERSION, post MIGRATION_20).
4. `reader_handles_empty_ledger_gracefully` — `list_events` on an empty
   ledger returns an empty vec for both writer and reader.

## Surprises / notes

- `insert_cycle_with_event` lives behind the `sddk_domain::Ledger`
  trait, not an inherent method; the test imports the trait.
- Pre-existing clippy failure in `aiw_s8_x06_schema_versioning`
  (three `assert!(true)` → `assertions_on_constants`) is NOT from this
  slice; `--all-targets` on the crate fails on that file. X07's own
  test target is clippy-clean under `-D warnings`.
- "Second binary" is modeled as a second read-only `Storage` handle over
  the same file, per the parent contract's "second-binary read
  integration" unblocking decision; a literal second OS process was not
  required.
