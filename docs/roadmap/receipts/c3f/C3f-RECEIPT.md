# C3f-RECEIPT — Migration re-application safety (T28)

**Cycle:** C3f
**WorkItem:** T28 — fix C3e-F1 (migration authority fail-closed under re-application)
**Baseline:** `main@35b0e9a` (workspace `v1.169.147`)
**HEAD post-cycle:** committed in same concern
**Date closed:** 2026-09-22T09:55:00Z
**Owner:** orchestrator (direct execution; subagent path remains unavailable)
**Status:** **PASS_OBSERVED** — C3e-F1 closed via ADR-0141 + `pre_flight_check` + `StorageError::InconsistentMigrationState`.

## Inputs

- **SCOPE-CONTRACT**: `docs/roadmap/receipts/c3f/SCOPE-CONTRACT.md`
- **ADR-0141**: `docs/architecture/adrs/ADR-0141-MIGRATION-AUTHORITY-MONOTONIC-ONLY.md`
- **Finding C3e-F1** (C3e): `run_migrations` re-application crashes on 17 of 20 migrations.

## Production code changes (first time in C3 sub-cycles)

| File | Change | Lines |
|---|---|---|
| `crates/sddk-storage/src/migrations.rs` | New `pre_flight_check` + `PRE_FLIGHT_ARTIFACTS` table + 3 probe helpers + call site at top of `run_migrations` | +148 |
| `crates/sddk-storage/src/lib.rs` | New `StorageError::InconsistentMigrationState` variant + Display + code + recovery | +27 |
| `crates/sddk-storage/src/lib.rs` (test) | New `mod schema_resilience_tests` T28-1..T28-6 + updated T27-3 + updated T27-4 comments | +156 |

Net production: **+175 lines**. Test code: **+156 lines**.

## Test additions (delta from `main@35b0e9a`)

| ID | Test | What it proves |
|---|---|---|
| T28-1 | `t28_1_pre_flight_rejects_rewound_to_v10_with_v11_table` | Setup DB at v=20, rewind user_version to 10, open via Storage. Storage::open returns `Err(InconsistentMigrationState)` naming MIGRATION_11 and `workflow_runs_v1`. |
| T28-2 | `t28_2_pre_flight_passes_on_fresh_db` | Open fresh in-memory storage; no diagnostic. Regression check. |
| T28-3 | `t28_3_pre_flight_passes_on_full_db` | Open DB at v=20 with all artifacts; `user_version` stays 20; no diagnostic. |
| T28-4 | `t28_4_pre_flight_detects_v16_column_after_rewind` | Rewind to 15, open. Storage::open returns `Err(InconsistentMigrationState)` naming MIGRATION_16 and `spine_order`. |
| T28-5 | `t28_5_pre_flight_detects_v19_column_after_rewind` | Rewind to 18, open. Storage::open returns `Err(InconsistentMigrationState)` naming MIGRATION_19 and `relation`. |
| T28-6 | `t28_6_inconsistent_state_error_carries_diagnostics` | Code is stable; Display names on_disk/migration/artifact; recovery names user_version and gives actionable advice. |

T27-3 (C3e) was **updated** to assert the new typed error instead of the raw SQLite message.

## Evidence (real commands, real output)

### Pre-cycle baseline
```
cargo test -p sddk-storage --lib → 75 passed; 0 failed; 3 ignored
cargo clippy -p sddk-storage --all-targets -- -D warnings → clean
cargo fmt --all -- --check → clean
```

### Post-cycle (default suite + new T28 + updated T27-3)
```
cargo test -p sddk-storage --lib → 81 passed; 0 failed; 3 ignored
cargo test -p sddk-storage --lib schema_resilience_tests → 14 passed; 0 failed
cargo test --workspace --lib → 27 passed; 0 failed
cargo build --workspace → clean
cargo clippy -p sddk-storage --all-targets -- -D warnings → clean
cargo fmt --all -- --check → clean
```

