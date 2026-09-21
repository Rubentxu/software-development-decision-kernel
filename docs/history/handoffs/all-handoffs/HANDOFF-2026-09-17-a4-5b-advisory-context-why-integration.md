# HANDOFF — A4-5b AdvisoryContext + WHY Integration (v1.169.66)

## Certified state

| | |
|---|---|
| **Cycle** | `p-63676b11dc0ef88f/a4-5b-advisory-context-why-integration` (A-min, **CLOSED**) |
| **Change budget** | `ADVISORY CONTEXT + WHY INTEGRATION` (single) |
| **Released baseline** | `v1.169.65` → `38b0a13e940a75bee5177a830122c796a347d592` |
| **Development head at cycle open** | `a5fb350e5a6224e4984c9c0406b5a19e4f93d84e` |
| **Workspace version** | `1.169.66` |
| **Actual release tag** | `v1.169.66` |
| **Release SHA** | `aa3aa91c11794baf9b5bce34cb26e704d1c7d59b` |
| **HEAD == origin/main** | yes |
| **Tag → SHA** | `v1.169.66` → `aa3aa91` |
| **PublicReleaseGate** | **PASS** (draft=false, prerelease=false, 9/9 assets, local install coherent) |
| **Binary / bundle / current** | all `1.169.66` |
| **Binary sha256** | `cba8ca2a937692e168630ab65a9a5bd2b7fddacbb866b20e773f28b3ad8ba413` |
| **ADR** | `ADR-0128-ADVISORY-CONTEXT-WHY-INTEGRATION` |
| **Unblocks** | A4-5C (does NOT auto-open) |

> §25: release SHA, development HEAD, workspace version, actual tag and
> released baseline are recorded **separately** per
> `INC-A4-RELEASE-VERSION-DRIFT`.

## Test totals (real, at close)

```text
cargo test --workspace  →  PASSED=4644  FAILED=0  IGNORED=14
A4-5b corpus            →  20 tests (P1..P18 + 2 real end-to-end UAT)
```

## What shipped

The A4-5a composition output now feeds the **existing A3
`AdvisoryContext`** (reused verbatim) and a typed, provenance-backed WHY.

```text
Knowledge/observations → Verify → DebVerify → AlignmentLens →
reduce_alignment → compose_intelligence_loop
        → IntelligenceLoopResult
        → AdvisoryContext  (A3, reused)
        → AdvisoryWhy      (new, typed, EPHEMERAL)
```

New module `crates/sddk-engine/src/intelligence_advisory/`:

- `derive_advisory_context(&IntelligenceLoopResult) -> AdvisoryContext` —
  deterministic, canonical. One item per verification pair, the
  reconciliation, each lens contribution, each lens coverage gap, and the
  alignment posture.
- `AdvisorySubjectRef` — typed selector; `subject_key` is the single
  render point, never parsed back.
- `AdvisoryWhy { subject, basis, legs, unresolved, why_not }` —
  EPHEMERAL; `AdvisoryWhyBasis` is clock-stable; `AdvisoryWhyLeg` carries
  the exact typed loop values; `AdvisoryWhyNot` only fires on a typed
  negative reason; `AdvisoryWhyUnresolved` matches the
  `architecture_why` shape.

Reuse, not duplication:

- `AdvisoryContext`/`AdvisoryItem`/`AdvisoryProvenance` (A3) reused
  unchanged; `AdvisoryKind` extended with `VerificationOutcome`,
  `DebtReconciliation`, `LensCoverageGap`; `AlignmentTension` (reserved at
  A3 closeout) is now produced.
- `architecture_why` continues to explain contract/finding provenance.
  The advisory WHY explains advisory items — a different subject domain,
  the same philosophy. Its stale "no evidence node kind" claim (false
  after A4-S15R) is corrected.
- `ArchitectureGraphOverlay` gained a **read-only**
  `contract_provenance(contract_id) -> ContractProvenance` exposing
  A4-S15R's two axes as `specified_by` (spec payloads) and `verified_by`
  (typed `EvidenceRef`). No new relation.
- `EvidenceKind::from_domain_tag` — a closed-tag codec (fail-closed).

## §10 Falsification corpus (P1–P19)

`crates/sddk-engine/tests/a4_5b_advisory_context_why.rs` (20 tests):

