# C3c — SCOPE-CONTRACT: Storage Security canarios (T23, T24, T25)

**Cycle:** C3c (Resilience/Storage Security)
**Baseline:** `main@edaea67` (workspace v1.169.144, post-C3b)
**Opened:** 2026-09-22T08:57:00Z
**Owner:** orchestrator (direct execution)
**Authority basis:** AGENTS.md §3; ROADMAP.md §C3

## 1. Objective (falsable)

Demonstrate via real test execution that:

- **T23 (Capability receipt lifecycle, fail-closed)**: `begin_capability_receipt` ONLY accepts `Status::Started`; `finalize_capability_receipt` ONLY accepts terminal states (`Succeeded`/`Failed`/`Unknown`); a terminal receipt cannot be re-finalized; the idempotency_key + request_hash contract is enforced.
- **T24 (Cycle lease guards, fail-closed)**: `acquire_cycle_lease` rejects `now_ms < 0` and `expires_at_ms <= now_ms`; rejects leases on missing cycles; rejects acquiring an unexpired lease; expired-lease re-acquire increments the fencing token; `renew_cycle_lease` rejects with stale fencing tokens.
- **T25 (Schema guard fail-closed)**: `assert_compatible` rejects `TooOld` and `NewerThanSupported` with typed `GuardError`; `classify` correctly enumerates all four variants. (schema_guard already has 5 tests; the new tests focus on the boundary at `MIN_SUPPORTED_SCHEMA_VERSION`.)

Both PASS_OBSERVED, FAIL_OBSERVED, or BLOCKED — never PASS_BY_CODE_READING.

## 2. Findings pre-investigación (OBSERVED, this session)

| # | Hallazgo | Archivo / línea | Implicación |
|---|---|---|---|
| F1 | `begin_capability_receipt` enforces `status == Started` (line 1035) but **no test directly exercises this contract**. The test would be: input with `status = Succeeded` → `Err(InvalidReceiptBegin)`. | `crates/sddk-storage/src/lib.rs:1035-1037` | T23 begin guard gap |
| F2 | `finalize_capability_receipt` accepts ONLY terminal states (line 1134) but **no test exercises the inverse** — `finalize(... Started)` → `Err(InvalidReceiptBegin)`. | `lib.rs:1134-1136` | T23 finalize non-terminal gap |
| F3 | `finalize_capability_receipt` rejects terminal-on-terminal (line 1140-1144, `TerminalReceipt`). Untested. | `lib.rs:1140-1144` | T23 re-finalize gap |
| F4 | Idempotency contract: same key + same request → return existing receipt; same key + different request → `IdempotencyConflict`. Documented at line 1026-1029. Untested at the unit level. | `lib.rs:1042-1058` | T23 idempotency gap |
| F5 | `acquire_cycle_lease` validates `now_ms < 0 || expires_at_ms <= now_ms` (line 1222) → `InvalidLease`. Untested. | `lib.rs:1222-1224` | T24 lease-time guard gap |
| F6 | `acquire_cycle_lease` calls `cycle_exists` first (line 1226-1228) → `not_found` if missing. Untested at unit level. | `lib.rs:1226-1228` | T24 lease-missing-cycle gap |
| F7 | `acquire_cycle_lease` rejects unexpired lease (line 1233-1239) → `LeaseConflict`. Untested. | `lib.rs:1233-1239` | T24 lease-conflict gap |
| F8 | Expired-lease re-acquire increments `fencing_token` (line 1240). Untested. | `lib.rs:1240` | T24 lease-fencing gap |
| F9 | `schema_guard` has 5 tests but lacks boundary tests at `MIN_SUPPORTED_SCHEMA_VERSION` (line 14) — `classify(MIN_SUPPORTED_SCHEMA_VERSION)` should yield `Exact` (if `== COMPILED`) or `Migratable { from: MIN, to: COMPILED }` (if `< COMPILED`). The boundary is not asserted. | `schema_guard.rs:14, 48-67` | T25 boundary gap |

## 3. Non-goals

- No production code changes to capability / lease / schema_guard unless tests reveal a real defect.
- No changes to schema_guard's classification logic (it's straightforward and well-tested at 5 tests).
- No changes to cycle / project APIs.
- No bump de versión (v1.169.144 estable hasta operator release).

## 4. Surface area

- `crates/sddk-storage/src/lib.rs` — añadir `mod tests` con 6-8 tests cubriendo T23+T24
- `crates/sddk-storage/src/schema_guard.rs` — añadir 2-3 tests al `mod tests` existente para T25 boundary
- Sin cambios a APIs públicas

## 5. Test plan (TDD-first incremental)

### Fase 0 — baseline (no changes)
- `cargo fmt --all -- --check && cargo clippy -p sddk-storage --all-targets -- -D warnings`
- `cargo test -p sddk-storage --lib` → expect 53/53 verde (post-C3b)

