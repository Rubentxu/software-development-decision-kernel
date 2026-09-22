# SCOPE-CONTRACT — C3f (T28 Migration re-application safety)

**Cycle:** C3f
**WorkItem:** T28 — migration re-application safety (fix C3e-F1)
**Baseline:** `main@35b0e9a` (post-C3e close)
**Workspace baseline:** `v1.169.147`
**Date opened:** 2026-09-22T09:41:00Z
**Owner:** orchestrator (direct execution; subagent path remains unavailable)
**Status target:** PASS_OBSERVED after fixing C3e-F1 via fail-closed pre-flight check + ADR-0141.

## 1. Falsable objective

> The `run_migrations` authority becomes **fail-closed under
> re-application**: when a DB is found in an inconsistent state (e.g.,
> `user_version` was rewound and the schema has tables/columns that
> correspond to a newer migration), `run_migrations` aborts with a
> clear, typed error **before** executing any DDL. The T27-3 failure
> mode (`SqliteFailure(1, "duplicate column: spine_order")`) is replaced
> with a typed `StorageError::InconsistentMigrationState`.

## 2. Background

C3e (T27-3) revealed finding **C3e-F1**: rewinding `user_version` and
re-running `run_migrations` crashes with raw SQLite errors because
17 of 20 migrations lack defensive guards. Only MIGRATION_4 (RENAME+recreate),
MIGRATION_7 (pragma_table_info on capability_receipts), and MIGRATION_10
(pragma_table_info on events_v1) have guards; the rest assume monotonic
`version < N`.

Severity: LOW for production (real flow never rewinds `user_version`),
MEDIUM for DR (operator might manually rewind to recover).

## 3. Decision

Two ADR candidates were considered:

