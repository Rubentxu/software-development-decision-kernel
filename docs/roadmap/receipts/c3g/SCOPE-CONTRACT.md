# C3g — SCOPE-CONTRACT: Performance budget for Base profile

**Cycle:** C3g (Performance budget — Base only, not Static/Runtime)
**Baseline:** `main@a89dde6` (workspace v1.169.150)
**Opened:** 2026-09-22T10:02:00Z
**Owner:** orchestrator (direct execution; scope < 100 lines test code)
**Authority basis:** AGENTS.md §3 (gates preauthorized); ROADMAP §C3 ("presupuesto medible de p95 y recursos en tres escenarios fijados (Base, static, runtime); baseline antes de optimizar"); no provider artifacts required for Base; no ADR required (measurement only).

## 1. Objective (falsable)

Measure wall-clock latency (p50, p95, max) and peak memory (RSS) of three representative
Base-profile CLI commands on the current HEAD, with N=100 runs each, in a stable environment
(single CPU affinity, no concurrent workload), and document the observed budget.

**Exit criterion:** A `BENCH-RESULTS.yaml` file with three measured scenarios, each with
p50/p95/max wall-clock and peak RSS, reproducible via the benchmark harness, and a
narrative explaining what the budget implies.

**NOT in this cycle**: optimization, criterion adoption, multi-process, multi-host, flame
graphs, perf, pprof, anything that touches production code.

## 2. Findings (OBSERVED, session-11 reconnaissance)

| # | Finding | Command | Implication |
|---|---|---|---|
| F1 | `sddk version` takes ~8 ms wall-clock | smoke check | Trivial; no budget needed beyond "fast" |
| F2 | `sddk agent-help` takes ~7 ms | smoke check | Trivial |
| F3 | `sddk validate --root . --scope .` takes ~7 ms | smoke check | Trivial |
| F4 | `sddk cycle status` (no active cycle) takes ~6 ms | smoke check | Trivial; with cycle, ~40 ms |
| F5 | `sddk lint` takes ~184 ms | smoke check | **The expensive one** — runs fmt + clippy + test lint |
| F6 | `sddk metrics --help` takes ~6 ms | smoke check | Trivial |
| F7 | `sddk why --help` takes ~7 ms | smoke check | Trivial |

**Selected scenarios** (representative of Base profile real usage):
- **S1 — `sddk version`**: instant read of resolved framework version. p95 < 50 ms.
- **S2 — `sddk cycle status` (no active cycle)**: typical pre-flight check before
  starting a cycle. p95 < 100 ms.
- **S3 — `sddk lint`**: the validation command that exercises fmt/clippy/test-lint
  checks. p95 < 500 ms.

These scenarios cover the spectrum: pure metadata (S1), cycle-state read (S2), and
the only non-trivial Base command (S3).

## 3. Non-goals

- No release cut.
- No change to Authority/Storage boundaries.
- No new tests beyond the bench harness.
- No optimization (baseline only per ROADMAP §C3).
- No Static-Enhanced / Runtime-Enhanced scenarios (providers absent).
- No criterion adoption (out of scope per C3d §"Decisions taken").

## 4. Surface area

- `crates/sddk-cli/benches/perf_budget_base.rs` — new bench harness (test code, `#[ignore]`).
  Lives in the existing sddk-cli crate; opt-in via `cargo test -- --ignored`.
- `docs/roadmap/receipts/c3g/{SCOPE-CONTRACT.md, UAT-EVIDENCE.yaml, RECEIPT.md, BENCH-RESULTS.yaml}` — durable artifacts.

No production code changes.

## 5. Test plan (scoped)

| Phase | Action | Expected |
|---|---|---|
| T0 | `cargo fmt --all -- --check` | clean |
| T1 | `cargo build --release -p sddk-cli --bin sddk` | build OK; binary at `target/release/sddk` (or cargo-targets override) |
| T2 | `cargo test -p sddk-cli --lib perf_budget_base -- --ignored --nocapture` | 3 scenarios × N=100 each, observable numbers |
| T3 | `cargo test -p sddk-cli --lib` | existing tests still pass (no regression) |
| T4 | `cargo clippy -p sddk-cli --all-targets -- -D warnings` | clean |

The bench harness prints p50/p95/max wall-clock and peak RSS for each scenario.

## 6. STOP conditions

- If any scenario fails to run (segfault, panic, missing binary) → STOP, report error.
- If p95 of any scenario exceeds the soft target by >10x → STOP, report anomaly.
- If existing tests break → STOP, revert bench harness (production code untouched anyway).

## 7. Deliverables

1. Commit `test(c3g): perf budget harness for Base profile — 3 scenarios × N=100`.
2. `docs/roadmap/receipts/c3g/BENCH-RESULTS.yaml` with observed numbers (REAL commands).
3. `docs/roadmap/receipts/c3g/UAT-EVIDENCE.yaml` with PASS_OBSERVED.
4. `docs/roadmap/receipts/c3g/RECEIPT.md` documenting cycle close.
5. SESSION-JOURNAL.md entry with SHA before/after.
6. CURRENT.md / STATE.yaml reconcile.

## 8. Risks

- **R1**: Variance is high on shared/loaded systems. Mitigated: pin to a single CPU,
  drop caches between runs, document the environment in BENCH-RESULTS.yaml.
- **R2**: `sddk lint` may invoke `cargo` subprocess which has its own variance. Mitigated:
  document the observed variance; do NOT compare across hosts.
- **R3**: The harness introduces a `#[ignore]` test that depends on a built release binary.
  Mitigated: the harness builds the binary itself if absent; T1 builds it before T2.

## 9. Out-of-scope

- Static-Enhanced perf (requires CogniCode).
- Runtime-Enhanced perf (requires Chronos).
- Criterion adoption (separate ADR cycle).
- Multi-process, multi-host, flame graphs, perf, pprof.
- Optimization of any kind.

## 10. Acceptance

- [ ] `cargo build --release -p sddk-cli --bin sddk` → clean.
- [ ] `cargo test -p sddk-cli --lib perf_budget_base -- --ignored --nocapture` →
  observable numbers for all 3 scenarios × N=100.
- [ ] `cargo test -p sddk-cli --lib` → existing tests pass.
- [ ] `cargo clippy -p sddk-cli --all-targets -- -D warnings` → clean.
- [ ] BENCH-RESULTS.yaml has real p50/p95/max wall-clock and peak RSS for each scenario.
- [ ] No production code change (test code only).
- [ ] No release / version bump (workspace 1.169.150 stays; this is a measurement cycle).
