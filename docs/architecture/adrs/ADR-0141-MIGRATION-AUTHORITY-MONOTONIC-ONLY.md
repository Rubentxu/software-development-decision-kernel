---
id: ADR-0141-MIGRATION-AUTHORITY-MONOTONIC-ONLY
status: accepted
supersedes_history: false
adopted_at: 2026-09-22
adoption_cycle: p-63676b11dc0ef88f/c3f-migrations-monotonic
package_local_id: null
package_source: null
accepted_at: 2026-09-22
accepted_by_cycle: p-63676b11dc0ef88f/c3f-migrations-monotonic
superseded_by: []
related_adrs:
  - ADR-0097-COMMON-REVISION-SUBSTRATE
stale_after: 2027-09-22
---

# ADR-0141 — Storage migration authority is monotonic-only; inconsistent state fails closed

> New ADR written in repo (no package source).

## Context

The `sddk-storage` crate owns a SQLite-based storage authority
(`run_migrations` in `crates/sddk-storage/src/migrations.rs`) with 20
ordered migrations (`MIGRATION_1` through `MIGRATION_20`) tracked via
the SQLite `user_version` pragma. The historical contract is
**monotonic-only**: `user_version` advances strictly forward via
`IMMEDIATE` transactions, and each migration's DDL assumes a clean
catalog at `version < N`.

Cycle C3e (T27-3) revealed **finding C3e-F1**: when `user_version` is
forcibly rewound (e.g., disaster-recovery operator intervention,
backup-restore mistake, or partial migration corruption), the next
`run_migrations` re-applies migrations on a DB whose catalog already
contains the corresponding tables/columns/triggers. 17 of the 20
migrations lack defensive guards against re-application. Only
MIGRATION_4 (intrinsically safe via RENAME+recreate), MIGRATION_7, and
MIGRATION_10 have guards. The other 17 crash with raw SQLite errors
(e.g., `SqliteFailure(1, "duplicate column name: spine_order")`).

Severity: **LOW** for normal operation (production never rewinds
`user_version`) and **MEDIUM** for disaster-recovery scenarios.

## Decision

The migration authority is **monotonic-only**, with the following
contract:

1. **`user_version` always advances forward.** No re-application of
   migrations to a DB whose `user_version` is lower than a DDL artifact
   present in the catalog.

2. **`run_migrations` calls `pre_flight_check` BEFORE any DDL.** If
   the catalog is inconsistent with `user_version`, the function
   returns `Err(StorageError::InconsistentMigrationState)` and the
   DDL is **not** executed.

3. **`pre_flight_check` is a pure read.** It scans `sqlite_master` and
   `pragma_table_info` for artifacts that correspond to migrations
   strictly greater than `user_version`. The check is a constant table
   (`PRE_FLIGHT_ARTIFACTS` in `migrations.rs`) plus three small probe
   helpers (`artifact_table_exists`, `artifact_trigger_exists`,
   `artifact_column_exists`).

4. **The error is typed and operator-visible.** `StorageError::
   InconsistentMigrationState { on_disk, conflicting_migration,
   conflicting_artifact, diagnostic }` carries the on-disk version,
   the lowest-numbered migration whose artifact was found, the
   artifact name, and a human-readable diagnostic. The Display string
   and `recovery()` hint both name `user_version`, the conflicting
   migration, and the artifact, with a recommended recovery path.

5. **Already-guarded migrations are documented exceptions.** MIGRATION_4
   (RENAME+recreate), MIGRATION_7 (pragma_table_info guard), and
   MIGRATION_10 (pragma_table_info guard) were written before this
   contract was formalized and are intrinsically safe or self-guarded.
   They are intentionally absent from `PRE_FLIGHT_ARTIFACTS`. MIGRATION_8
   is `SELECT 1;` (no-op); MIGRATION_20 is DROP-only.

## Strategy (alternatives considered)

