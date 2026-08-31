---
id: INC-DEBT-007-preexisting-clippy-sddk-cli
title: "Preexisting workspace clippy debt in crates/sddk-cli/ (cycle-30 carry-forward)"
status: closed
severity: medium
priority: P2
fingerprint: "1d12f33bf9490185"
fingerprint_aliases: ["1d12f33bf9490185e96067ff05ea8f9164e3d73773a3355c8e31d8b4348f9e1b"]
cluster_id: CL-01
created: 2026-08-25
created_by: sddk-debt-verify (cycle-30, A-min smoke)
owner: orchestrator
cycle_source: p-52b95ef55999f9de/kernel-cycle-30-map-source-context-isolation-cross-tick-replay
finding_ref: FIND-15935401
attribution: pre_existing
base_sha_when_discovered: aac9920 (cycle-29 housekeeping)
---

# INC-DEBT-007 — Preexisting workspace clippy debt in crates/sddk-cli/

> Durable cross-cycle record. Created from FIND-15935401 in cycle-30 debt-report.
> See ADR-0047 §3.2.

## Context

The workspace-level `cargo clippy --workspace --all-targets -- -D errors` gate
fails with **7 errors** in `crates/sddk-cli/` that predate cycle-30. The cycle-30
diff is engine-only (only `crates/sddk-engine/src/operator.rs` + tests + ADR +
handoff), so cycle-30 introduces **zero new clippy regressions**. The engine
scope (`cargo clippy -p sddk-engine --all-targets -- -D errors`) is GREEN.

### Error inventory (all under `crates/sddk-cli/`)

| File | Lines | Lint |
|---|---|---|
| `crates/sddk-cli/src/json.rs` | 211 / 220 / 229 | `this if statement can be collapsed` (×3) |
| `crates/sddk-cli/src/inventory_cycle.rs` | 236 | `manual implementation of ok` |
| `crates/sddk-cli/tests/reconcile_tests.rs` | 223 / 735 | `useless use of format!` (×2) |
| `crates/sddk-cli/tests/reconcile_tests.rs` | 593 | `field assignment outside of initializer` |

### Baseline confirmation

Verified pre-existing on base commit `aac9920` (cycle-29 housekeeping) by
reverting `crates/sddk-cli/` to that SHA and re-running `cargo clippy`. The
identical errors reproduce.

## Rationale

- **Severity = medium**: clippy is a workspace-level gate. It blocks a future
  release tag. Cycle-30 does NOT block on it because (a) the gate is
  engine-scope-isolated for cycle-30 and (b) the debt is pre-existing on the
  base commit. The cycle-30 diff has no `sddk-cli/` changes, so cycle-30 itself
  is not regressing.
- **Priority = P2**: remediation candidate for cycle-32+ or earlier if a
  dedup is found. Not a current-cycle obligation.
- **Attribution = pre_existing**: discovered on `aac9920`; cycle-30 only
  rediscovered it because the workspace-level gate is broader than the
  engine-scope gate.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-25 | sddk-debt-verify (cycle-30) | created | FIND-15935401 from debt-report.json cycle-30 |
| 2026-08-25 | sddk-apply (cycle-33) | status: open → closed | 8 hunks landed on feat branch; cargo clippy --workspace exits 0 |

## References

- `crates/sddk-cli/src/json.rs` L211, L220, L229
- `crates/sddk-cli/src/inventory_cycle.rs` L236
- `crates/sddk-cli/tests/reconcile_tests.rs` L223, L593, L735
- `docs/cycle-artifacts/kernel-cycle-30-map-source-context-isolation-cross-tick-replay/debt-report.json` → `findings[3]`
- `docs/cycle-artifacts/kernel-cycle-30-map-source-context-isolation-cross-tick-replay/verify-report.md` §"Workspace clippy details (preexisting, out of scope)"

> Filled by `sddk-archive` (cycle-8+); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.