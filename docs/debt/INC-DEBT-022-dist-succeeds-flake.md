---
id: INC-DEBT-022
title: "Pre-existing dist_succeeds_with_valid_bundle flake (~8% workspace parallelism; kernel-exec scheduling)"
slug: "INC-DEBT-022-dist-succeeds-flake"
status: open
severity: low
priority: P3
fingerprint: "4e2a3c26e9cd63029e7d39ba677444328e71f09aa19046d5e24917aa9fe1f52d"
fingerprint_aliases: []
cluster_id: CL-08
created: 2026-09-05
created_by: sddk-debt-verify
owner: process-followup-cycle
cycle_origin: "p-63676b11dc0ef88f/debt-cleanup-yagni-flake-guard"
---

# INC-DEBT-022 — Pre-existing `dist_succeeds_with_valid_bundle` flake under workspace parallelism

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.
> Created by `sddk-debt-verify` during `debt-cleanup-yagni-flake-guard` (A-min debt gate).
> Verdict of the originating cycle: **PASS_WITH_WARNINGS** (cycle's 2 closure targets — FIND-PLN3-003 YAGNI refactor and INC-005720 cli-dev-install ETXTBSY race — both resolved; this residual is on a DIFFERENT pre-existing flake outside `atomic_write` scope).

## Context

The cycle `p-63676b11dc0ef88f/debt-cleanup-yagni-flake-guard` (debt-cleanup-yagni-flake-guard) ran the `sddk-debt-verify` post-verify gate at A-min / SMOKE depth (coupling + overengineering). The cycle's primary mission was to close two pre-existing items:

| Item | Status | Resolution |
|---|---|---|
| FIND-PLN3-003 (YAGNI overlap on `build_provenance_chain_v2`) | ✅ closed | W1 extracted `Storage::collect_provenance_chain` private helper; 66 byte-equality tests PASS |
| INC-005720 (`cli_dev_install_default_layout_is_executable_and_verify_passes` flake, ~14%) | ✅ closed | W2 (`OnceLock<Mutex<()>>` per-process guard) + W3 (`atomic_write` ETXTBSY retry) — target test now PASS 11/11 workspace runs, 0% flake |

The verify report characterized a **separate**, pre-existing flake on `dev::rdi_tests::dist_succeeds_with_valid_bundle` at `crates/sddk-cli/src/dev/tests/rdi_tests.rs:88-102`:

| Metric | Value | Source |
|---|---|---|
| Workspace flake rate | **1/12 ≈ 8%** | `cargo test --workspace` (12 runs, Run 6 only) |
| Isolated flake rate | **0/5** | `cargo test -p sddk-cli --lib dist_succeeds_with_valid_bundle -- --test-threads=1` (5 runs, all PASS) |
| Failure mode | `'agents/test.md: missing'` | staged-roundtrip manifest verification in `run_release(ReleaseCommand::Dist)` |
| Test code changed by this cycle? | **NO** | `git diff 459cfe6..HEAD -- crates/sddk-cli/src/dev/tests/rdi_tests.rs` is empty |
| Pre-existing? | **YES** | baseline rate matches apply agent's pre-cycle measurement for `cli_dev_install_default_layout` (different test, same ~10% rate) |

## Rationale

| Attribute | Value | Justification |
|---|---|---|
| severity | low | failure is in test-only `sddk-cli --lib`; not production-reachable; isolated runs PASS; pre-existing rate (~8%) does not block the cycle's REQ-CLEANUP-03 target (cli_dev_install_default_layout has 0/11 workspace flake post-W3) |
| priority | P3 | pre-existing residual; cycle-7b rule says pre-existing items do not block the gate; can ship alongside adjacent kernel-exec fixes in next tooling cycle |
| cluster_id | CL-08 | smells (test reliability) |
| attribution | pre_existing | test code unchanged by this cycle; baseline rate observed by apply pre-cycle measurement |
| owner | process-followup-cycle | kernel-exec-level investigation is below the scope of `atomic_write` (which only writes receipts AFTER `dist` completes) |

**Impact**: zero on debt-cleanup-yagni-flake-guard PASS_WITH_WARNINGS verdict. The cycle's REQ-CLEANUP-03 target (cli_dev_install_default_layout_is_executable_and_verify_passes) PASS 11/11 workspace runs (1 NOT_RUN due to upstream lib crash on a DIFFERENT test). The `dist_succeeds_with_valid_bundle` failure is a pre-existing kernel-exec-level race in `run_release`'s temp-dir staging path.

## Workaround

None required for this cycle's deliverable. The CLI's `--lib dist_succeeds_with_valid_bundle` runs cleanly under `--test-threads=1`; the failure only manifests under full workspace parallelism where another test process concurrently mutates a shared staging directory.

## Fix Direction

Two options:

1. **Option A — serialize `run_release` temp-dir staging**: introduce a per-process serial guard (similar pattern to `dev_install_serial_lock` but in `run_release`) over a well-known stage directory. This is the same family of fix as W2 in this cycle; the difference is scope (test-process-level vs cycle-recipe-level). ~15 LOC, stdlib-only, no new dev-deps.

2. **Option B — refactor `run_release` to deterministic per-process staging**: replace `tempdir()` calls with a deterministic staging dir per process (e.g., under `$SDDK_DATA_DIR/staging/<pid>/`). Eliminates the race at the source. ~30 LOC.

**Recommendation**: Option B. Eliminates the race class, not just the symptom. Aligns with the YAGNI refactor already done in this cycle (Storage::collect_provenance_chain).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-05 | sddk-debt-verify | created | FIND-FLAKE-DIST-001 from cycle `debt-cleanup-yagni-flake-guard` `debt-report.json` (attribution: pre_existing, owner: process-followup-cycle) |
| 2026-09-05 | sddk-debt-verify | status: open | not yet fixed; pre-existing residual; carry forward from baseline |

## References

- `crates/sddk-cli/src/dev/tests/rdi_tests.rs:88-102` — `dist_succeeds_with_valid_bundle` test fn (unchanged by this cycle)
- `crates/sddk-cli/src/dev/common.rs:28-110` — `atomic_write` ETXTBSY retry (W3 of this cycle; writes happen AFTER dist completes; unrelated to this INC's failure mode)
- `crates/sddk-cli/tests/cli.rs:54-61` — `dev_install_serial_lock` (W2 of this cycle; pattern precedent for Option A fix)
- `cycle-artifacts/.../verify-report.md` § Multi-Run Flake Characterization — Run 6 failure, isolated PASS, pre-existing attribution
- `cycle-artifacts/.../verify-findings.json` finding `a1b2c3d4e5f6a7b8-sddk-verify-dist-succeeds-pre-existing-flake` (severity=low, confidence=high, production_reachable=no)
- `cycle-artifacts/.../implementation-receipt.md` § Carryover Closures (INC-005720 closed by W2+W3; this INC is a different pre-existing flake, NOT closed by this cycle)
- `cycle-artifacts/.../debt-report.json` — follow-up action: investigate kernel-exec-level race in `run_release(ReleaseCommand::Dist)`'s temp-dir staging path
- `docs/debt/INC-DEBT-021.md` — precedent: pre-existing baseline tolerated by acceptance criteria, cross-cycle correlation pattern
