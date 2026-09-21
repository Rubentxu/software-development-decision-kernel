# SCOPE-CONTRACT — AIW-S8 X04: two-CLI concurrency

- **Cycle**: p-63676b11dc0ef88f / AIW-S8 (CLI host evaluation)
- **Slice**: X04 (CONC — dos CLIs contra mismo storage con update)
- **Scope source**: `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s8-cli-host-evaluation/SCOPE-CONTRACT.md` §S8-STOP-2
- **HEAD at slice start**: 7805d4c

## Intent

Close the X04 gap with a two-CLI integration test simulating two simultaneous
CLI processes (each with its own `AgentHost` + `LeaseStore` reference) racing
for the same `cycle_id` lease against the same storage. Proves:

1. Only one wins the lease (no double authority).
2. The other gets `LeaseError::Conflict` naming the winner and its fencing token.
3. After the winner releases, the loser can acquire cleanly (monotonic fencing token).
4. No state divergence: the store view of `cycle-1` stays consistent after a
   failed attempt (loser cannot release or take over; owner/token unchanged).

## In scope

- `crates/sddk-engine/tests/aiw_s8_x04_two_cli_concurrency.rs` (NEW, 4 tests).

## Out of scope

- Real OS-process parallelism (shared `InMemoryLeaseStore` behind `Arc` simulates
  two CLI processes against one storage; thread-race coverage of the Mutex-protected
  store exists at unit level).
- Production `LeaseStore` impls backed by `sddk_storage::Ledger` (already covered
  by the ledger lease tests).
- X06/X07/X08 rows of the AIW-S8 matrix (separate STOPs).

## Verification batch (change-scoped)

- `cargo fmt --check`
- `cargo clippy -p sddk-engine --all-targets -- -D warnings`
- `cargo test -p sddk-engine --test aiw_s8_x04_two_cli_concurrency` (4 tests)
- `cargo build --release -p sddk-engine`

Rationale: SUT is a new integration test file for `sddk-engine`; no production
code touched, so the scoped batch above is the minimum justified set.
