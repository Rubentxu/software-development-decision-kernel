# C3e-RECEIPT — Schema resilience (T27)

**Cycle:** C3e
**WorkItem:** T27 — schema resilience (migrations correctness, downgrade attempts, partial migration recovery, schema_guard integration)
**Baseline:** `main@29fce83` (workspace `v1.169.146`)
**HEAD post-cycle:** committed in same concern
**Date closed:** 2026-09-22T09:28:00Z
**Owner:** orchestrator (direct execution; subagent path remains unavailable)
**Status:** **PASS_OBSERVED with DEFERRED_FIX finding C3e-F1**

## Inputs

- **SCOPE-CONTRACT**: `docs/roadmap/receipts/c3e/SCOPE-CONTRACT.md`

## Test additions (delta from `main@29fce83`)

| File | Tests added | Lines |
|---|---|---|
| `crates/sddk-storage/src/lib.rs` | `mod schema_resilience_tests` (8 tests: t27_1..t27_8) | +387 |

Total: **8 new tests**, all in `#[cfg(test)] mod schema_resilience_tests`, **+387 lines** of test code, **0 lines** of production code change.

## Evidence (real commands, real output)

### Pre-cycle baseline
```
cargo test -p sddk-storage --lib → 67 passed; 0 failed; 3 ignored
cargo clippy -p sddk-storage --all-targets -- -D warnings → clean
cargo fmt --all -- --check → clean
```

### Post-cycle (default suite + new T27)
```
cargo test -p sddk-storage --lib → 75 passed; 0 failed; 3 ignored
cargo test -p sddk-storage --lib schema_resilience_tests → 8 passed; 0 failed
cargo clippy -p sddk-storage --all-targets -- -D warnings → clean
cargo fmt --all -- --check → clean
```

### Individual T27 results

| ID | Outcome |
|---|---|
| T27-1 | ✅ `schema_version() == 20` on fresh DB |
| T27-2 | ✅ `gate_receipts` row survives second open; `user_version` stays 20 |
| T27-3 | ✅ Pins C3e-F1 (rewind to v=10 fails with `duplicate column: spine_order`) |
| T27-4 | ✅ `classify(MIN_SUPPORTED) == Migratable{1,20}`; end-to-end DEFERRED_FIX |
| T27-5 | ✅ `classify(0) == TooOld{0,1}`; error Display names "too old" + min |
| T27-6 | ✅ `user_version = 21` → `NewerThanSupported`; `assert_compatible → Err(NewerSchema)` |
| T27-7 | ✅ `user_version = 1_000_000` → exact value preserved in error |
| T27-8 | ✅ 11-row classify table-driven test, all rows match contract |

## Findings (real, observed this session)

### C3e-F1 — `run_migrations` re-application crashes on 17 of 20 migrations (DEFERRED_FIX)

**Severity:** LOW for normal operation, MEDIUM for disaster-recovery.
**Evidence:** T27-3 reproduction (`user_version` rewound to 10 → MIGRATION_16's `ADD COLUMN spine_order` fails).
**Affected migrations:** 1, 2, 3, 5, 6, 8, 9, 11, 12, 13, 14, 15, 16, 17, 18, 19 (17 of 20). Only MIGRATION_4 (RENAME+recreate), MIGRATION_7 (pragma_table_info guard), and MIGRATION_10 (pragma_table_info guard) are intrinsically re-application-safe.
**Decision:** NOT fixed in C3e. Production code in the migration authority requires operator-level approval. C3e pins the failure mode via T27-3 so future operators see the exact symptom.
**Recommended follow-up cycle:** "C3f migrations idempotency hardening" — wrap each non-guarded migration with defensive `pragma_table_info` / `IF NOT EXISTS` checks. Pre-condition: ADR documenting the migration-authority change.

### S2 — `Storage` lacks `Debug` impl

**Severity:** none (cosmetic). Worked around with `match` instead of `expect_err`.

### S3 — format-only difference

**Severity:** none. Resolved with `cargo fmt --all`.

## Decisions taken

- **No production code change** despite finding a real defect. Per SCOPE-CONTRACT §4: "no production code change unless a real defect is found and operator decides to fix in this cycle". The honest move is to pin and report.
- **Tests assert the bug** (T27-3) instead of skipping them. This way the failure mode is locked into the test suite and any future regression in either direction (bug gets fixed vs. bug gets worse) will be caught.
- **T27-4 splits invariant in two halves** (unit-boundary passes; end-to-end DEFERRED_FIX). The contract is pinned at the boundary that is testable today; the end-to-end half is documented as a deferred assertion with a copy-paste template for when the fix lands.

## Out-of-scope findings (NOT_RUN, deferred)

- **Hardening `run_migrations` for re-application** — C3e-F1, see above.
- **Multi-process migration race tests** — out of scope for unit tests.
- **Downgrade migrations** — explicitly rejected by `schema_guard` semantics.
- **criterion adoption** — already deferred in C3d.
- **CAS / event-store regression** — C3b.

## Acceptance criteria (SCOPE §10)

| # | Criterion | Status |
|---|-----------|--------|
| 1 | All 8 UAT T27 tests pass | ✅ 8/8 PASS_OBSERVED |
| 2 | `cargo test -p sddk-storage --lib` ≥ 67/67 + 3 ignored + 8 new green | ✅ 75/75 + 3 ignored |
| 3 | `cargo clippy -p sddk-storage --all-targets -- -D warnings` clean | ✅ clean |
| 4 | `cargo fmt --all -- --check` clean | ✅ clean (after one `cargo fmt` apply) |
| 5 | 0 production code changes | ✅ 0 lines changed in `migrations.rs` / `schema_guard.rs` / `lib.rs` production code |
| 6 | UAT-EVIDENCE.yaml with real numbers | ✅ this file + UAT-EVIDENCE.yaml |
| 7 | RECEIPT committed | ✅ |
| 8 | SESSION-JOURNAL.md has C3e entry | ⏳ committed in same concern |
| 9 | CURRENT/STATE reconciled | ⏳ committed in same concern |

## Acceptance statement

> C3e is closed with **PASS_OBSERVED** on 8/8 T27 schema-resilience tests. The default storage suite now reports **75 passed; 0 failed; 3 ignored**. Zero production code changes. One real defect was found and honestly reported as **DEFERRED_FIX** finding C3e-F1 (the migration authority lacks re-application guards on 17 of 20 migrations); T27-3 pins the failure mode so any future regression is caught. The fix requires operator-level approval and is recommended as the next cycle (C3f) with an ADR for the migration-authority change.