| Pin | What it proves |
|---|---|
| P1 | insertion-order-independent derivation + canonical subject order |
| P2 | `SpecifiedBy` does not imply `VerifiedBy`; empty evidence axis reported, not negated |
| P3 | `VerifiedBy` cardinality 0/1/N + typed reconstruction + dedup |
| P4 | `Conflicted` posture keeps supporting **and** contradicting (no latest-wins) |
| P5 | `NoRegisteredLens` survives to payload + WHY |
| P6 | `LensExistsButRefused` distinct from `NoRegisteredLens` (different subjects; querying the wrong reason → unresolved) |
| P7 | `VerificationClaim ↔ VerificationResult` pairing exact |
| P8 | all `ReconciliationSummary` variants preserved verbatim |
| P9 | `AlignmentAssessment` + `AlignmentAssessmentId` preserved |
| P10 | outcome notes distinct; `NotApplicable` ≠ `NotEvaluated` ≠ `Unknown` ≠ `Stale` |
| P11 | absent contract → empty axes + `unresolved`, no negative |
| P12 | no string semantics (source pin, comments stripped) |
| P13 | no hidden orchestration (source pin) |
| P14 | `MISALIGNED` is advisory; no authority/capability/instruction/DENY |
| P15 | no global verdict field (debug-repr) |
| P16 | A3 `AdvisoryContext` reused verbatim + canonicalization invariant |
| P17 | deterministic WHY across iteration order |
| P18 | `evaluation_time` not in the basis |
| UAT ×2 | real VerifyKernel + AlignmentLensKernel + reduce_alignment + compose; one case with `VerifiedBy=[]`, one with N evidence |

## §24 Claim → Evidence

```text
OBSERVED:  derive_advisory_context returns the A3 AdvisoryContext type.
OBSERVED:  the payload is canonical (insertion-order independent).
OBSERVED:  WHY is typed, provenance-backed, and deterministic.
OBSERVED:  an empty VerifiedBy axis is reported, never negated.
OBSERVED:  SpecifiedBy and VerifiedBy are separate axes end-to-end.
OBSERVED:  a real end-to-end chain differs only by real provenance.

STRUCTURAL: no second advisory model; A3 type reused.
STRUCTURAL: no orchestration (source-pinned).
STRUCTURAL: no authority/capability/instruction/Governance coupling.
STRUCTURAL: no global verdict surface.
STRUCTURAL: no provider/LLM/workbook/counterfactual encroachment.
STRUCTURAL: no new relation added to the graph.

DERIVED:    SDDK can answer "why is this advisory item here?" with typed,
            verifiable provenance — and cannot answer "what decision
            should I make?" from this surface.
```

## Exit criteria (§17)

- [x] existing A3 AdvisoryContext reused
- [x] no competing AdvisoryContext model
- [x] typed WHY exists
- [x] WHY is provenance-backed
- [x] WHY does not infer stronger conclusions
- [x] SpecifiedBy != VerifiedBy preserved
- [x] absence != negation preserved
- [x] epistemic distinctions preserved
- [x] VerificationClaim/Result pairing preserved
- [x] ReconciliationSummary preserved
- [x] LensEvaluation gaps preserved
- [x] AlignmentAssessment + ID preserved
- [x] MISALIGNED != DENY structurally pinned
- [x] no hidden orchestration
- [x] no authority/capability/instruction coupling
- [x] no global score/verdict
- [x] no provider/LLM/workbook/counterfactual encroachment
- [x] falsification corpus PASS
- [x] full workspace PASS (4644 pass / 0 fail)
- [x] release acceptance PASS (PublicReleaseGate)
- [x] arch-spec-047 updated as partial
- [x] Roadmap Delta written
- [x] archive manifest written
- [x] handoff written

## Roadmap Delta (§14)

```text
baseline:                  v1.169.65
delivered:                 A4-5b AdvisoryContext + WHY integration
not_delivered:             A4-5C acceptance (NEXT, not auto-opened)
next:                      A4-5C
blocked_by:                (A4-5C unblocked)
debt_added:                none
debt_closed:               none (FU-A3-S15-1/4 remain P3, non-blocking)
unexpected_drift:          none
semantic_authorities_added: none
compatibility_added:       AdvisoryKind variants; overlay contract_provenance
                           accessor; EvidenceKind::from_domain_tag codec
compatibility_removed:     none
claims:                    see §24 above
evidence_refs:             corpus, ADR-0128, arch-spec-047, release receipt

before                          after
A4-5a    CLOSED v1.169.64       A4-5a    CLOSED v1.169.64
A4-S15R  CLOSED v1.169.65       A4-S15R  CLOSED v1.169.65
A4-5b    NEXT (unblocked)       A4-5b    CLOSED v1.169.66
A4-5C    blocked_by A4-5b       A4-5C    NEXT (not auto-opened)
```

## Artifacts

- Cycle spec: `.sddk/cycles/p-63676b11dc0ef88f-a4-5b-advisory-context-why-integration/spec.md`
- Seam: `crates/sddk-engine/src/intelligence_advisory/mod.rs`
- Corpus: `crates/sddk-engine/tests/a4_5b_advisory_context_why.rs`
- ADR: `docs/architecture/adrs/ADR-0128-ADVISORY-CONTEXT-WHY-INTEGRATION.md`
- Spec: `docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md` (partial)
- Roadmap: `docs/architecture/README.md`
- Followups: `.sddk/followups/a4-followups.md`

## STOP

A4-5C is **not** auto-opened. The next cycle (A4-5C) owns arch-spec-047
acceptance / receipt / UAT.
