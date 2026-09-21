# SCOPE-CONTRACT — AIW-S2 — Captura estructurada de tests y gates

> **Slice id:** `p-63676b11dc0ef88f/aiw-s2-test-runner-capture`
> **Macro-cycle (AIW adoption):** `aiw-delivery-complete`
> **Status:** planning + implementation in one session.

## §1 Goal

Close **AIW-S2**: a real test runner (cargo-nextest) executes against
a known claim; the result is captured in a typed, content-addressable
**RunnerReceipt** that carries `status`, `timeout`, `complete`,
`attempt`, `source basis`, `output refs`, and `redaction`. The
receipt is consumable by Verify/agent-experience consumers via the
Gateway, **without introducing a new runner** (the existing
`sddk_gateway::runner` is reused; the existing `test_runner` family
adapters are reused).

**Important prior context**: `sddk_gateway::runner` already provides
`RunSpec`/`RunOutcome` with bounded-process-execution guarantees
(REQ-WF-RT-018). `sddk_gateway::test_runner` already provides
adapters for cargo-nextest, pytest, jest, go/test, maven/test,
gradle/test. AIW-S2's gap is therefore *not* "build a runner" — it
is "build the typed receipt that wraps `RunOutcome` plus the AIW-S2
fields, so an agent/host consumer (and the Verify pipeline) can
consume a single, structured artefact".

## §2 UAT rows in scope

Per `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`
§Shell / runner / tests (T01..T10):

| UAT id | Scenario | Expected invariant |
|---|---|---|
| T01 | E2E agent executes test via authorized route | SDDK records exit/attempt/basis; LLM report is presentation, not fact. |
| T02 | NEG exit=1 with text `passed` | No PASS, no green gate. |
| T03 | NEG exit=0 but suite doesn't cover claim | Successful command result; Verify still unverified. |
| T04 | NEG timeout, cancel, spawn failure, assertion fail, infra fail | Distinct classes; infra error ≠ test fail. |
| T05 | NEG stdout/stderr/report truncated | `incomplete` propagated; missing testcase → Unknown. |
| T06 | SEC canary secrets in args/stdout/stderr/error/denied | No leak in host view, diagnostic prefix, receipts, logs, CAS; zero effects on deny. |
| T07 | IT run twice with same inputs | Two physical attempts linked; no dedup hides the second. |
| T08 | IT native test report from selected family | Per-testcase/suite normalization with pinned version; do not assert that brief `RunOutcome` already offers this. |
| T09 | NEG external CLI run outside gateway | Do not attribute a non-SDDK run to SDDK; agent may contribute separately. |
| T10 | REG tool missing / not installed / allowlist denied | Typed failure; canonical state unchanged except where logging permitted; no shell fallback. |

## §3 Hard constraints

- **C1**: No modification of `RunOutcome` or `RunSpec` (preserves
  baseline byte-identical, contract pinned in
  `crates/sddk-gateway/tests/bounded_runner_contract.rs`).
- **C2**: No new test runner introduced — reuse
  `sddk_gateway::runner::run` and `sddk_gateway::test_runner::*`.
- **C3**: Wrapper CLI is **optional** per the AIW milestone
  (verbatim: "El wrapper CLI es opcional y se nombra tras verificar
  CommandRegistry"). This slice does **not** add a CLI command; it
  delivers the receipt type, the agent/host integration point
  (via the gateway public surface), and the UAT coverage.
- **C4**: No new dependency added.
- **C5**: Workspace test green; AIW-S0, AIW-S1, AIW-S5, A6 macro-cycle S1..S3 still pass.

## §4 STOP conditions

| Condition | Action |
|---|---|
| Adding `Serialize` to `RunOutcome` is required for the AIW-S2 contract | **STOP** — would break the bounded-runner-contract byte-identical baseline. Use a wrapper type instead. |
| New test runner must be added (no existing family fits the claim) | **STOP** — AIW-S2 says "without introducing a new runner". |
| Wrapper CLI is required for the contract | **STOP** — AIW-S2 says wrapper is optional. Re-evaluate. |
| `cargo fmt` / `cargo clippy -p sddk-gateway --all-targets -- -D warnings` fails | Fix and continue (not a STOP). |
| New authority surface required (e.g. new capability in `CapabilityPolicy`) | **STOP** — material authority change. |

## §5 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../aiw-s2-test-runner-capture/SCOPE-CONTRACT.md` | ✅ |
| RunnerReceipt type + helpers | `crates/sddk-gateway/src/runner_receipt.rs` | 🔲 |
| Unit tests for RunnerReceipt | `crates/sddk-gateway/tests/runner_receipt_contract.rs` | 🔲 |
| Integration test exercising the receipt against a real `cargo-nextest` run | `crates/sddk-gateway/tests/runner_receipt_e2e.rs` (EXT-gated on `CARGO_NEXTEST_BIN`) | 🔲 |
| UAT evidence rows | `tests/cycle-artifacts/.../aiw-s2-test-runner-capture/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `tests/cycle-artifacts/.../aiw-s2-test-runner-capture/RECEIPT.md` | 🔲 |
| 1 commit `feat(gateway)` (source + tests + cycle docs) | — | 🔲 |

## §6 Out of scope

- **AIW-S3** (handoff durable) — independent slice.
- **AIW-S4** (expansión dinámica) — depends on S3.
- **AIW-S6** (correlación A8) — depends on S1+S5.
- **CLI wrapper** — deferred to a follow-up slice if the contract
  proves it necessary after the receipt type is consumed by
  Verify/agent-experience.
- **Modify `RunOutcome` / `RunSpec` / bounded-runner-contract** —
  forbidden by C1.

## §7 References

- AIW milestone: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md` §AIW-S2.
- AIW UAT matrix: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md` §Shell / runner / tests (T01..T10).
- AIW merge plan: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/integration/MERGE-PLAN.md`.
- Bounded runner contract: `crates/sddk-gateway/src/runner.rs` (REQ-WF-RT-018) + `crates/sddk-gateway/tests/bounded_runner_contract.rs`.
- Existing family adapters: `crates/sddk-gateway/src/test_runner/{cargo_nextest,pytest,jest,go_test,maven_test,gradle_test}.rs`.
- AIW state matrix: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md`.
