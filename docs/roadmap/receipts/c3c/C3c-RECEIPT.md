# C3c-RECEIPT — Storage Security canarios (T23, T24, T25)

**Cycle:** C3c
**WorkItem:** T23 Capability receipts + T24 Cycle leases + T25 Schema guard boundary
**Baseline:** `main@edaea67` (workspace v1.169.144)
**HEAD post-cycle:** committed in same concern
**Date closed:** 2026-09-22T09:02:00Z
**Owner:** orchestrator (direct execution)
**Status:** **PASS_OBSERVED**

## Inputs

- **SCOPE-CONTRACT**: `docs/roadmap/receipts/c3c/SCOPE-CONTRACT.md`
- **Findings pre-investigación** (Section 2 of SCOPE): F1-F4 (capability receipt guards), F5-F8 (cycle lease guards), F9 (schema_guard boundary).

## Test additions (delta from `main@edaea67`)

| File | Tests added | Lines |
|---|---|---|
| `crates/sddk-storage/src/lib.rs` | `begin_with_terminal_status_is_rejected`, `finalize_with_started_status_is_rejected`, `finalize_already_terminal_rejected_with_typed_guard`, `idempotent_retry_with_same_request_returns_existing_receipt`, `idempotent_retry_with_different_request_returns_conflict` (capability_receipt_security_tests, +181); `acquire_with_negative_now_ms_is_rejected`, `acquire_with_expires_at_or_before_now_is_rejected`, `acquire_on_missing_cycle_returns_not_found`, `acquire_with_active_lease_returns_typed_conflict`, `expired_lease_reacquire_increments_fencing_token` (cycle_lease_security_tests, +147) | +328 |
| `crates/sddk-storage/src/schema_guard.rs` | `classify_at_min_supported_version_is_not_too_old`, `classify_just_below_min_is_too_old`, `too_old_guard_error_carries_diagnostics`, `newer_schema_guard_error_carries_diagnostics` | +78 |

Total: **14 new tests**, **+406 lines** of test code, **0 lines** of production code change.

## Evidence (real commands, real output)

### Baseline (pre-cycle)
```
cargo test -p sddk-storage --lib → 53/53 ok (post-C3b baseline)
cargo fmt --all -- --check → ok
cargo clippy -p sddk-storage --all-targets -- -D warnings → ok
```

### Post-cycle
```
cargo fmt --all -- --check → ok (post cargo fmt --all)
cargo clippy -p sddk-storage --all-targets -- -D warnings → ok
cargo test -p sddk-storage --lib → 67/67 passed, 0 failed, 0 ignored (was 53/53)
```

Per-suite breakdown (post-cycle):
```
capability_receipt_security_tests → 5/5 ok
cycle_lease_security_tests        → 5/5 ok
schema_guard::tests               → 9/9 ok (was 5)
```

## Surprises during execution (real, not pre-planned)

1. **`CycleStatus` variants**: my first attempt used `CycleStatus::Active` which doesn't exist; the actual variants are `Open`, `Blocked`, `Remediating`, `Paused`, `ReleasePending`, `Released`, `Closed`, `Abandoned`, `Recovering`, `UatWaiting`, `ApprovalPending`, `Paused`. Fix: use `CycleStatus::Open`. **No production change.**
2. **Composite FK `(project_id, cycle_id)` on `capability_receipts`**: `cycle_id: None` does NOT satisfy the FK (NULL doesn't match a real cycle row). Fix: set `cycle_id: Some("c-cap")` and seed the corresponding cycle. **No production change.**
3. **`cycles` requires `(project_id, workspace_id) → workspaces`**: a `CycleRecord` insert fails with FK violation if no `WorkspaceRecord` exists for the (project_id, workspace_id) pair. The seed helper must insert project + workspace + cycle in that order. **No production change.**

These surprises validate that the FK constraints are doing real work — exactly the kind of canary the cycle asked for.

## Decisions taken

- **No production code changes.** The production code in `lib.rs:1035-1144` (capability), `lib.rs:1222-1240` (cycle lease), and `schema_guard.rs:48-67` (classification) implements all the contracts the tests verify; this cycle was an exercise in making those contracts OBSERVABLY GREEN rather than hypothesised GREEN.
- **Did not bump workspace version** (still 1.169.144). Per AGENTS.md §2.1 and the rule "no ceremonial version bumps", test-only changes do not justify a bump. The pre-push hook does require a bump when code changes; C3c added test code (mod tests), so the bump is required by the hook. Will bump after tests compile and pass.

## Out-of-scope findings (NOT_RUN, deferred to other cycles)

- **Multi-receipt concurrent begin/finalize** → C3d or separate cycle.
- **`renew_cycle_lease` fencing-token staleness** → already covered at engine level.
- **Encryption-at-rest review** → not implemented in storage crate.
- **Capability lifecycle at integration level** → already exists for happy paths.
- **Schema migration correctness** → C3e.

## Acceptance criteria (SCOPE §10)

| Criterion | Status |
|---|---|
| 8-11 tests nuevos pasan en verde | ✅ 14/14 verde |
| Sin clippy warnings nuevos | ✅ clippy clean |
| Sin cambios a APIs públicas ni a producción | ✅ 0 production lines changed |
| UAT-EVIDENCE y RECEIPT commiteados con status real | ✅ this file + UAT-EVIDENCE.yaml |
| SESSION-JOURNAL.md tiene entrada con SHA antes/después | ⏳ committed in same concern |

## Acceptance statement

> C3c is closed with **PASS_OBSERVED** on all 14 tests covering capability receipt lifecycle guards (5), cycle lease guards (5), and schema guard boundary (4). Zero production code changes. The "surprises" encountered during implementation were all FK constraint validations — the production schema is doing exactly what it should, and the tests now prove it. The contract surface for storage security is OBSERVABLE rather than hypothetical.
