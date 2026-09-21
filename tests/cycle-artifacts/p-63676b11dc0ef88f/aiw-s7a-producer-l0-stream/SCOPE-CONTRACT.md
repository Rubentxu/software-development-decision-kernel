# SCOPE-CONTRACT — AIW-S7a — Producer→L0 stream adapter

> **Slice id:** `p-63676b11dc0ef88f/aiw-s7a-producer-l0-stream`
> **Macro-cycle (AIW adoption):** `aiw-delivery-complete`
> **Status:** Dependency blocker RESOLVED (operator authorization 2026-09-21); ready for implementation.

## §1 Goal

Expose a synchronous gateway `ProducerToL0Adapter::dispatch(ProducerEvent)` that converts CogniCode findings and Chronos crash/race events into public `ReactiveEvent` values and evaluates them with the real `SecretaryL0Engine`. Unknown events return `ReactiveSignal::Silent`. No engine modifications, no new dependency edge, no release or push.

## §2 UAT rows in scope

| UAT id | Scenario | Expected invariant |
|---|---|---|
| G04 | Producer evidence reaches L0 | Public gateway adapter dispatches to the real engine and returns configured signals. |
| G06 | Unknown producer input | Unknown events are silent, with no automatic WorkItem or external side effect. |

Four planned tests: three in-module tests for CogniCode, Chronos crash, unknown events, and one integration test for real rule registration and dispatch. These demonstrate the typed adapter boundary, not a live MCP connection or durable persistence. No broader AIW-S7 row is closed by fixture-only evidence.

## §3 Hard constraints

- C1: No changes under `crates/sddk-engine/src/`.
- C2: Engine imports use existing root reexports in `sddk-engine/src/lib.rs`.
- C3: No **new** dependency edge. The `sddk-engine` entry is promoted from `[dev-dependencies]` to `[dependencies]` in `crates/sddk-gateway/Cargo.toml` (a promotion of an existing path dependency, not an addition). No new crate is added.
- C4: Three source/test deliverables only: new `src/producer_l0_adapter.rs`, module declaration in gateway `src/lib.rs`, new `tests/aiw_s7a_producer_l0.rs`. The Cargo.toml promotion is the fourth deliverable, already committed.
- C5: Sync dispatch, three unit tests and one integration test.
- C6: Required scoped checks: fmt check, gateway all-targets clippy with warnings denied, gateway lib tests, named integration test, release gateway build, full gateway tests.
- C7: One feature commit on top of the Cargo.toml promotion (or merged with it), no push, no release script.

## §4 STOP conditions

| Condition | Action |
|---|---|
| Required engine modification or non-public engine type | STOP and raise to operator. |
| Tests fail to compile | Fix within admitted scope, never skip. |

### Resolved dependency blocker

Initial diagnosis by the spec agent flagged `sddk-engine` as only in
`[dev-dependencies]`. Operator authorized promotion to `[dependencies]`
under the S7a slice (consistent with gateway being the producer-side
layer per AIW-S7 §S7-STOP-1). After promotion, `cargo build --release
-p sddk-gateway` compiles clean (verified 2026-09-21) and the 119
existing gateway lib tests pass.

## §5 Deliverables

| Deliverable | Status |
|---|---|
| SCOPE-CONTRACT.md | Written first |
| Three source/test files | Blocked, not written |
| UAT-EVIDENCE.yaml | No PASS evidence available |
| RECEIPT.md | Pending blocker disposition |
| Single local commit | Not created |

## §6 Out of scope

Engine changes, live MCP transport wiring, durable persistence, other AIW-S7 rows, manifest edits without authorization, releases and pushes. Concurrent changes in `docs/proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md` are not owned by this slice.

## §7 References

- `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md` §S7-STOP-1.
- `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s3-handoff-durable/SCOPE-CONTRACT.md` and `RECEIPT.md` (format).
- `crates/sddk-engine/src/lib.rs` and `secretary_l0.rs` (read-only API evidence).
- `crates/sddk-gateway/Cargo.toml` (dependency evidence).
