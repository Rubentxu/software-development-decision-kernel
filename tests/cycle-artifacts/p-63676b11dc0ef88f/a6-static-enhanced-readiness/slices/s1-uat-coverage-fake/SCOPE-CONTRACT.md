# SCOPE-CONTRACT — S1 — UAT Coverage via Fake

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s1-uat-coverage-fake`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Status:** planning + implementation in one session (auto-run between slices).

## §1 Goal

Cover `PR-UAT-C01, C02, C03, C04, C06, C08, C10` from
`docs/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md`
§E end-to-end through `CoverageContract` evaluation. Convert the spike
character of CC-S0/CC-S1 into a UAT-anchored coverage matrix using the
fake provider.

This slice is the **deterministic subset** of A6 — the rows that can
be exercised without a real CogniCode provider. The remaining rows
(C05, C07, C09) move to S2; AC10 to S3; durability to S4; real
provider to S5.

## §2 UAT rows in scope

| UAT id | Scenario | Expected invariant (verbatim from §E) | Test |
|---|---|---|---|
| `PR-UAT-C01` | Provider stopped, requirement OPTIONAL/PREFERRED | SDDK remains Base-capable; records EvidenceGap/NOT_EVALUATED | `t_uat_c01_optional_provider_unavailable_yields_evidence_gap` |
| `PR-UAT-C02` | Provider stopped, requirement REQUIRED | Requested operation fails explicitly; Base state remains valid | `t_uat_c02_required_provider_unavailable_yields_typed_failure` |
| `PR-UAT-C03` | Compatible provider startup/handshake | Runtime protocol/capabilities negotiated and recorded | `t_uat_c03_compatible_handshake_records_capabilities` |
| `PR-UAT-C04` | Localized delta analysis | Typed result + basis/analyzer provenance; no forced full graph transfer | `t_uat_c04_localized_delta_typed_result_with_basis` |
| `PR-UAT-C06` | Cancellation/deadline | Incomplete/cancelled result; no green VerifyReceipt from missing requested evidence | `t_uat_c06_cancellation_yields_incomplete_not_satisfied` |
| `PR-UAT-C08` | Protocol major incompatible | Provider state is INCOMPATIBLE; no capability inference from product version | `t_uat_c08_protocol_incompatible_no_capability_inference` |
| `PR-UAT-C10` | Repeat deterministic analysis on same basis/analyzer set | Promised deterministic semantic digest/result identity is stable | `t_uat_c10_repeat_deterministic_analysis_yields_stable_digest` |

## §3 Hard constraints (from macro-cycle)

- **C1**: No architectural change to `CodeIntelligencePort` trait or its ADTs (CC-S0/CC-S1 already shipped them; this slice only consumes them).
- **C2**: Zero provider DTO types in `sddk-domain` (continuation of CC-S0/CC-S1 invariant).
- **C3**: No modification of `arch-spec-021`. The `arch-acceptance-coverage-001` contract already covers AR-1..AR-7; this slice only **exercises** them.
- **C4**: No new dependency added.
- **C5**: `cargo test --workspace --offline` must stay green for all crates (not just `sddk-engine`).

## §4 STOP conditions

Per macro-cycle plan §4:

1. **Discovery that AC10 (Verify consumes static evidence) requires architectural change to Verify semantics** → STOP and report. (S3 owns AC10, not S1; if S1 reveals AC10 needs change, it goes to S3 planning.)
2. **Discovery that the fake provider's `CapabilitySnapshot` cannot faithfully represent a real consumer's claim** → STOP and report.
3. **`cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` failures** → fix and continue (not a STOP; routine).
4. **Discovery that `PR-UAT-C0X` cannot be expressed in terms of the existing CC-S0/CC-S1 types** → STOP and report; the slice is not the right place to invent new ADTs.

## §5 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../slices/s1-uat-coverage-fake/SCOPE-CONTRACT.md` | ✅ |
| Test binary with 7 UAT tests | `crates/sddk-engine/tests/a6_s1_uat_coverage_fake.rs` | 🔲 |
| UAT evidence rows | `tests/cycle-artifacts/.../slices/s1-uat-coverage-fake/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `tests/cycle-artifacts/.../slices/s1-uat-coverage-fake/RECEIPT.md` | 🔲 |
| 1 commit `feat(engine)` (test-only) | — | 🔲 |
| 1 push to `origin/main` | — | 🔲 |

## §6 Out of scope

- **C05, C07, C09** — S2.
- **AC10** — S3.
- **Durability** — S4.
- **Real CogniCode EXT** — S5.
- **Fake relocation** — S6.
- **Closeout** — S7.

## §7 References

- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/SCOPE-CONTRACT.md` (macro-cycle plan)
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md` §E, §I
- `docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md` (CC-S0 close)
- `tests/cycle-artifacts/.../a6-cc-s1-static-graph-completeness/RECEIPT.md` (CC-S1 close)
- `docs/architecture/specs/arch-acceptance-coverage-001.md` (acceptance contract)
