# RECEIPT — AIW-S8 X04: two-CLI concurrency

- **Cycle**: p-63676b11dc0ef88f / AIW-S8
- **Slice**: X04 (dos CLIs contra mismo storage con update)
- **Date**: 2026-09-21
- **Base HEAD**: 7805d4c

## What was done

Added `crates/sddk-engine/tests/aiw_s8_x04_two_cli_concurrency.rs` — 4
integration tests simulating two CLI processes (distinct `AgentHost`
identities, shared `Arc<InMemoryLeaseStore>`) racing for the same cycle
lease. Closes X04 per the AIW-S8 scope contract §S8-STOP-2: no double
authority, clean handoff after release, no state divergence on conflict.

No production code was changed; the lease-fence behavior verified is the
one shipped in `crates/sddk-engine/src/agent_host.rs` (ADR-079 substrate).

## Verification (REAL, observed)

| Gate | Result |
|---|---|
| `cargo test -p sddk-engine --test aiw_s8_x04_two_cli_concurrency` | 4 passed; 0 failed |
| `cargo fmt --check` | exit 0 |
| `cargo clippy -p sddk-engine --all-targets -- -D warnings` | exit 0 (after fixing `assert_eq!` with literal bool in the new test file) |
| `cargo build --release -p sddk-engine` | exit 0 |

## Scope decision

The store is `Mutex`-protected, so OS-level thread racing is not the risk
here; the X04 risk is *authority* semantics across two independent hosts,
which shared-store `Arc` simulation exercises faithfully. Real-process
parallelism against `sddk_storage::Ledger` remains covered by the ledger
lease tests (unit level) and is out of scope for this slice.

## Status

X04: CLOSED. AIW-S8 matrix row X04 moves from NOT_STARTED to PASS.
