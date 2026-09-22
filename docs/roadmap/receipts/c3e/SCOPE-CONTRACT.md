# SCOPE-CONTRACT — C3e (T27 Schema resilience)

**Cycle:** C3e
**WorkItem:** T27 — Schema resilience (migrations correctness, downgrade attempts, partial migration recovery, schema_guard integration)
**Baseline:** `main@29fce83` (post-C3d close)
**Workspace baseline:** `v1.169.146`
**Date opened:** 2026-09-22T09:23:00Z
**Owner:** orchestrator (direct execution; subagent path remains unavailable)
**Status target:** PASS_OBSERVED on UAT T27-1..T27-8 with real evidence.

## 1. Falsable objective

> The migrations.rs + schema_guard.rs pipeline is **observable as honest** under the
> four resilience failure modes: fresh DB lands exactly at LATEST_SCHEMA_VERSION,
> stale N migrates forward, too-old is rejected fail-closed, and a future on-disk
> version is rejected fail-closed. Idempotency holds: re-running migrations is
> a no-op.

## 2. Background

- `crates/sddk-storage/src/migrations.rs` exposes `LATEST_SCHEMA_VERSION = 20` and
  `run_migrations(conn)` (pub(crate)). The pipeline is `1 → 2 → 3 → … → 20`,
  each step a separate `IMMEDIATE` transaction that bumps `user_version`.
- `crates/sddk-storage/src/schema_guard.rs` exposes `classify`, `check_compatibility`,
  `assert_compatible`, and the `SchemaCompatibility` enum (Exact / Migratable /
  NewerThanSupported / TooOld).
- MIGRATION_4 is special: it does `RENAME → recreate → INSERT → DROP` of
  `gate_receipts` to fix a CHECK constraint and a misnamed FK.
- MIGRATION_7 / MIGRATION_10 are special: defensive `pragma_table_info` checks
  because some columns might already exist (idempotent re-run).
- MIGRATION_20 is `DROP IF EXISTS` for the legacy `ledger_events` corpus.
- C3c (session-11) added 4 boundary tests on schema_guard; this cycle adds the
  end-to-end resilience tests against a real SQLite connection.

## 3. Invariants

1. `Storage::open_in_memory()` lands exactly at `LATEST_SCHEMA_VERSION = 20`.
2. Re-running migrations on a fully-migrated DB is a no-op (idempotent).
3. A DB whose `user_version` is in `[MIN_SUPPORTED_SCHEMA_VERSION, LATEST)`
   opens and migrates forward to LATEST.
4. A DB whose `user_version < MIN_SUPPORTED_SCHEMA_VERSION` is rejected
   fail-closed by `assert_compatible` (TooOldSchema).
5. A DB whose `user_version > LATEST_SCHEMA_VERSION` is rejected fail-closed
   by `assert_compatible` (NewerSchema).
6. `classify` is deterministic, monotonic, and matches the contract:
   `on_disk == COMPILED → Exact`, `on_disk > COMPILED → NewerThanSupported`,
   `on_disk < MIN → TooOld`, otherwise `Migratable`.
7. After migration, all C3c capability-receipt / cycle-lease / schema_guard
   tests still pass (no regression).

## 4. Non-objectives (this cycle)

- No production code change unless a real bug is found during the test
  execution. The cycle is **measurement + adversarial coverage**.
- No new migration (MIGRATION_21+).
- No rollback/downgrade support (would require writing inverse migrations;
  out of scope and explicitly REJECTED by the schema_guard contract).
- No criterion adoption (already deferred in C3d).
- No CAS corruption / event-store regression (C3b).

## 5. Code surface

- **Read-only files** (study only):
  - `crates/sddk-storage/src/migrations.rs`
  - `crates/sddk-storage/src/schema_guard.rs`
  - `crates/sddk-storage/src/lib.rs` (Storage ctor + schema_version accessor)
- **Test additions** (delta):
  - `crates/sddk-storage/src/lib.rs` `mod tests` — new sub-module
    `schema_resilience_tests` (or extend an existing one if a closer fit
    exists). The module exposes tests T27-1..T27-8.
- **No production code change** unless T27-3..T27-7 finds a real defect.

## 6. Tests added (UAT T27)

| ID | Test | What it proves |
|---|---|---|
| T27-1 | `fresh_storage_lands_at_latest_schema_version` | `Storage::open_in_memory().schema_version() == LATEST_SCHEMA_VERSION (20)`. The fresh-DB happy path. |
| T27-2 | `run_migrations_is_idempotent_on_fresh_db` | Apply migrations twice; user_version stays 20; no errors. |
| T27-3 | `partial_migration_completes_forward` | Pre-populate `user_version = 10` (only migrations 1..10 applied), open with `Storage::open_in_memory`-equivalent; migrates 11..20 idempotently. Schema-guard verdict: Migratable{from:10,to:20}. |
| T27-4 | `migratable_at_min_supported_is_not_too_old` | `user_version = MIN_SUPPORTED_SCHEMA_VERSION (1)`: classify returns `Migratable{from:1,to:20}`. assert_compatible passes. |
| T27-5 | `too_old_below_min_is_fail_closed` | `user_version = 0` (artificial pre-migration era). classify returns `TooOld{on_disk:0,binary_min:1}`. assert_compatible returns Err(TooOldSchema). |
| T27-6 | `newer_than_compiled_is_fail_closed` | `user_version = LATEST+1 = 21` (artificial future). classify returns `NewerThanSupported{on_disk:21,binary_max:20}`. assert_compatible returns Err(NewerSchema). |
| T27-7 | `extreme_future_clamps_diagnostics` | `user_version = 1_000_000`. assert_compatible returns Err(NewerSchema{on_disk:1_000_000,binary_max:20}); error message names both numbers. |
| T27-8 | `classify_monotonicity_invariant` | For every `on_disk ∈ [-2, -1, 0, MIN-1, MIN, MIN+1, COMPILED-1, COMPILED, COMPILED+1, COMPILED+5, 99999]`, classify returns the correct verdict. Single-parameterised test (table-driven). |

