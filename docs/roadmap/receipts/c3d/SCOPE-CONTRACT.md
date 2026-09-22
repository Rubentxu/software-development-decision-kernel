# C3d — SCOPE-CONTRACT: Storage Performance baseline (T26)

**Cycle:** C3d (Resilience/Storage Performance)
**Baseline:** `main@e308ddb` (workspace v1.169.145, post-C3c)
**Opened:** 2026-09-22T09:05:00Z
**Owner:** orchestrator (direct execution)
**Authority basis:** AGENTS.md §3; ROADMAP.md §C3

## 1. Objective (falsable)

Produce a reproducible performance baseline for the three most-frequent storage operations:

- **T26-append**: end-to-end `SqliteEventStore::append` latency on a single stream, single-thread (mean over N=1000 events).
- **T26-lease**: `Storage::acquire_cycle_lease` + release latency on a fresh cycle (mean over N=1000 acquisitions).
- **T26-cas-put-get**: `FilesystemCas::put` + `get` round-trip latency (mean over N=1000 blobs of size 4 KiB).

The baseline MUST be expressed as concrete numbers (mean, p50, p99 in µs or ms) printed to stdout and asserted against reasonable upper bounds, with an explicit `#[ignore]` marker so the suite does not run by default (it's an opt-in performance measurement, not a correctness gate). The numbers are evidence; the assertions prevent regression on the order of 10×.

Both PASS_OBSERVED, FAIL_OBSERVED, or BLOCKED — never PASS_BY_CODE_READING.

## 2. Findings pre-investigación (OBSERVED, this session)

| # | Hallazgo | Archivo / línea | Implicación |
|---|---|---|---|
| F1 | No criterion dependency, no `benches/` directory in any crate, no `[[bench]]` config. | (none) | Bench harness absent. Cannot use criterion without ADR-level commitment. |
| F2 | `std::time::Instant` is the standard library timing primitive; `assert!` with measured mean is enough for an upper-bound sanity check. | (stdlib) | Lightweight path viable. |
| F3 | Existing tests use no `#[ignore]` markers actively; precedent shows they are acceptable for opt-in heavy tests (state_survives_restart.rs:12, workflow_run_restart_survival.rs:78). | `tests/state_survives_restart.rs` | `#[ignore]` is idiomatic in this codebase for "do not run by default". |
| F4 | C3b already exercised `event_store` end-to-end under contention (T22). The contention stress 5/5 ran in 1.17s–1.92s wall time for 40 events on 4 threads → ~25ms/op including barrier synchronization. Useful as a sanity threshold. | C3b C3b-U7 | Sanity threshold for T26-append under single-thread is much lower (sub-ms). |

## 3. Non-goals

- **No criterion / nightly / unstable features.** The performance baseline is captured via `std::time::Instant` and `#[ignore]` markers. Criterion is a future Performance Engineering concern.
- **No changes to production code** for performance reasons — this is measurement, not optimization.
- **No micro-optimization** even if measurements suggest slow paths. That's C5 territory.
- **No pprof / perf / flamegraphs.** Out of scope.
- **No bump de versión** (v1.169.145 estable hasta operator release). Note: per pre-push hook, code changes (test additions) DO require a bump; will bump after tests pass.

## 4. Surface area

- `crates/sddk-storage/src/event_store.rs` — append a `mod bench` with 1 `#[ignore]` test measuring append latency.
- `crates/sddk-storage/src/lib.rs` — append a `mod bench` with 1 `#[ignore]` test measuring lease acquire latency.
- `crates/sddk-storage/src/cas.rs` — append a `#[ignore]` test measuring put+get latency.
- Sin cambios a APIs públicas.

## 5. Test plan

### Fase 0 — baseline (no changes)
- `cargo test -p sddk-storage --lib` → expect 67/67 verde (post-C3c).

### Fase 1 — T26-append microbench
- `mod bench` in `event_store.rs`:
  - `bench_append_throughput` (`#[ignore]`): open in-memory, append 1000 events to one stream, capture per-event wall time, print mean/p50/p99, assert mean < 10 ms/event.
  - Expected: append is a single SQLite INSERT inside IMMEDIATE; mean should be sub-millisecond on modern hardware.

### Fase 2 — T26-lease microbench
- `mod bench` in `lib.rs`:
  - `bench_acquire_release_cycle` (`#[ignore]`): seed one cycle, then 1000× acquire-and-release a lease. Print mean. Assert mean < 5 ms/op.
  - Expected: each op is one BEGIN IMMEDIATE + two INSERTs + COMMIT.

### Fase 3 — T26-cas microbench
- `#[ignore]` test in `cas.rs::tests`:
  - `bench_put_get_4kib_roundtrip` (`#[ignore]`): put + get 1000× of a 4 KiB blob. Print mean. Assert mean < 5 ms/op.
  - Expected: SHA256 + 4 KiB write + 4 KiB read + verify = a few ms.

### Fase 4 — verification
- `cargo fmt --all`
- `cargo clippy -p sddk-storage --all-targets -- -D warnings` clean (the `#[ignore]` tests still compile).
- `cargo test -p sddk-storage --lib` → 67/67 still verde (the new tests are ignored by default).
- Run the `#[ignore]` suite manually: `cargo test -p sddk-storage --lib -- --ignored --nocapture`. Capture the printed numbers in RECEIPT.

## 6. STOP conditions

- If a benchmark prints numbers WAY above the threshold (e.g. mean append > 100ms) → STOP, report the regression, do not optimize in this cycle. Open follow-up C5 for performance.
- If `#[ignore]` markers cause CI failures → adjust marker placement or move to `tests/` directory.
- If `clippy` complains about `Instant::elapsed()` formatting → use `as_micros()`/`as_millis()` explicitly.

## 7. Deliverables

1. Commit funcional bajo `feat(c3d): T26 — storage performance baseline (append, lease, cas) with #[ignore] opt-in`.
2. 3 `#[ignore]` tests total, all observable (numbers printed to stdout).
3. `docs/roadmap/receipts/c3d/{SCOPE-CONTRACT,UAT-EVIDENCE.yaml,C3d-RECEIPT}.md` with the actual measured numbers.
4. Entrada de SESSION-JOURNAL.md.

## 8. Risks

- **R1**: `#[ignore]` tests don't run by default — easy to forget. Mitigate: README/RECEIPT will document the manual run command.
- **R2**: Numbers will vary across machines. Mitigate: only assert UPPER BOUNDS (so a fast machine passes, a slow machine doesn't break the assertion but is reported).
- **R3**: `tempfile` and `TempDir` setup overhead pollutes small-N measurements. Mitigate: pre-warm with 10 ops before measuring.

## 9. Out-of-scope for this WorkItem

- Criterion adoption.
- Multi-process / multi-host benchmarks.
- Flame graphs, perf, pprof.
- Optimization work — only measurement.
- C3e (schema resilience) — separate cycle.

## 10. Acceptance

Este WorkItem se considera cerrado cuando:

- [ ] 3 `#[ignore]` tests added with printed numbers.
- [ ] `cargo test -p sddk-storage --lib` → 67/67 still verde.
- [ ] `cargo test -p sddk-storage --lib -- --ignored --nocapture` produces observable numbers within reasonable bounds.
- [ ] Sin clippy warnings nuevos.
- [ ] UAT-EVIDENCE y RECEIPT commiteados con los números REALES medidos.
- [ ] SESSION-JOURNAL.md tiene entrada con SHA antes/después.

Si algún criterio falla → status `BLOCKED` con acción de recuperación; no PASS.
