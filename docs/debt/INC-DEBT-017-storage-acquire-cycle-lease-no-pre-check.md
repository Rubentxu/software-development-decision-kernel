---
id: INC-DEBT-017
title: "sddk-storage acquire_cycle_lease lacks pre-existence check (FK fires STORAGE_DATABASE for own-project missing cycles)"
status: resolved
severity: medium
priority: P2
fingerprint: "6b4c9a5f3d8e2c1a4b7d9e0f5a3c8b1e"
fingerprint_aliases: []
cluster_id: CL-NN
created: 2026-09-01
created_by: sddk-verify (p-63676b11dc0ef88f/gap6-foreign-cycle-typed-error)
resolved_by: p-63676b11dc0ef88f/storage-cycle-lease-pre-existence-check
resolved_at: 2026-09-01
owner: unassigned
---

# INC-DEBT-017 — sddk-storage acquire_cycle_lease lacks pre-existence check (FK fires STORAGE_DATABASE for own-project missing cycles)

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`sddk cycle lock acquire --cycle <own-project>/<missing-cycle> --owner o` returns
`error[STORAGE_DATABASE]: sqlite storage error: FOREIGN KEY constraint failed`
when the cycle row does not exist in the `cycles` table (same project prefix,
but the cycle was never created).

The cycle's project prefix is correct (matches the workspace's adopted project),
so the GAP-6 typed error `STORAGE_CYCLE_PROJECT_MISMATCH` does not fire — that
guard only triggers for **foreign** project prefixes (which is the actual GAP-6
scope). The same-project-missing case therefore reaches the storage layer
where `Storage::acquire_cycle_lease` (crates/sddk-storage/src/lib.rs:902-946)
performs an INSERT into `cycle_leases` without first checking that the
referenced `cycles.cycle_id` exists. The FK on
`cycle_leases.cycle_id REFERENCES cycles(cycle_id) ON DELETE RESTRICT`
(migrations.rs:327-333) fires and surfaces as the misleading
`STORAGE_DATABASE` envelope.

This is the exact same misleading error class that GAP-6 was originally
complaining about for foreign cycles — just for a different cause. The user
cannot distinguish "wrong database" from "cycle doesn't exist in this ledger"
from the error envelope alone.

Reproduction (any own-project workspace):
```bash
$ sddk cycle start --remote <R> --name placeholder --path a-full --actor t --format json
$ cycle_id=$(jq -r .cycle_id < output.json)
$ project_prefix=${cycle_id%/*}              # e.g. p-63676b11dc0ef88f
$ sddk cycle lock acquire --remote <R> \
    --cycle "${project_prefix}/never-existed" --owner t --format json
error[STORAGE_DATABASE]: sqlite storage error: FOREIGN KEY constraint failed
  cause: FOREIGN KEY constraint failed
  recovery: retry after checking the SQLite database integrity   ← MISLEADING
```

## Rationale

**Why this is open debt after GAP-6**: The exploration report
(`exploration-report.md`, exploration of cycle
`gap6-foreign-cycle-typed-error`, 2026-09-01T10:25Z) explicitly listed this
case in §"Affected Areas" row 4 (`cycle lock status`) and
§"Recommended hybrid" (Approach B — storage-layer pre-check). The chosen
hybrid (Approach A + C) only fixed the foreign-prefix case to honor the
ADR-0011 / AGENTS.md §4.1 invariants. The own-project-missing case is a
**pre-existing** storage-layer gap that GAP-6 did not introduce and did not
fix; the apply cycle deferred it to a follow-up.

**Severity: medium** — The error does not block release of v1.66.0 (GAP-6
fix is independent and the fix does not regress this case — pre-existing
behavior is preserved). It does degrade user experience on the
"typo in cycle name" path: every typo returns a misleading
`STORAGE_DATABASE` instead of an actionable `STORAGE_NOT_FOUND`. Workaround:
check `sddk cycle list --status open` before lock acquire.

**Priority: P2** — Degrades non-core functionality with a workaround. Not
release-blocking; should be picked up alongside the next storage-layer
hardening cycle. Per the spec's out-of-scope section and the apply agent's
forecast, this naturally fits in the follow-up cycle
`storage-cycle-lease-pre-existence-check` (separate cycle, not part of
GAP-6 scope).

**Cluster**: unassigned (no existing cluster of FK-error-classification
bugs; this could become CL-006 if a second instance appears).

