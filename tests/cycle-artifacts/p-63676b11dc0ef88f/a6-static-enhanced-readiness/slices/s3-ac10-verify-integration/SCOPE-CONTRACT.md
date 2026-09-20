# SCOPE-CONTRACT — S3 — AC10 Verify Integration

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s3-ac10-verify-integration`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Status:** planning + implementation in one session.

## §1 Goal

Close **AC10** end-to-end: a CogniCode-driven `AnalysisResult` from
`CodeIntelligencePort` flows into the existing `VerifyKernel` via a
typed **adapter** that translates `code_intelligence_port::ObservationSet`
into `observation::types::ObservationSet` (the substrate shared by
`VerifyKernel` and `DebVerifyKernel` per ADR-0122). The adapter must
satisfy `PR-UAT-019` (Verify delta-scoped; no unconditional full repo
scan) and must distinguish `OBSERVED_STATIC` from `INFERRED` evidence
classes.

**Important prior context**: `StaticProviderDomain` already exists at
`crates/sddk-engine/src/verify_kernel/adapter_static_provider.rs` and
**already consumes** `observation::ObservationSet` (per the Verify
substrate). The AC10 seam therefore is *not* "build a new Verify
domain"; it is "build the **bridge** between the two SDDK
`ObservationSet` types so that a CogniCode `AnalysisResult` becomes a
Verify-consumable `ObservationSet`".

## §2 UAT rows in scope

| UAT id | Scenario | Expected invariant | Adapter assertion |
|---|---|---|---|
| AC10 | Static evidence is consumed by Verify | A `code_intelligence_port::AnalysisResult` round-trips into a `VerifyKernel::evaluate` call that produces a `VerificationResult` derived from the provider's observations. | `t_ac10_cognicode_observation_set_bridges_into_verify_kernel` |
| `PR-UAT-C04` (re-pinned) | Localized delta analysis | A localized `analyze_delta` produces a Verify-consumable set **without** forcing a full graph transfer. | `t_ac10_localized_delta_no_full_graph_in_verify` |
| `PR-UAT-019` | Localized source delta | Verify analyzes affected scope/impact without unconditional full repository scan; emits a `VerifyReceipt` (verified/contradicted/unknown — never false PASS). | `t_ac10_verify_receipt_marks_observed_static_evidence` |

## §3 Hard constraints

- **C1**: No modification of `Evidence` / `EvidencePosture` (A4 §2
  freeze). The adapter may CONSTRUCT `EvidenceRef` and
  `SoftwareObservation` (which carry an `EvidenceRef`) but does not
  modify the type definitions.
- **C2**: No modification of `arch-spec-021` or
  `arch-acceptance-coverage-001`. AC10 already lives there.
- **C3**: No modification of `EvidenceSource` (substrate shared by
  Verify + DebVerify — adding methods here is an anti-encroachment
  violation per the file's own doc comment).
- **C4**: Zero provider DTO types in `sddk-domain` (CC-S0/CC-S1/S1/S2
  invariant). The adapter lives in `sddk_engine::code_intelligence_port`
  or `sddk_engine::verify_kernel`, NOT in `sddk_engine::observation`.
- **C5**: No new dependency added.
- **C6**: Workspace test green; CC-S0, CC-S1, S1, S2 still pass.

## §4 STOP conditions

| Condition | Action |
|---|---|
| Building the bridge requires modifying `Evidence` / `EvidencePosture` / `EvidenceRef` / `EvidenceSource` / `ObservationSet` (either one) | **STOP** that line only. Report with proposed schema. S4 (durability) is independent and can continue. |
| Building the bridge requires adding a CogniCode DTO type to `sddk-engine::observation` or `sddk-domain` | **STOP** that line only. Report. |
| `cargo fmt --check` / `cargo clippy -p sddk-engine --all-targets -- -D warnings` fails | Fix and continue (not a STOP). |
| The bridge requires a new authority surface in `arch-spec-021` | **STOP**. The macro-cycle plan says S3 STOPs if AC10 forces `Evidence`/`EvidencePosture` change; a new authority is a stronger stop. |

The operator's instruction this session: only that line stops; S4
(durability) and S6 (fake relocation) are independent and may
continue if S3 stops.

## §5 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../slices/s3-ac10-verify-integration/SCOPE-CONTRACT.md` | ✅ |
| Adapter module + helpers | `crates/sddk-engine/src/code_intelligence_port/verify_bridge.rs` (or extend an existing module) | 🔲 |
| Test binary with 3 UAT tests | `crates/sddk-engine/tests/a6_s3_ac10_verify_integration.rs` | 🔲 |
| UAT evidence rows | `tests/cycle-artifacts/.../slices/s3-ac10-verify-integration/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `tests/cycle-artifacts/.../slices/s3-ac10-verify-integration/RECEIPT.md` | 🔲 |
| 1 commit `feat(engine)` (source + tests + cycle docs) | — | 🔲 |

## §6 Out of scope

- **Durability** (S4): the bridge does not persist anything.
- **Real CogniCode EXT** (S5): out — S5 owns that.
- **Fake relocation** (S6): out — independent slice.
- **Closeout** (S7): out.

## §7 References

- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/SCOPE-CONTRACT.md` §S3 (macro-cycle plan)
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md` (CC scope, AC10 mention)
- `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md` (AC10 / IPB-008)
- `crates/sddk-engine/src/verify_kernel/adapter_static_provider.rs` (existing static-provider Verify domain)
- `crates/sddk-engine/src/verify_kernel/evidence_source.rs` (substrate; must NOT be modified)
- `crates/sddk-engine/src/code_intelligence_port.rs` (the `AnalysisResult` source)
- `crates/sddk-engine/src/observation/types.rs` (the `SoftwareObservation` target)
