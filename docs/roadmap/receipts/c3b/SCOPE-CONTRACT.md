# C3b — SCOPE-CONTRACT: Storage adversarial (T21, T22)

**Cycle:** C3b (Resilience/Storage adversarial)
**Baseline:** `main@efe7c44` (workspace v1.169.143, post-C3a)
**Opened:** 2026-09-22T08:35:00Z
**Owner:** orchestrator (direct execution; subagent path proved unreliable due to usage limits)
**Authority basis:** AGENTS.md §3; ROADMAP.md §C3 + §3

## 1. Objective (falsable)

Demonstrate via real test execution that:

- **T21 (Crash/reopen)**: a CAS-blob corruption on disk is detected and refused on read; an event-store reopen after process-restart yields the same chain sequence and chain_hash continuity as before; re-append of the same event_id is idempotent and returns the original row without allocating a new sequence.
- **T22 (Contención SQLite)**: two threads concurrently calling `EventStore::append` against the same `SqliteEventStore::open_in_memory()` (or against a single on-disk ledger.sqlite under WAL+busy_timeout) never lose an event: each event is appended exactly once and the resulting sequence numbers are contiguous 1..N with no gaps and no duplicates.

Both with observed PASS, observed FAIL, or observed BLOCKED — never PASS_BY_CODE_READING.

## 2. Findings pre-investigación (OBSERVED, this session)

| # | Hallazgo | Archivo / línea | Implicación |
|---|---|---|---|
| F1 | `cas.rs` has 5 tests; **no test exercises the corrupt-file path** (line 109-121: `get()` verifies hash on read and returns `HashMismatch`). The code path is implemented but unverified. | `crates/sddk-storage/src/cas.rs:115-121` | T21 corrupt-CAS gap |
| F2 | `event_store.rs` has **0 unit tests** (grep `fn test\|#\[test\]` returns empty). The module is 809 lines covering append, chain_hash, idempotency, project bootstrap, snapshots, projections. Untested at the unit level. | `crates/sddk-storage/src/event_store.rs:1-809` | T21 + T22 wide gap |
| F3 | `event_store.append` uses `TransactionBehavior::Immediate` (line 162) and `busy_timeout(5s)` + `journal_mode=WAL` in `open()` (lines 50-55). IMMEDIATE-on-write is exactly the surface C3 warns about for contention. Untested. | `event_store.rs:46-58, 162` | T22 contention gap |
| F4 | `event_store.append` line 251-262: `INSERT OR IGNORE` on `event_id` provides idempotency for re-append; comment at line 248-250 says "a DIFFERENT event colliding with a stored event_id is a real integrity failure". The two cases (same event retry vs. conflicting event) must be tested separately. | `event_store.rs:248-262` | T21 idempotency gap |
| F5 | `event_store` reopens: `open()` is idempotent (line 46-59) — apply migrations on every open. We need a test that opens, appends N events, drops the connection, reopens, asserts chain continuity. | `event_store.rs:46-58` | T21 reopen gap |
| F6 | `backlog_store.rs` has 15+ tests but the prior flake (session-10 cycle, INC-RELEASE-TAG-FIX) was in concurrent `open_owned_*` — already closed at v1.169.141. Out of scope for this cycle. | (history) | noted, not in scope |

## 3. Non-goals

- No production code changes to CAS or event_store unless tests reveal a real defect (and even then, fix is its own SCOPE).
- No changes to backlog_store (already covered + de-flaked in v1.169.141).
- No changes to migrations, schema_guard, or fork_store.
- No changes to ports/traits (CasPort, EventStore, SnapshotPort).
- No bump de versión (v1.169.143 estable hasta operator release).

## 4. Surface area

- `crates/sddk-storage/src/cas.rs` — añadir 2-3 tests al `mod tests`
- `crates/sddk-storage/src/event_store.rs` — añadir `mod tests` nuevo (no existe) con 4-6 tests
- Sin cambios a APIs públicas

## 5. Test plan (TDD-first incremental)

### Fase 0 — baseline (no changes)
- `cargo fmt --all -- --check && cargo clippy -p sddk-storage --all-targets -- -D warnings`
- `cargo test -p sddk-storage --lib cas` → expect 5/5 verde (baseline)
- `cargo test -p sddk-storage --lib event_store` → expect 0 tests run (no `mod tests` exists)

### Fase 1 — T21 corrupt CAS test (RED → GREEN, no production change expected)
- **RED**: Añadir `#[test] fn get_detects_corrupted_blob_on_disk()` en `cas.rs::tests`. Pattern: temp_cas(); put content `b"original"`; compute its hash; manually overwrite the file at `hash_path` with bytes `b"corrupted"` (a different content); call `cas.get(&hash)`; assert `Err(CasError::HashMismatch { .. })`.
- **GREEN**: Run; expected to PASS without any production change because `cas.rs:115-121` already verifies hash. If it FAILS, that's a real defect — STOP and emit BLOCKED with diagnosis.