**Execution mechanism:** Some tests (T27-3..T27-7) require manipulating
`user_version` directly. The technique is:
1. Open a `rusqlite::Connection::open_in_memory()`.
2. Apply migrations 1..N selectively (or just bump `user_version` after a
   fresh apply, since the SQLite `pragma user_version` is freely settable
   from raw SQL).
3. Construct a `Storage` from that connection is NOT directly possible
   (Storage owns its connection). **Alternative**: Use a tempdir path.
   Apply migrations 1..N manually via `run_migrations` on a `Connection`
   to a tempdir file, then bump `user_version` down to N (rewind), close,
   reopen with `Storage::open(path)`, observe the migration completes.
   The test asserts `schema_version() == LATEST_SCHEMA_VERSION`.

For T27-5 / T27-6 / T27-7 (artificial too-old / too-new), the same tempdir
technique works: open, manually `pragma user_version = X` (X negative or
above LATEST), close, reopen with `Storage::open`, observe the guard
either migrates (Migratable branch) or refuses (fail-closed).

But Storage::open ALWAYS calls run_migrations unconditionally. So:
- For T27-5 (user_version = 0): run_migrations will migrate 1..20 (treating
  it like fresh); the test must observe user_version AFTER open and see
  LATEST. To exercise the FAIL-CLOSED path, the test must use
  `assert_compatible` on a Storage whose user_version is manually set ABOVE
  LATEST (or below MIN). This requires either:
  - (a) Constructing a Storage with a future schema by manually injecting
    `user_version = X > LATEST` (without running migrations, which would
    bring it back to LATEST). **Trick**: open an `Connection::open(path)`
    externally, set `pragma user_version = X`, close, then re-open via
    `Storage::open(path)` which calls `run_migrations` AGAIN — but
    run_migrations only applies `version < N` branches, so if version > 20,
    NOTHING runs. ✅
- For T27-6 (user_version > LATEST): same trick. After the trick, the DB
  has user_version = 21 but no MIGRATION_21 has run. Storage reports
  schema_version() == 21. Schema guard classifies → NewerThanSupported.
- For T27-5 (user_version < MIN): same trick: set user_version = 0.
  When we reopen with Storage::open, run_migrations sees version=0 and
  applies ALL migrations 1..20, ending at 20. So we lose the "too-old"
  observable UNLESS we use `assert_compatible` on the raw connection
  BEFORE calling run_migrations. **Better**: use `SchemaCompatibility::classify(0)`
  directly (already covered by schema_guard unit tests + T25-1 / T25-2
  added in C3c). For the END-TO-END "fail-closed" test, set
  user_version = MIN-1 (i.e., 0) and bypass Storage::open (which would
  auto-migrate) — classify directly via `schema_guard::classify`.
  Then separately verify that Storage::open on user_version=0 actually
  recovers to LATEST (that's the "too-old, but we still have migrations
  to bring you forward" case).

This is the cleanest split:
- **Pure-classify tests** (T27-5, T27-6, T27-7): use `schema_guard::classify`
  and `schema_guard::assert_compatible` directly. Already tested at unit
  level by C3c; C3e adds the **post-open** behaviour.
- **End-to-end open tests** (T27-1..T27-4, T27-8): use a tempdir + manual
  schema_version manipulation.

## 7. STOP conditions

- If a real defect is found (e.g., a migration that corrupts state, a guard
  that silently reinterprets), escalate to a fix cycle. C3e becomes a
  measurement-only PASS_OBSERVED with a deferred-fix addendum.
- No silent CI bypasses.

## 8. Risks

| Risk | Mitigation |
|---|---|
| tempdir leakage between tests | `tempfile::TempDir` guard pattern; every test uses a fresh tempdir. |
| Idempotency bug in MIGRATION_4 (gate_receipts recreate) | T27-2 explicitly runs migrations twice and re-checks user_version + row counts (one SELECT COUNT FROM gate_receipts). |
| Schema guard silently reinterpret | T27-6 / T27-7 assert Err (not Ok), and inspect the variant. |
| Test pollution from shared state | Each test uses its own tempdir; no shared `Storage` handles. |

## 9. Dependencies and authority

- No new dependencies.
- No new crates.
- No new ADR needed (this is adversarial coverage, not a new authority).
- Pre-push hook requires workspace bump after code-modified commits:
  `1.169.146 → 1.169.147` for the test commit.

## 10. Acceptance criteria

| # | Criterion | Status (post-cycle) |
|---|-----------|---------------------|
| 1 | All 8 UAT T27 tests pass | ⏳ |
| 2 | `cargo test -p sddk-storage --lib` ≥ 67/67 + 3 ignored + 8 new green | ⏳ |
| 3 | `cargo clippy -p sddk-storage --all-targets -- -D warnings` clean | ⏳ |
| 4 | `cargo fmt --all -- --check` clean | ⏳ |
| 5 | 0 production code changes (measurement only) | ⏳ |
| 6 | UAT-EVIDENCE.yaml with real numbers | ⏳ |
| 7 | RECEIPT committed | ⏳ |
| 8 | SESSION-JOURNAL.md has C3e entry | ⏳ |
| 9 | CURRENT/STATE reconciled | ⏳ |
