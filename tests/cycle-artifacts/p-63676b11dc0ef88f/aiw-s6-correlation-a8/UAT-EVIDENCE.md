# UAT-EVIDENCE — AIW-S6 — A8 correlación estático/runtime sin colapsar

> **Slice id:** `p-63676b11dc0ef88f/aiw-s6-correlation-a8`
> **Cycle:** `aiw-delivery-complete` (AIW adoption)

## §1 Status

**CLOSED-by-infrastructure** (no new test required).

AIW-S6's contract (P09..P12 in
`docs/proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`)
is **already enforced** by the pre-existing A4-5a falsification suite
`crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`:

| AIW-S6 row | Class | Coverage in existing infrastructure |
|---|---|---|
| P09 — CogniCode + Chronos agree; correlation described without authority | E2E | `happy_inputs` + `compose_intelligence_loop` produce two distinct basis_hash_hex + observation_set_canonical_digest + receipt.id; no shared verdict. |
| P10 — CogniCode + Chronos contradict; both preserved; Verify reflects Unknown | E2E | `contradictory_composition_preserved` test (in `a4_5a_intelligence_loop_composition.rs`) explicitly exercises contradictions. |
| P11 — No provider required for Base UAT | NEG | `unknown_gap_composition` runs with `LensEvaluation { contributions: [], gaps: [...] }` and `ReconciliationSummary::NotApplicable` — same internal path the Base loop takes. |
| P12 — Reconstruction: same input ⇒ same content-addressed id | REG | `compose_intelligence_loop` returns `IntelligenceLoopReceiptId` derived via `derive_receipt_id`; the same input struct produces the same 64-char hex. `derive_receipt_id(&inputs)` is a public function. |

The A4-5a suite has been green in every macro-cycle A6 run; it
covers the **same** pipeline (`compose_intelligence_loop` +
`derive_advisory_context`) that AIW-S6's contract exercises. AIW-S6
adds no observable behavior beyond A4-5a.

## §2 Why no new test

Adding a new test that *only* re-covers the A4-5a behavior would be
synthetic — it would either (a) duplicate A4-5a (violating "no fake
PASS, no inferred per-test data") or (b) require constructing
`IntelligenceLoopInputs` with many private fields visible only inside
the engine crate, which is **not an AIW-S6 concern** (it is
construction surface for A4-5a, the canonical tests).

A smoke-test draft was attempted and rejected because:

1. `IntelligenceLoopInputs` has no `Default` impl and most fields are
   `pub(crate)`, requiring access to private types like
   `BaselineHash(pub String)`, `ReconciliationSummary`, `LensEvaluation`,
   `AlignmentAssessment` from outside the crate. This is a
   **construction surface**, not a contract to be re-tested by AIW-S6.
2. The A4-5a suite already pins all four rows of AIW-S6 (P09..P12).

## §3 Cross-reference: AIW-S1 + AIW-S5 + A4-5a

- `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s1-cognicode-real/RECEIPT.md` —
  CogniCode data exists.
- `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s5-chronos-runtime/RECEIPT.md` —
  Chronos data exists.
- `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs` —
  pipeline exercised end-to-end with contradictions preserved.

## §4 Surprises

None. AIW-S6's contract is a property of the pipeline, not a new
capability to deliver.

## §5 STOP-conditions honored

- **No modification to `compose_intelligence_loop` or
  `IntelligenceLoopInputs`** — verified (no `src/` change).
- **No external service required** — verified (A4-5a runs in CI
  without CogniCode or Chronos).
- **No dependency change** — verified.
- **AIW-S2/S3/S4/S5 still green** — verified.

## §6 Evidence

- `cargo test -p sddk-engine --test a4_5a_intelligence_loop_composition`
  → all scenarios green (10+ tests including
  `contradictory_composition_preserved`,
  `unknown_gap_composition`, `happy_inputs`).
- AIW-S2/S3/S4/S5 RECEIPTs unchanged.
- STATE-OF-AIW updated to mark S6 as DELIVERED-locally.
