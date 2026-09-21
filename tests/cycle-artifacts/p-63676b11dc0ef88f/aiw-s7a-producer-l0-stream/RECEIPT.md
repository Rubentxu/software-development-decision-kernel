# RECEIPT — AIW-S7a — Producer→L0 stream adapter

> **Slice:** `p-63676b11dc0ef88f/aiw-s7a-producer-l0-stream`
> **Status:** CLOSED-LOCALLY (pending `scripts/release.sh` push authorization)
> **Scope contract:** `SCOPE-CONTRACT.md` (same dir)
> **UAT evidence:** `UAT-EVIDENCE.yaml` (same dir)

## §1 What was delivered

A synchronous gateway adapter (`ProducerToL0Adapter`) that converts
external producer events (CogniCode findings, Chronos crash/race) into
public `ReactiveEvent` values and evaluates them with the real
`SecretaryL0Engine`. Unknown events return an empty signal list (G06).

### Surface changes

| Path | Δ | Description |
|---|---|---|
| `crates/sddk-gateway/src/producer_l0_adapter.rs` | new | `FindingKind`, `ProducerEvent`, `ProducerToL0Adapter::{new,with_now,dispatch}`. 4 in-module unit tests. |
| `crates/sddk-gateway/src/lib.rs` | +1 LOC | `pub mod producer_l0_adapter;` |
| `crates/sddk-gateway/tests/aiw_s7a_producer_l0.rs` | new | 4 integration tests (G04 + G06). |
| `tests/cycle-artifacts/.../aiw-s7a-producer-l0-stream/{SCOPE-CONTRACT.md,UAT-EVIDENCE.yaml,RECEIPT.md}` | new | Slice cycle artifacts. |

## §2 Acceptance vs scope

### §2.1 Constraints (SCOPE §3)

| Constraint | Compliance | Evidence |
|---|---|---|
| C1: no changes under `crates/sddk-engine/src/` | YES | engine untouched; `git status` shows only gateway + artifacts |
| C2: engine imports via root reexports | YES | `use sddk_engine::{ReactiveEvent, ReactiveMatcher, ReactiveRule, ReactiveSignal, ReactiveTrigger, SecretaryL0Engine}` — all in the `sddk-engine/src/lib.rs` reexport list |
| C3: no new dependency edge | YES | engine promoted to `[dependencies]` (pre-existing path dep, committed before this slice); no new crate |
| C4: three deliverables only | YES | adapter module, lib.rs decl, integration test file |
| C5: sync dispatch | YES | `dispatch` is a plain sync fn returning `Vec<ReactiveSignal>` |
| C6: scoped checks | YES | see §3 |
| C7: one feature commit, no push, no release script | YES | see §6 |

### §2.2 UAT coverage

| UAT id | Status | Why |
|---|---|---|
| G04 | PASS | real `engine.register` + `evaluate` with the adapter's `ReactiveEvent` shape fires the configured `PersistEvidence`/`OpenHumanDecision` signals |
| G06 | PASS | `ProducerEvent::Unknown` → empty signal list, unit + integration |

## §3 Real verification output (cargo, 2026-09-21)

```
$ cargo fmt --check                      # exit 0
$ cargo clippy -p sddk-gateway --all-targets -- -D warnings
    Checking sddk-gateway v1.169.122
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.75s

$ cargo test -p sddk-gateway --lib producer_l0_adapter
running 4 tests
test producer_l0_adapter::tests::unknown_event_empty ... ok
test producer_l0_adapter::tests::cognicode_finding_dispatches ... ok
test producer_l0_adapter::tests::chronos_crash_dispatches ... ok
test producer_l0_adapter::tests::registered_rule_fires_persist_evidence ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 119 filtered out; finished in 0.00s

$ cargo test -p sddk-gateway --test aiw_s7a_producer_l0
running 4 tests
test chronos_race_e2e ... ok
test cognicode_finding_e2e ... ok
test chronos_crash_e2e ... ok
test unknown_event_e2e ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

$ cargo build --release -p sddk-gateway
    Finished `release` profile [optimized] target(s) in 3.55s
```

**Counts:** 8 new tests (4 in-module + 4 integration), all passing; 119
existing gateway lib tests unaffected (filtered only by name in the scoped
lib run); release build clean.

## §4 Deviations

None. No STOP condition hit; no private engine API required.

## §5 Out of scope (not closed by this slice)

Live MCP transport wiring, durable persistence, other AIW-S7 rows,
manifest edits, release, push.

## §6 Pending release.sh authorization

This slice is committed locally on top of the gateway `[dependencies]`
promotion but **not pushed and not released**. Running
`bash scripts/release.sh` requires explicit operator authorization per
the AIW adoption workflow. Until then, local state is: workspace version
`1.169.122`, development HEAD = this feature commit, no new tag.