### Sample error message (T28-1)
```
inconsistent schema state: user_version is 10, but migration 11 artifact
"workflow_runs_v1" already exists. Catalog is ahead of user_version;
refusing to re-apply migrations. Diagnostic: catalog contains table
'workflow_runs_v1' but user_version is 10; migration 11 has not been
recorded as applied. Recovery: either restore a backup where the
catalog matches user_version, or fully migrate forward with user_version = 20.
```

### Recovery hint (T28-6)
```
user_version is 10 but migration 11 artifact 'workflow_runs_v1' already
exists in the catalog. The catalog is ahead of user_version. Recovery:
restore a backup where the catalog matches user_version, or set
user_version to 20 via a clean migration (no rewind). Do NOT manually
edit user_version downward while the catalog retains newer DDL.
```

## Decisions taken

- **Pre-flight detection, not migration hardening.** Two alternatives
  were considered (ADR-0141 §Strategy). The hardening approach
  (adding `IF NOT EXISTS` / `pragma_table_info` to every migration)
  was rejected because it would touch 12+ migrations and risk subtle
  DDL behavioral drift. The pre-flight approach changes one function
  in `migrations.rs` and adds one variant to `StorageError` —
  lower surface, lower risk.
- **Lowest conflicting migration is reported.** When multiple
  artifacts conflict, the operator sees the earliest divergence
  point. This gives a clearer "where to start recovery from" hint
  than listing all conflicts.
- **No criterion adoption, no multi-process race tests, no
  PRE_FLIGHT_ARTIFACTS consistency check** — all deferred per
  SCOPE §6 / §8.

## Out-of-scope (NOT_RUN, deferred)

- **Hardening individual migrations** — explicitly rejected by ADR-0141.
- **Multi-process migration race tests** — out of scope.
- **Pre-flight perf bench** — not gated.
- **`PRE_FLIGHT_ARTIFACTS` ↔ `MIGRATION_N` consistency test** —
  documented in ADR-0141 §Consequences as future hardening.

## Findings (closed)

| ID | Description | Status |
|---|---|---|
| C3e-F1 | `run_migrations` re-application crashes on 17 of 20 migrations | **CLOSED in C3f** |

## Acceptance criteria (SCOPE §10)

| # | Criterion | Status |
|---|-----------|--------|
| 1 | ADR-0141 written, accepted, in `docs/architecture/adrs/` | ✅ |
| 2 | `pre_flight_check` implemented in `migrations.rs` | ✅ |
| 3 | `StorageError::InconsistentMigrationState` added | ✅ |
| 4 | T28-1..T28-6 tests pass | ✅ 6/6 PASS_OBSERVED |
| 5 | T27-3 updated to assert the new error variant | ✅ |
| 6 | `cargo test -p sddk-storage --lib` ≥ 75 + 6 new green, no regression on 75 prior | ✅ 81/81 + 3 ignored |
| 7 | `cargo clippy -p sddk-storage --all-targets -- -D warnings` clean | ✅ |
| 8 | `cargo fmt --all -- --check` clean | ✅ (after one `cargo fmt` apply) |
| 9 | No regression in C3a-d | ✅ 27/27 workspace lib |
| 10 | UAT-EVIDENCE.yaml + RECEIPT committed with real numbers | ✅ |
| 11 | CURRENT/STATE reconciled to new SHA | ⏳ committed in same concern |
| 12 | Workspace bump 1.169.147 → 1.169.148 | ⏳ committed in same concern |

## Acceptance statement

> C3f is closed with **PASS_OBSERVED**. C3e-F1 is closed: the
> migration authority is now fail-closed under re-application. The
> pre-flight check intercepts inconsistent state before any DDL is
> executed, replacing the raw SQLite failure mode with a typed,
> operator-visible `StorageError::InconsistentMigrationState` carrying
> `on_disk`, `conflicting_migration`, `conflicting_artifact`, and
> `diagnostic`. ADR-0141 documents the contract and the chosen
> strategy (pre-flight, not migration hardening). 81/81 storage
> tests pass; clippy clean; workspace builds clean. The first C3
> sub-cycle to change production code, approved by ADR-0141.
