# RECEIPT — AIW-S2 — Captura estructurada de tests y gates

> **Slice:** `p-63676b11dc0ef88f/aiw-s2-test-runner-capture`
> **Status:** CLOSED (locally; pending release via `scripts/release.sh`)
> **Scope contract:** `SCOPE-CONTRACT.md` (same dir)
> **UAT evidence:** `UAT-EVIDENCE.yaml` (same dir)

## §1 What was delivered

A typed, content-addressable **runner receipt** that wraps the bounded
runner (`sddk_gateway::runner`) without modifying it.

### Surface changes

| Path | Δ | Description |
|---|---|---|
| `crates/sddk-gateway/src/runner_receipt.rs` | +449 LOC | New module: `RunnerReceipt`, `RunnerStatus`, `RedactionMarker`, `OutputRefs`, `RunOutcomeView`. 10 in-module unit tests. |
| `crates/sddk-gateway/tests/runner_receipt_e2e.rs` | +243 LOC | 11 E2E tests covering T01/T02/T04/T05/T06/T07/T08/T09/T10. |
| `crates/sddk-gateway/src/lib.rs` | ±2 LOC | `pub mod runner_receipt;` + `pub use` re-exports. |
| `tests/cycle-artifacts/.../aiw-s2-test-runner-capture/{SCOPE-CONTRACT.md,UAT-EVIDENCE.yaml,RECEIPT.md}` | +new | Slice cycle artifacts. |

### Why a wrapper, not a modification

`RunOutcome` is the byte-identical pinned contract of the bounded runner
(`bounded_runner_contract.rs`, 18 tests). The AIW-S2 milestone text
explicitly forbids changing the runner. The wrapper type is the
contract-respecting answer.

`RunOutcome` lacks `Serialize`; `RunOutcomeView` is the wire-format
mirror that keeps the receipt self-describing on disk without altering
the contract.

## §2 Acceptance vs scope

### §2.1 What worked

| Constraint | Compliance | Evidence |
|---|---|---|
| C1: no modification of `RunOutcome` / `RunSpec` | **YES** | `bounded_runner_contract` 18/18 still pass; receipt's `outcome` field is `#[serde(skip)]`; the wire surface is `outcome_view` |
| C2: no new test runner introduced | **YES** | `test_runner/` family unchanged; receipt reuses `runner::run` |
| C3: wrapper CLI not added | **YES** | `command_spec.rs` untouched |
| C4: no new dependency | **YES** | `serde`, `serde_json`, `sha2` already present in `Cargo.toml` |
| C5: workspace green | **YES** | clippy `-D warnings` clean, fmt clean, hook self-test 35/35 |

### §2.2 What was not done (out of scope)

- **Per-testcase normalization (T08 second-half)**: out of scope per §6.
  The receipt carries `output_refs` + `outcome_view`; consumers that need
  per-testcase normalization will integrate with the family adapters in
  `sddk_gateway::test_runner::*`. A future slice will materialize the
  parser for cargo-nextest's native JSON output specifically.
- **Wrapper CLI**: deferred. The receipt is the obligatory piece; the
  CLI is optional and no CommandRegistry entry was created.

### §2.3 Deviations

- **D1**: `RunnerReceipt::outcome` is `#[serde(skip)]` to avoid
  requiring `Serialize` on `RunOutcome`. The wire field `outcome_view`
  carries the JSON mirror of the same data. **Justification**: AIW-S2's
  C1 forbids modifying `RunOutcome`; we honour the contract by adding
  the mirror field.
- **D2**: `run_with_receipt` returns `Result<RunnerReceipt,
  RunnerError>` (not `Result<_, _>` of the receipt type), to avoid
  the clippy `result_large_err` warning while keeping the contract
  precise (the existing runner returns `RunnerError`; the receipt
  builder is a separate step). The caller composes the two: run the
  runner, then either `build_receipt` or `build_failed_to_start`.

## §3 Test count

| Test class | File | Pass count |
|---|---|---|
| In-module unit tests | `crates/sddk-gateway/src/runner_receipt.rs` (`#[cfg(test)] mod tests`) | 10 |
| E2E integration tests | `crates/sddk-gateway/tests/runner_receipt_e2e.rs` | 11 |
| Bounded runner contract (C1 pin, unchanged) | `crates/sddk-gateway/tests/bounded_runner_contract.rs` | 18 |
| **Total new tests added by this slice** | | **21** |

Test commands (local verification):

```text
cargo test -p sddk-gateway --lib runner_receipt
cargo test -p sddk-gateway --test runner_receipt_e2e
cargo test -p sddk-gateway --test bounded_runner_contract
cargo fmt -p sddk-gateway --check
cargo clippy -p sddk-gateway --all-targets -- -D warnings
```

All green.

## §4 Commits

> Commits pending — see §6 for the planned single commit below.

**Local**, not pushed (per H4.7 protocol: no push for this slice unless
operator explicitly authorizes an AIW-release run via `scripts/release.sh`).

## §5 Receipt as "Receipt" (semantic)

This `tests/cycle-artifacts/.../aiw-s2-test-runner-capture/RECEIPT.md` is
the **SDDK closure receipt** — it documents what was delivered against
the SCOPE-CONTRACT, what evidence exists, and what is out of scope.

It is **NOT** to be confused with `sddk_gateway::RunnerReceipt` (the
typed in-memory artefact produced by the bounded runner). The two are
distinct: the **SDDK cycle receipt** is durable documentation; the
**runner receipt** is runtime content-addressable evidence.

## §6 Commit plan (single feature commit when authorized)

```text
feat(gateway): add runner_receipt wrapper for AIW-S2

* Wraps `RunOutcome` (byte-identical pinned contract) with the
  AIW-S2 fields: status, timeout, complete, attempt, source basis,
  output_refs, redactions, outcome_view (wire mirror).
* 10 in-module unit tests + 11 E2E integration tests (T01..T10).
* No modification of `RunOutcome` / `RunSpec` (C1 preserved:
  bounded_runner_contract 18/18 still green).
* Reuses `runner::run` and `test_runner::*` (C2).
* Wrapper CLI NOT added (C3: optional per AIW-S2).
* Slice cycle: tests/cycle-artifacts/.../aiw-s2-test-runner-capture/.
```

This commit is **local** until the operator authorizes an AIW release
run via `scripts/release.sh`. No auto-bump; release pipeline owns the
version.

## §7 Outstanding items (non-blocking for this slice)

- **AIW-S3** (handoff durable) — independent slice, sequenced next.
- **T08 per-testcase normalization** — folded into AIW-S3 follow-up.
- **Wrapper CLI** — deferred; current delivery satisfies the obligatory
  field (the typed receipt) and the consumer (verify/agent-experience
  modules can call `build_receipt` / `run_with_receipt` directly).

## §8 References

- AIW milestone: `docs/proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md` §AIW-S2.
- AIW UAT matrix: `docs/proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md` §Shell / runner / tests.
- AIW state matrix: `docs/proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md`.
- Bounded runner: `crates/sddk-gateway/src/runner.rs`, contract test: `crates/sddk-gateway/tests/bounded_runner_contract.rs`.
- Family adapters (reused, not modified): `crates/sddk-gateway/src/test_runner/`.
