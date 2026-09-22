# C3d-RECEIPT — Storage Performance baseline (T26)

**Cycle:** C3d
**WorkItem:** T26 — append / cas / lease microbenchmarks
**Baseline:** `main@e308ddb` (workspace v1.169.145)
**HEAD post-cycle:** committed in same concern
**Date closed:** 2026-09-22T09:12:00Z
**Owner:** orchestrator (direct execution)
**Status:** **PASS_OBSERVED**

## Inputs

- **SCOPE-CONTRACT**: `docs/roadmap/receipts/c3d/SCOPE-CONTRACT.md`
- **Findings pre-investigación** (Section 2 of SCOPE): F1 (no criterion), F2 (Instant available), F3 (`#[ignore]` idiomatic), F4 (sanity thresholds from C3b).

## Test additions (delta from `main@e308ddb`)

| File | Tests added | Lines |
|---|---|---|
| `crates/sddk-storage/src/event_store.rs` | `bench_append_throughput` (`#[ignore]`) | +85 |
| `crates/sddk-storage/src/lib.rs` | `bench_acquire_release_lease` (`#[ignore]`) | +145 |
| `crates/sddk-storage/src/cas.rs` | `bench_put_get_4kib_roundtrip` (`#[ignore]`) | +45 |

Total: **3 new benchmarks** (all `#[ignore]`-marked, opt-in), **+275 lines** of test code, **0 lines** of production code change.

## Evidence (real commands, real output)

### Baseline (pre-cycle)
```
cargo test -p sddk-storage --lib → 67/67 ok (post-C3c baseline)
cargo fmt --all -- --check → ok
cargo clippy -p sddk-storage --all-targets -- -D warnings → ok
```

### Post-cycle (default suite still passes)
```
cargo test -p sddk-storage --lib → 67 passed, 0 failed, 3 ignored
```

### Bench opt-in run
```
cargo test -p sddk-storage --lib -- --ignored --nocapture

T26-append over N=1000: mean=339 µs, p50=333 µs, p99=410 µs
T26-cas put+get 4 KiB over N=1000: mean=439 µs, p50=436 µs, p99=478 µs
T26-lease acquire+release over N=1000: mean=14959 µs, p50=14556 µs, p99=21956 µs

test result: ok. 3 passed; 0 failed; 0 ignored
```

### Lease bench variance check (3 runs)
```
Run 1: mean=34960 µs, p50=32246 µs, p99=60581 µs
Run 2: mean=16125 µs, p50=14213 µs, p99=44371 µs
Run 3: mean=16239 µs, p50=14327 µs, p99=46912 µs
```
Variance is real; threshold set to 100 ms (loose) to absorb CI noise.

## Surprises during execution (real, not pre-planned)

1. **Tight threshold on lease**: my first threshold (5 ms) was too optimistic; the measured mean was 15 ms. Adjusted to 100 ms (loose, with documented rationale). **No production change.**
2. **Helper visibility**: `envelope_with_event_id` is private to `event_store::tests`. Solution: local mirror `bench_envelope` inside `mod bench`. **No production change.**
3. **Mod tests scope**: my first CAS bench landed AFTER the closing `}` of `mod tests`. Solution: moved inside the existing `mod tests`. **No production change.**

## Decisions taken

- **No criterion adoption.** Criterion is the gold-standard harness, but adopting it requires a workspace-level dev-dep change plus `[[bench]]` config in `Cargo.toml`. That's an ADR-level decision and outside the AUTO scope. The `#[ignore]` opt-in with `Instant` gives reproducible measurement at zero-cost on the production path.
- **Loose thresholds, not tight ones.** CI noise is real; a tight threshold would produce flaky bench failures that hide real regressions. The 100 ms lease threshold catches >5× regressions and tolerates 2× variance.
- **Benchmarks live in `#[cfg(test)] mod bench`** rather than in a separate `benches/` directory. Reason: no criterion; using `#[ignore]` + `cargo test -- --ignored` is the simplest opt-in.

## Out-of-scope findings (NOT_RUN, deferred)

- **Criterion adoption** — out of scope (would require ADR + operator approval for new dev-dep).
- **Multi-process / multi-host benchmarks** — out of scope.
- **Flame graphs, perf, pprof** — out of scope.
- **Optimization work** — measurement only; this cycle deliberately does NOT touch production code.
- **C3e schema resilience** — separate cycle.

## Acceptance criteria (SCOPE §10)

| Criterion | Status |
|---|---|
| 3 `#[ignore]` tests added with printed numbers | ✅ 3/3 |
| `cargo test -p sddk-storage --lib` → 67/67 still verde | ✅ 67/67 + 3 ignored |
| `-- --ignored` produces observable numbers within reasonable bounds | ✅ 3/3 with documented means |
| Sin clippy warnings nuevos | ✅ clippy clean |
| UAT-EVIDENCE y RECEIPT commiteados con números REALES medidos | ✅ this file + UAT-EVIDENCE.yaml |
| SESSION-JOURNAL.md tiene entrada con SHA antes/después | ⏳ committed in same concern |

## Acceptance statement

> C3d is closed with **PASS_OBSERVED** on all 3 microbenchmarks. The numbers are observable and reproducible via `cargo test -p sddk-storage --lib -- --ignored --nocapture`. Zero production code changes. The lease bench's high variance is documented in code as a known characteristic, not a regression. The thresholds are loose enough to tolerate CI noise but tight enough to catch catastrophic regressions (>5×). Tight perf gating is deferred to a criterion-engineering cycle.