### Fase 1 — T23 Capability receipt canarios
- Añadir `mod capability_receipt_security_tests` en lib.rs con:
  - `begin_with_terminal_status_rejected` — input status = `Succeeded` → `Err(StorageError::InvalidReceiptBegin)`.
  - `finalize_with_started_status_rejected` — first begin (Started OK), then finalize with `Started` → `Err(StorageError::InvalidReceiptBegin)`.
  - `finalize_already_terminal_rejected_with_typed_guard` — begin, finalize(Succeeded), then finalize(Failed) → `Err(TerminalReceipt { .. })`.
  - `idempotent_retry_with_same_request_returns_existing_receipt` — begin, then begin with same key + same request → returns the original receipt (no double-insert).
  - `idempotent_retry_with_different_request_returns_conflict` — begin, then begin with same key + different request → `Err(IdempotencyConflict { .. })`.

### Fase 2 — T24 Cycle lease canarios
- Añadir `mod cycle_lease_security_tests` en lib.rs con:
  - `acquire_with_negative_now_ms_rejected` — `now_ms = -1` → `Err(InvalidLease)`.
  - `acquire_with_expires_before_now_rejected` — `now_ms = 100`, `expires_at_ms = 50` → `Err(InvalidLease)`.
  - `acquire_on_missing_cycle_rejected_with_not_found` — cycle_id not in DB → `Err(not_found)`.
  - `acquire_with_active_lease_rejected_with_typed_conflict` — acquire, then acquire again before expiry → `Err(LeaseConflict { .. })`.
  - `acquire_with_expired_lease_increments_fencing_token` — acquire at T0, expire, acquire at T1 → new lease with `fencing_token = 2`.

### Fase 3 — T25 Schema guard boundary
- Añadir al `mod tests` de schema_guard.rs:
  - `classify_at_min_supported_version` — `classify(MIN_SUPPORTED_SCHEMA_VERSION)` → either `Exact` or `Migratable { from: MIN, .. }`, depending on `MIN == COMPILED`. Test the actual relationship.
  - `assert_rejects_too_old_at_min_boundary` — `classify(MIN_SUPPORTED_SCHEMA_VERSION - 1)` → `TooOld`. Assert `assert_compatible` (via the manual error-path construction since `assert_compatible` takes `&Storage`) rejects with `GuardError::TooOldSchema`.
  - `classify_at_compiled_version_returns_exact` (already covered by `classify_exact_on_compiled_version` but worth keeping for boundary).

### Fase 4 — verification
- `cargo fmt --all`
- `cargo clippy -p sddk-storage --all-targets -- -D warnings` clean
- `cargo test -p sddk-storage --lib` → all green

## 6. STOP conditions

- If T23/T24/T25 reveal a real defect in production code → STOP, do not fix in this cycle, emit BLOCKED with diagnosis. Open follow-up SCOPE.
- If `clippy` introduces warnings in code I didn't touch → STOP, report upstream regression.
- If helper for building `CapabilityReceiptInput` requires extensive setup that exceeds 30 lines → simplify or use a smaller surface area.

## 7. Deliverables

1. Commit funcional bajo `feat(c3c): T23+T24+T25 — capability receipt and cycle lease security canarios + schema guard boundary`.
2. 8-11 new tests total (4-5 capability, 4-5 lease, 2-3 schema_guard), all green.
3. `docs/roadmap/receipts/c3c/{SCOPE-CONTRACT,UAT-EVIDENCE.yaml,C3c-RECEIPT}.md`.
4. Entrada de SESSION-JOURNAL.md.

## 8. Risks

- **R1**: `Storage::begin_capability_receipt` takes `&mut self`. Tests can use `Storage::open_in_memory()` cleanly.
- **R2**: cycle/lease helpers may require creating a project + cycle first; pre-conditions add 5-10 lines per test.
- **R3**: `assert_compatible` takes `&Storage`, not a version number. The error-path construction must be inline (not invoking `assert_compatible` directly). Acceptable.

## 9. Out-of-scope for this WorkItem

- Performance benchmarks (C3d).
- Schema migration correctness (C3e).
- Capability lifecycle tests at integration level (engine crate).
- Multi-receipt concurrency / multi-lease contention (separate cycle).
- Encryption-at-rest review (not implemented in storage crate).

## 10. Acceptance

Este WorkItem se considera cerrado cuando:

- [ ] 8-11 tests nuevos pasan en verde bajo `cargo test -p sddk-storage --lib`.
- [ ] Sin clippy warnings nuevos.
- [ ] Sin cambios a APIs públicas ni a producción (a menos que el RED demuestre un defecto real, en cuyo caso emitir BLOCKED).
- [ ] UAT-EVIDENCE y RECEIPT commiteados con status real (PASS_OBSERVED / BLOCKED).
- [ ] SESSION-JOURNAL.md tiene entrada con SHA antes/después.

Si algún criterio falla → status `BLOCKED` con acción de recuperación; no PASS.