**Why not already a release blocker**: The GAP-6 cycle
(`p-63676b11dc0ef88f/gap6-foreign-cycle-typed-error`) verified that this
case does **not** regress — the apply agent added the regression test
`cli_cycle_lock_acquire_missing_own_project_returns_foreign_mismatch_when_cycle_does_not_exist`
that documents the actual pre-existing behavior. Same-project prefix still
does NOT produce `STORAGE_CYCLE_PROJECT_MISMATCH` (the typed mismatch is
reserved for foreign prefixes). The drift is bounded and observable.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-01 | sddk-verify (p-63676b11dc0ef88f/gap6-foreign-cycle-typed-error) | created | verify-report.md (this cycle); pre-existing pre-GAP-6 |

## Resolution (planned for separate cycle: `storage-cycle-lease-pre-existence-check`)

The minimal fix mirrors what `Storage::get_cycle` already does for sibling
commands (`cycle status`, `transition`, `rebuild`, `replan`, etc.) — they all
emit `STORAGE_NOT_FOUND` correctly because they call `get_cycle` first. The
fix:

1. Add `Storage::cycle_exists(&self, cycle_id) -> Result<bool>` (single-row
   SELECT on `cycles` table by primary key).
2. Call it at the top of `acquire_cycle_lease`, `renew_cycle_lease`, and
   `release_cycle_lease`. If `false`, return `StorageError::NotFound { entity: "cycle", id }`.
3. Keep the `STORAGE_NOT_FOUND` envelope (same code/recovery as siblings).
4. Tests: integration test that constructs a missing own-project cycle and
   asserts the typed `STORAGE_NOT_FOUND` error, not `STORAGE_DATABASE`.

Estimated effort: ~30-60 LOC + 3 integration tests. Single-cycle
delivery (A-min path). No schema change. No new CLI flag.

## References

- Cycle artifacts: `~/.local/share/sddk/projects/p-63676b11dc0ef88f/cycle-artifacts/p-63676b11dc0ef88f/gap6-foreign-cycle-typed-error/`
  - `spec.md` REQ-GAP6-8 (lines 130-135)
  - `exploration-report.md` §"Affected Areas" row 4; §"Recommended hybrid"
  - `apply-report.md` §"Deviations from Spec" REQ-GAP6-8
  - `implementation-receipt.md` §"Drift / out-of-scope findings"
  - `verify-report.md` (forthcoming from this verify run)
- Code:
  - `crates/sddk-storage/src/lib.rs:902-946` (`Storage::acquire_cycle_lease` — INSERT without pre-check)
  - `crates/sddk-storage/src/migrations.rs:327-333` (FK declaration)
  - `crates/sddk-storage/src/lib.rs:1602-1617` (`StorageError::code` for `CycleProjectMismatch` — does not catch this case)
- Anti-regression test (in-tree at HEAD `aaefa26`):
  `crates/sddk-cli/tests/cli.rs::cli_cycle_lock_acquire_missing_own_project_returns_foreign_mismatch_when_cycle_does_not_exist`
  — documents that same-project-missing does NOT regress to typed mismatch
  (the test asserts `!stderr.contains("STORAGE_CYCLE_PROJECT_MISMATCH")`).
- ROADMAP: GAP-6 entry (`docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md:857-861`)
  documents the typed-error contract; this gap is adjacent but distinct.
## Resolution

Closed by `p-63676b11dc0ef88f/storage-cycle-lease-pre-existence-check` (v1.66.2).

- `Storage::cycle_exists(&self, cycle_id) -> Result<bool>` helper added (crates/sddk-storage/src/lib.rs:340-352).
- 4 pre-check guards added: `acquire_cycle_lease` (lib.rs:939-942), `renew_cycle_lease` (lib.rs:988-990), `release_cycle_lease` (lib.rs:1013-1015), `cycle lock status` propagation (cycle.rs:1163-1178).
- Reuses existing `not_found(...)` path → maps to `STORAGE_NOT_FOUND` (code + recovery hint already in lib.rs:1591, 1614).
- **No new StorageError variant**, no schema migration, no auto-creation.
- 5 anti-regression tests GREEN (cycle_exists unit + 4 CLI integration); 1736/0 workspace tests pass.
- Behavior change: `cycle lock status` and `cycle lock release` for missing own-project cycles now exit non-zero with `STORAGE_NOT_FOUND` (was silent `lease: none` / `released: false`). Consumers spot-checked; none broken.
- GAP-6 anti-regression test flipped: `cli_cycle_lock_acquire_missing_own_project_returns_not_found` (was `_returns_foreign_mismatch_when_cycle_does_not_exist`).
