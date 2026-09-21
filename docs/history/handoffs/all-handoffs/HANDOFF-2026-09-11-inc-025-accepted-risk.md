# HANDOFF: INC-025 closure (accepted_risk)

**Date**: 2026-09-11
**INC**: INC-025-tainted-receipt-governance-signal
**Cycle**: `p-63676b11dc0ef88f/inc-hygiene-2026-09-11` (audit, not runtime)
**Path**: B-direct (vault hygiene, no code changes)
**Outcome**: INC-025 closed with `accepted_risk`

## Summary

INC-025 (`gate-implementation-complete-8e2d05d85556f5d4-1` — tainted receipt governance signal, P1 high, PROC cluster) was closed during the `inc-hygiene-2026-09-11` audit. The vault frontmatter was updated:

- `status: open → closed`
- `resolved_in_cycle: [[inc-hygiene-2026-09-11]]`
- `resolution: <accepted_risk explanation>`
- Resolution section filled with engine invariant citations

## Engine invariant cited (already implemented)

The post-hoc freshness invariant that covers the failure mode INC-025 warns about is already enforced by the engine at `crates/sddk-engine/src/lib.rs:1587`:

```rust
if receipt.plan_hash != expected_hash {
    return Err(EngineError::StaleGateReceipt {
        receipt_id: reference.receipt_id.clone(),
    });
}
```

where `expected_hash = self.plan_hash(&state_before.cycle_id, transition.id.as_str(), &state_before)` (`RuntimeState::plan_hash`, lib.rs:1084), and the receipt's `plan_hash` is computed at attestation time via `plan_hash_for_receipt(cycle_id, transition_id, state)` (lib.rs:1867).

## Why accepted_risk, not full resolution

INC-025's proposed Action was a **pre-flight *preventive*** check in the apply phase that compares attested workspace state against actual state before attesting any implementation-complete gate receipt. The engine already enforces a **post-hoc** freshness check on receipt *citation* (the receipt cannot advance a transition if its `plan_hash` does not match the current state). The pre-flight check would be defense-in-depth:

- No recurring failure mode observed since M7.9 closure (2026-08-27)
- Post-hoc invariant is the same logical guarantee (receipt attestation must reflect current state)
- Pre-flight adds an extra call site in `apply` for marginal benefit

## Audit log

Detailed audit log written to `.sddk/cycles/p-63676b11dc0ef88f/inc-hygiene-2026-09-11/audit-log.md` (excluded from git by `.gitignore: .sddk/cycles/`). Includes:

- Findings from initial scan (4 stale INCs reconciled in `271ce63`)
- INC-025 evidence review
- Triage decision rationale
- INC counts before/after
- Commits referenced

## INC counts after this handoff

| Status | Count | Delta |
|--------|-------|-------|
| open   | 30 | -1 |
| closed | 12 | +1 |
| resolved | 7 | 0 |
| tracked | 1 | 0 |

## Files changed

- `~/.sddk-knowledge/sddk-framework/incs/INC-025-tainted-receipt-governance-signal.md` — frontmatter + resolution (vault, not versioned in this repo)
- `.sddk/cycles/p-63676b11dc0ef88f/inc-hygiene-2026-09-11/audit-log.md` — new (runtime storage, ignored by git)

No code changes. No Cargo.toml changes. No version bump.

## Why no release commit

Per AGENTS.md §2.1 ("una concernencia por commit") and §8 (canonical release flow), the hygiene + INC closure does not warrant a release. The `chore(release): bump version` subject in `bc8f168` was a ceremonial marker combining INC-024 refactor with the hook-required marker; INC-025 closure is even smaller (frontmatter only) and does not need a new release tag. The pre-push hook (`githooks/pre-push`, `INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION`) would block push to main without a release marker commit, so this handoff does not push. The vault file change is durable in the knowledge directory and does not require git.