**(A) Harden all migrations with `IF NOT EXISTS` / `pragma_table_info`
guards.** Touches 12+ migrations. Risk: subtle DDL differences between
guards can change observable behavior (e.g., `CREATE TABLE IF NOT EXISTS`
won't add a new column to an existing table). High risk.

**(B) Fail-closed pre-flight check.** A single new `pre_flight_check`
function is added to `run_migrations`. It validates that no "future
DDL artifact" exists at the current `user_version`. If validation
fails, `run_migrations` returns `StorageError::InconsistentMigrationState`
with a diagnostic naming the conflicting table/column/trigger. The
existing migrations stay unchanged.

**Decision: (B).** Reasons:
1. Smaller surface, lower risk.
2. The bug's severity (LOW for production) doesn't justify rewriting
   every migration.
3. The fail-closed pre-flight matches the schema_guard contract
   (never silently reinterpret; always refuse with diagnostics).
4. The diagnostic makes recovery clearer: "user_version=N but
   `events_v1.chain_hash` column exists — schema is ahead of
   user_version, refusing to proceed".

## 4. Strategy

### ADR-0141 (new)

Title: "Storage migration authority is monotonic-only; inconsistent
state fails closed."

Frontmatter:
- `id: ADR-0141-MIGRATION-AUTHORITY-MONOTONIC-ONLY`
- `status: accepted`
- `adopted_at: 2026-09-22`
- `acceptance_cycle: p-63676b11dc0ef88f/c3f-migrations-monotonic`
- `supersedes_history: false`
- `related_adrs: [ADR-0097-COMMON-REVISION-SUBSTRATE]` (uses the
  revision-substrate ADR's "append-only / fail-closed" pattern)
- `stale_after: 2027-09-22`

Body: declares the migration authority contract as **monotonic-only**.
Re-application safety is delegated to a `pre_flight_check` that detects
inconsistent state and aborts. The 3 already-guarded migrations
(MIGRATION_4, MIGRATION_7, MIGRATION_10) are documented as exceptions
because they were written before the contract was formalized.

### Implementation (production code)

Add to `crates/sddk-storage/src/migrations.rs`:

```rust
/// Pre-flight check for inconsistent schema state.
///
/// Compares `user_version` against the SQLite catalog and refuses to
/// proceed if the catalog contains DDL artifacts that correspond to a
/// migration > `user_version`. This catches the C3e-F1 failure mode
/// (rewound `user_version` with newer tables/columns still present)
/// BEFORE any DDL is executed, replacing a raw SQLite error with a
/// typed `StorageError::InconsistentMigrationState`.
///
/// Returns `Ok(())` when the catalog is consistent with `user_version`.
pub(crate) fn pre_flight_check(
    conn: &rusqlite::Connection,
    on_disk: i32,
) -> Result<(), super::StorageError>;
```

The check iterates over migrations `on_disk+1..=LATEST_SCHEMA_VERSION`
and verifies that none of their key DDL artifacts exist in the catalog.
For each migration we list the **first artifact** that would fail:

| Migration | Key artifact (first) | Catalog probe |
|-----------|----------------------|---------------|
| MIGRATION_1 | `projects` table | `SELECT 1 FROM sqlite_master WHERE type='table' AND name='projects'` |
| MIGRATION_2 | `gate_receipts` table | same pattern |
| MIGRATION_5 | `events_v1` table | same |
| MIGRATION_11 | `workflow_runs_v1` table | same |
| MIGRATION_12 | `incs_v1` table | same |
| MIGRATION_14 | `work_items_v1` table | same |
| MIGRATION_15 | `evidence_attachments_v1` table | same |
| MIGRATION_16 | `work_items_v1.spine_order` column | `SELECT 1 FROM pragma_table_info('work_items_v1') WHERE name='spine_order'` |
| MIGRATION_17 | `workflow_run_events_v1` table | same |
| MIGRATION_18 | `backlog_items_v1` table | same |
| MIGRATION_19 | `evidence_attachments_v1.relation` column | pragma_table_info check |
| MIGRATION_20 | (DROP-only — no check) | n/a |

If a check finds the artifact exists, return
`Err(InconsistentMigrationState { on_disk, conflicting_migration, conflicting_artifact, diagnostic })`.

The error variant is added to `StorageError` in `lib.rs`.

### Test additions (T28)

| ID | Test | What it proves |
|---|---|---|
| T28-1 | `pre_flight_rejects_rewound_to_v10_with_v11_table_present` | Setup DB at v=20, rewind user_version to 10, open via Storage. Storage::open returns `Err(InconsistentMigrationState)` naming MIGRATION_11 and `workflow_runs_v1`. |
| T28-2 | `pre_flight_passes_on_fresh_db` | Open fresh in-memory storage; no diagnostic. |
| T28-3 | `pre_flight_passes_on_full_db` | Open DB at v=20 with all artifacts; `user_version` stays 20; no diagnostic (consistent state). |
| T28-4 | `pre_flight_detects_v16_column_after_rewind` | Setup DB at v=20, rewind user_version to 15, open. Storage::open returns `Err(InconsistentMigrationState)` naming MIGRATION_16 and `spine_order`. |
| T28-5 | `pre_flight_detects_v19_column_after_rewind` | Rewind to 18, open. Storage::open returns `Err(InconsistentMigrationState)` naming MIGRATION_19 and `relation`. |
| T28-6 | `inconsistent_state_error_carries_diagnostics` | The `InconsistentMigrationState` variant's Display names the conflicting migration and the artifact. |

T27-3 will be **updated** to expect the new error variant instead of
the raw SQLite failure.

## 5. Invariants (after C3f)

1. `Storage::open` on a DB with `user_version` in `[MIN_SUPPORTED, LATEST]` lands at LATEST (already true).
2. `Storage::open` on a DB with `user_version` in `[MIN_SUPPORTED, LATEST]` and inconsistent catalog fails fast with `InconsistentMigrationState` (new).
3. `Storage::open` on a DB with `user_version > LATEST` fails fast with `GuardError::NewerSchema` (already true, covered by C3e T27-6).
4. `Storage::open` on a fresh DB lands at LATEST_SCHEMA_VERSION (already true, covered by C3e T27-1).
5. Re-running migrations on a fully-migrated DB is a no-op (already true, covered by C3e T27-2).

## 6. Non-objectives

- No re-application hardening of the migrations themselves (the IF NOT EXISTS / pragma_table_info guards). ADR-0141 declares the alternative: pre-flight detection + typed error.
- No new migration (MIGRATION_21+).
- No downgrade/rollback support (already rejected by schema_guard contract).
- No criterion adoption (deferred in C3d).
- No DDL changes to existing migrations.

## 7. Code surface

- **Modified (production):**
  - `crates/sddk-storage/src/migrations.rs` — add `pre_flight_check` + call site at top of `run_migrations`.
  - `crates/sddk-storage/src/lib.rs` — add `StorageError::InconsistentMigrationState` variant.
- **Modified (test):**
  - `crates/sddk-storage/src/lib.rs::schema_resilience_tests` — update T27-3, add T28-1..T28-6.

## 8. STOP conditions

- If `InconsistentMigrationState` cannot be added to `StorageError` without breaking downstream crates, escalate (would require a different error type or API).
- If `pre_flight_check` accidentally produces false positives (e.g., a fresh in-memory DB returns inconsistent state), escalate.

## 9. Risks

| Risk | Mitigation |
|---|---|
| False positives on fresh DBs | T28-2 explicitly verifies a fresh in-memory DB passes. |
| StorageError variant breaks downstream | Downstream impact surveyed via `grep -r "StorageError" crates/` (planned). |
| T27-3 conflicts with T28-1 | T27-3 must be updated to expect `InconsistentMigrationState` instead of the raw SQLite error. |
| Pre-flight becomes a perf bottleneck | The catalog probes use `sqlite_master` + `pragma_table_info`, both indexed. Expected overhead: ~1ms total per open on a populated DB. Will be measured in T28-7 (perf bench, opt-in). |

## 10. Acceptance criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | ADR-0141 written, accepted, in `docs/architecture/adrs/` | ⏳ |
| 2 | `pre_flight_check` implemented in `migrations.rs` | ⏳ |
| 3 | `StorageError::InconsistentMigrationState` added | ⏳ |
| 4 | T28-1..T28-6 tests pass | ⏳ |
| 5 | T27-3 updated to assert the new error variant | ⏳ |
| 6 | `cargo test -p sddk-storage --lib` ≥ 75 + 6 new green, no regression on 75 prior | ⏳ |
| 7 | `cargo clippy -p sddk-storage --all-targets -- -D warnings` clean | ⏳ |
| 8 | `cargo fmt --all -- --check` clean | ⏳ |
| 9 | No regression in C3a-d (authority + adversarial + canarios + perf) | ⏳ |
| 10 | UAT-EVIDENCE.yaml + RECEIPT committed with real numbers | ⏳ |
| 11 | CURRENT/STATE reconciled to new SHA | ⏳ |
| 12 | Workspace bump 1.169.147 → 1.169.148 | ⏳ |

## 11. Honest limits

- T28-7 (perf bench) is deferred — opt-in only, no perf gate added.
- The fix is detection, not prevention. An operator who rewinds
  `user_version` still sees an error, just a clearer one. Full
  re-application safety would require rewriting every migration
  with guards and is explicitly out of scope.