### Fase 2 — T21 idempotency tests (event_store)
- Crear `mod tests` al final de `event_store.rs`. Helpers: `fn make_envelope(project, stream, event_id, content) -> EventEnvelopeV1` con chain hashes correctos.
- **T21-idem-same**: append envelope E; append same E again; assert `EventAppended::sequence` returned both times is identical (1 on second call, not 2); assert DB has exactly one row.
- **T21-idem-different-content-same-id**: append E1 with content_hash X; build envelope E2 with same `event_id` but different payload; assert append returns `Err` with the typed guard (NOT silent overwrite). Verifies the comment at `event_store.rs:248-250`.

### Fase 3 — T21 reopen test (event_store)
- **T21-reopen-chain**: `open_in_memory()` (or temp dir), append N=5 envelopes, drop, reopen same path, read all 5 back, assert chain_hash of last matches what was computed before drop, assert sequences are 1..5 contiguously.

### Fase 4 — T22 contention test (event_store)
- **T22-immediate-no-event-loss**: Use `Arc<Mutex<SqliteEventStore>>` (or `Arc<SqliteEventStore>` if Sync). N threads (say 4) each append M distinct events (say 25 each) to distinct stream_ids via `Arc<Barrier>` synchronization. After all join, query `events_v1` count → must equal `N*M`. Assert no duplicates by event_id.
- **T22-immediate-no-event-loss-disk**: Same pattern but using a tempdir-backed `open()` (WAL mode). N=2 threads, M=10 each. Assert count and uniqueness.

### Fase 5 — verification (no `cargo test --workspace` per scope discipline)
- `cargo fmt --all`
- `cargo clippy -p sddk-storage --all-targets -- -D warnings` clean
- `cargo test -p sddk-storage --lib cas` + `event_store` → all green
- Re-run T22 contention 5x for flake check

## 6. STOP conditions

- If `T21 corrupt CAS test` FAILS, the production code has a real defect — STOP, do not fix the production code in this cycle. Emit BLOCKED with full diagnostic (expected vs actual behavior, test output). Open a follow-up SCOPE for the fix.
- If `T22 contention test` is flaky (>1 failure in 5 stress runs), STOP. Report the flakiness honestly as a finding. Do not add `sleep`/`yield_now` to mask it.
- If `clippy` introduces warnings in code I didn't touch → STOP, report upstream regression.
- If `SqliteEventStore` is not `Sync` and the contention test cannot be expressed as multiple `&mut self` per-thread → emit BLOCKED with note "EventStore trait requires &mut self; T22 needs architectural support (per-thread clones via open_path of the same file)". Do NOT redesign.

## 7. Deliverables

1. Commit funcional bajo `feat(c3b): T21+T22 — CAS corruption detection, event idempotency, reopen, and IMMEDIATE contention tests`.
2. 5-7 new tests total (2-3 in CAS, 4-5 in event_store), all green.
3. `docs/roadmap/receipts/c3b/{SCOPE-CONTRACT,UAT-EVIDENCE.yaml,C3b-RECEIPT}.md`.
4. Entrada de SESSION-JOURNAL.md.

## 8. Risks

- **R1**: `EventStore::append` takes `&mut self`, so the contention test needs `Arc<Mutex<…>>` not bare Arc. This serializes appends at the Rust level, defeating the purpose of testing SQLite contention. **Mitigation**: use multiple `Connection` handles to the same file via `open_path` of the same tempdir path, each in its own thread. Verify rusqlite `Connection: Send` (it is).
- **R2**: env-store tests under high contention may exceed `busy_timeout(5s)` and return `Database("begin tx: ...")`. **Mitigation**: 2 threads × 10 events each at most, well below timeout threshold.
- **R3**: `EventEnvelopeV1::compute_content_hash` signature: verify it accepts the envelope shape I'm constructing in the test helper; if not, find the canonical constructor.
- **R4**: `event_store` exposes 800+ lines; ensure my test imports don't shadow production names (e.g., `EventEnvelopeV1`).

## 9. Out-of-scope for this WorkItem

- Performance benchmarks (C3d).
- Security canarios en Storage (C3c).
- Schema resilience (C3e).
- Changes to `backlog_store` (already covered).
- Migration correctness tests (separate cycle).
- Multi-process contention (single-process multi-thread only).

## 10. Acceptance

Este WorkItem se considera cerrado cuando:

- [ ] 5-7 tests nuevos pasan en verde bajo `cargo test -p sddk-storage --lib cas event_store` (cas + event_store modules).
- [ ] T22 contention stress 5/5 PASS sin flake.
- [ ] Sin clippy warnings nuevos.
- [ ] Sin cambios a APIs públicas ni a producción (a menos que el RED demuestre un defecto real, en cuyo caso emitir BLOCKED).
- [ ] UAT-EVIDENCE y RECEIPT commiteados con status real (PASS_OBSERVED / BLOCKED).
- [ ] SESSION-JOURNAL.md tiene entrada con SHA antes/después.

Si algún criterio falla → status `BLOCKED` con acción de recuperación; no PASS.
