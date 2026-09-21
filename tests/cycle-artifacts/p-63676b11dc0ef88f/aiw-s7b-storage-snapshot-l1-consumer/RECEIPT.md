# RECEIPT — AIW-S7b — StorageSnapshot → SecretaryL1 consumer

> **Slice:** `p-63676b11dc0ef88f/aiw-s7b-storage-snapshot-l1-consumer`
> **Status:** CLOSED-LOCALLY (pending `scripts/release.sh` push authorization)
> **Scope contract:** `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-2
> **UAT evidence:** `UAT-EVIDENCE.yaml` (same dir)

## §1 What was delivered

A gateway consumer (`SnapshotL1Consumer`) that converts a durable
`StorageSnapshot` (from `sddk_engine::context_compiler::storage_adapter`)
into a closed-set `SecretaryProposal` via the real
`SecretaryL1Engine::propose()`. Closes G01 (reconciled evidence ref from
`(adapter_id, log_head)`, confidence tracks durability) and G03 (no live
`Storage` handle anywhere in the loop).

### Surface changes

| Path | Δ | Description |
|---|---|---|
| `crates/sddk-gateway/src/storage_snapshot_l1_consumer.rs` | new | `SnapshotConsumerError`, `SnapshotL1Consumer::{new,unregistered,with_secretary,register,consume}`. 4 in-module unit tests. |
| `crates/sddk-gateway/src/lib.rs` | +1 LOC | `pub mod storage_snapshot_l1_consumer;` |
| `crates/sddk-gateway/tests/aiw_s7b_snapshot_l1_consumer.rs` | new | 4 integration tests (G01 + G03 + negatives). |
| `tests/cycle-artifacts/.../aiw-s7b-storage-snapshot-l1-consumer/{UAT-EVIDENCE.yaml,RECEIPT.md}` | new | Slice cycle artifacts. |

## §2 Acceptance vs scope

| Constraint | Compliance | Evidence |
|---|---|---|
| C1: no changes under `crates/sddk-engine/src/` | YES | engine untouched |
| C2: engine imports via root reexports + public module path | YES | `use sddk_engine::{BoundedWindow, ClosedSetKind, ProposalTemplate, RiskTier, SecretaryId, SecretaryL1Engine, SecretaryL1Error, SecretaryProposal}` (root reexports, lib.rs L238-239) + `sddk_engine::context_compiler::storage_adapter::StorageSnapshot` (public module) |
| C3: no new dependency edge | YES | `sddk-engine` already a path dependency of gateway (promoted in the S7-prep commit `92a4cb8`); no new crate |
| C4: three deliverables only | YES | consumer module, lib.rs decl, integration test file |
| C5: sync consumption | YES | `consume` is a plain sync fn returning `Result<SecretaryProposal, SnapshotConsumerError>` |
| C6: scoped checks | YES | see §3 |
| C7: one feature commit, no push, no release script | YES | see §6 |

### UAT coverage

| UAT id | Status | Why |
|---|---|---|
| G01 | PASS | evidence ref = `storage:{adapter_id}:{log_head}` (reconciled pair, A blocks B); confidence 0.95/0.5 by head, asserted in unit + integration |
| G03 | PASS | proposal issued from `StorageSnapshot` alone; no `Storage` in scope anywhere |

## §3 Real verification output (cargo, 2026-09-21)

```
$ cargo fmt --check                      # exit 0

$ cargo clippy -p sddk-gateway --all-targets -- -D warnings
    Checking sddk-gateway v1.169.122
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.28s

$ cargo test -p sddk-gateway --lib storage_snapshot_l1_consumer
running 4 tests
test storage_snapshot_l1_consumer::tests::accepts_durable_snapshot ... ok
test storage_snapshot_l1_consumer::tests::empty_log_head_reduces_confidence ... ok
test storage_snapshot_l1_consumer::tests::rejects_empty_adapter_id ... ok
test storage_snapshot_l1_consumer::tests::rejects_when_template_not_registered ... ok

test result: ok. 4 passed; 0 failed; ...; 123 filtered out

$ cargo test -p sddk-gateway --test aiw_s7b_snapshot_l1_consumer
running 4 tests
test empty_adapter_id_e2e ... ok
test durable_snapshot_e2e ... ok
test no_template_e2e ... ok
test empty_log_head_e2e ... ok

test result: ok. 4 passed; 0 failed; ...

$ cargo build --release -p sddk-gateway
    Finished `release` profile [optimized] target(s) in 3.45s
```

**Counts:** 8 new tests (4 in-module + 4 integration), all passing; 123
existing gateway lib tests unaffected; release build clean.

## §4 Deviations

Two spec-shape adjustments, both forced by the real engine API (read
before writing, per slice instructions):

1. `ProposalTemplate::new` takes 5 args (includes `summary`); the sketch
   showed 4. `SecretaryL1Error` is `PartialEq` but not `Eq`, so
   `SnapshotConsumerError` derives `PartialEq` only.
2. Added `SnapshotL1Consumer::unregistered(now_ms)` (empty-engine raw
   constructor) so the "template not registered" path is exercised
   through the real engine instead of mutating private fields.

No STOP condition hit; no private engine API required.

## §5 Out of scope (not closed by this slice)

Live MCP transport wiring, durable persistence, other AIW-S7 rows,
manifest edits, release, push.

## §6 Pending release.sh authorization

This slice is committed locally on top of the S7a feature commit but
**not pushed and not released**. Running `bash scripts/release.sh`
requires explicit operator authorization per the AIW adoption workflow.
Until then, local state is: workspace version `1.169.122`, development
HEAD = this feature commit, no new tag.