**Alternative A**: harden every migration with `IF NOT EXISTS` /
`pragma_table_info` guards. Touches 12+ migrations. Risk: subtle DDL
differences between guards can change observable behavior (e.g.,
`CREATE TABLE IF NOT EXISTS` won't add a new column to an existing
table; re-applying `ALTER TABLE ADD COLUMN` is a no-op on
`pragma_table_info` but the column is still declared with `NOT NULL`
constraints in some migrations). High risk of subtle behavioral drift.

**Alternative B (chosen)**: pre-flight detection + typed error.
Single new function (`pre_flight_check`) plus a new variant on
`StorageError` (one match arm in `code()` and `recovery()`). Existing
migrations stay unchanged. Lower risk, smaller surface. The
diagnostic gives the operator a clear recovery path.

## Invariants (after adoption)

1. `Storage::open` on a DB with `user_version` in `[MIN_SUPPORTED,
   LATEST]` and a consistent catalog lands at LATEST.
2. `Storage::open` on a DB with `user_version` in `[MIN_SUPPORTED,
   LATEST]` and an inconsistent catalog fails fast with
   `InconsistentMigrationState`. No DDL is executed.
3. `Storage::open` on a DB with `user_version > LATEST` fails fast
   with `GuardError::NewerSchema` (covered by `schema_guard`,
   independently).
4. `Storage::open` on a fresh DB lands at LATEST_SCHEMA_VERSION.
5. Re-running migrations on a fully-migrated DB is a no-op.

## Non-goals

- **No migration re-application hardening.** The 17 un-guarded
  migrations stay as-is. The pre-flight check intercepts the failure
  mode before any DDL is attempted. A future cycle (post-ADR) may
  harden individual migrations if needed.
- **No downgrade / rollback support.** Explicitly rejected by the
  `schema_guard` contract: an on-disk schema newer than the binary
  supports is `NewerThanSupported` (fail-closed).
- **No multi-process migration race tests.** Out of scope for unit
  tests. `user_version` is updated inside `IMMEDIATE` transactions,
  which serialise writers at the SQLite layer.
- **No criterion adoption** (deferred in C3d).
- **No MIGRATION_21+.**

## Consequences

Positive:
- The C3e-F1 failure mode is closed. Any DB with rewound
  `user_version` and a non-empty catalog now produces a typed,
  recoverable error instead of a raw SQLite failure.
- The error message and recovery hint name the conflicting migration
  and the artifact, making operator intervention faster and safer.
- Existing migrations stay unchanged, so no upgrade-path risk.

Negative:
- The fix is detection, not prevention. An operator who rewinds
  `user_version` still sees an error; we just made it clearer.
- The pre-flight adds 15 catalog probes per open. Measured overhead
  in a populated DB: sub-millisecond (sqlite_master is a small
  in-memory table). Not gated, but documented as a perf-relevant
  observation for future profiling.
- The `PRE_FLIGHT_ARTIFACTS` table must be kept in sync with new
  migrations. **Mitigation**: each future migration is required by
  SCOPE-CONTRACT to add its primary artifact to the table (a code
  review check; could be enforced by a `#[test]` that compares the
  set against the set of `MIGRATION_N` strings, but not in this
  cycle).

## Acceptance

- T27-3 (C3e) updated to assert the new typed error.
- T28-1..T28-6 (C3f) added: pre-flight detection across multiple
  artifact kinds (table, column, trigger), pass-on-fresh and
  pass-on-full regression checks, and operator-visible error surface
  (code + Display + recovery).
- 81/81 storage lib tests pass; clippy clean.

## See also

- `crates/sddk-storage/src/migrations.rs` (the authority)
- `crates/sddk-storage/src/schema_guard.rs` (companion guard for
  new-than-supported, fail-closed at the storage layer)
- `docs/roadmap/receipts/c3f/{SCOPE-CONTRACT,UAT-EVIDENCE.yaml,
  C3f-RECEIPT}.md`
- ADR-0097 (common revision substrate — fail-closed pattern origin)
