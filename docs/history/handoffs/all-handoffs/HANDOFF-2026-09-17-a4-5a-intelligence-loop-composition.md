# Handoff — A4-5a — Intelligence Loop Composition (v1.169.64)

> Date: 2026-09-17
> Cycle: `p-63676b11dc0ef88f/a4-5a-intelligence-loop-composition`
> Spec:  `arch-spec-047` part-1
> Release tag: `v1.169.64` → SHA `2135e6026cdf69b6041e230714d06374f43f60d1`

## What shipped

A4-5a closes the intelligence-loop composition seam. After this cycle, the
SDDK engine has a single content-addressed binding for the four existing
authority outputs:

- `KnowledgeBasis` (content-addressed anchor).
- `ObservationSet` (content-addressed anchor).
- `Vec<(VerificationClaim, VerificationResult)>` (paired per existing
  convention).
- `ReconciliationSummary` (preserved verbatim — closed enum, 6 variants).
- `LensEvaluation` (preserved complete: contributions + gaps, with
  `NotEvaluated { reason: NoRegisteredLens | LensExistsButRefused }`
  surviving).
- `AlignmentAssessment` (preserved verbatim, with `AlignmentAssessmentId`
  from `reduce_alignment::derive_id`).

Two new public types:

- `IntelligenceLoopResult` (EPHEMERAL) — side-by-side bundle, no
  derivation, no verdict.
- `IntelligenceLoopReceipt` (PROJECTION) — content-addressed +
  order-independent, rebuildable.

One new module: `crates/sddk-engine/src/intelligence_loop/mod.rs`.

## Receipt content-addressing

`derive_receipt_id(&IntelligenceLoopInputs)`:

```text
sha256(
  "sddk.intelligence_loop.receipt.id.v1|"
  || "k|" || knowledge_basis.basis_hash().to_hex()
  || "o|" || observation_set.canonical_digest()
  || "v|" || sorted verification-pair digests
  || "b|" || debverify_baseline_hash.as_str()
  || "r|" || reconciliation semantic digest
  || "c|" || sorted LensContributionId set
  || "g|" || sorted gap tags
  || "a|" || alignment_assessment.id.as_str()
)
```

`evaluation_time` is carried in the receipt but is **NOT** in identity.
Two compositions at different times over the same semantic inputs produce
the same id.

## Hard non-goals (honoured)

- NO `AuthorityDecision`, `Capability`, `InstructionSource` consumption.
  `MISALIGNED ≠ DENY` is structurally pinned.
- NO overall verdict, NO `status`/`health`/`score`/`verdict`/`confidence`
  /`risk_score`/`pass`/`fail`. `IntelligenceLoopResult` has no such fields.
- NO AdvisoryContext, NO WHY/WHY-NOT (A4-5b).
- NO Governance.
- NO mutation of any authority's types.
- NO call to legacy `paradigm_lens::evaluate_lens()`.
- NO mutation of `FU-A3-S15-3` (stays `BLOCKS_A4-5b` only).

## Test corpus

16 pins across 7 families in
`crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`:

- **P1 happy**: full chain through real `AlignmentLensKernel` +
  `reduce_alignment`; deterministic across N invocations.
- **P2 unknown/gap**: kernel returns
  `LensEvaluation { contributions: [], gaps: [NotEvaluated { reason:
  NoRegisteredLens }] }`; gap preserved; receipt changes.
- **P3 contradictory**: `ReconciliationSummary::Contradiction` +
  `VerificationResult::Contradicted`; both preserved verbatim.
- **P4 misaligned-without-deny**: `AlignmentState::Misaligned` in
  assessment; NO authority signal surfaced.
- **P5 lens gap preserved end-to-end**.
- **P6 concern preservation regression (A4-4MR)**.
- **P7 ordering independence** (verification pairs + contributions).
- **P8 receipt sensitivity per stage** (basis, observation, claim,
  baseline, reconciliation, contribution, gap, assessment).
- **P9 semantic-only stability** (Verified vs Stale; contract_id change).
- **P10 no overall status** (debug-repr check).
- **P11 no authority dependency** (type-level + debug-repr check).
- **P12 production lens kernel path** (registry + `ParadigmLens::ALL`).
- **P13 legacy facade not used** (source-grep, line-by-line, ignoring
  `//` comments).
- **P14 determinism** (100 invocations, same id).
- **P15 real UAT through `VerifyKernel::evaluate`** with a test
  `VerificationDomain` adapter that returns `Verified`.
- **P16 baseline + scope anchor end-to-end**.

All 16 pins green after the fix.

## Anti-encroachment pins

- `IntelligenceLoopInputs` does NOT contain `AuthorityDecision`,
  `Capability`, `InstructionSource` (P11).
- `IntelligenceLoopResult` has no `status`, `health`, `score`, `verdict`,
  `confidence`, `risk_score`, `pass`, `fail`, `ready` field (P10).
- Legacy `paradigm_lens::evaluate_lens()` is not referenced from the
  corpus (P13).
- Production `AlignmentLensKernel` + production `ParadigmLens::ALL`
  exercised end-to-end (P6, P12, P15).

## Acceptance gates (all PASS)

- `cargo fmt --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace` green (multiple suites; one flaky
  `sddk-storage` concurrency test passed on isolated re-run).
- PublicReleaseGate (step 9b) PASS — tag SHA anchored via
  `git ls-remote origin v1.169.64` = `2135e6026cdf69b6041e230714d06374f43f60d1`;
  `isDraft=false`, `isPrerelease=false`; 9-asset contract HTTP 200.
- Local install at `~/.local/share/sddk/framework/1.169.64/` updated;
  `sddk dev doctor` reports `binary.bundle_coherence: present` +
  `all_present: true`.

## Files changed

- `crates/sddk-engine/src/lib.rs` — `pub mod intelligence_loop;`.
- `crates/sddk-engine/src/intelligence_loop/mod.rs` — new module
  (~440 lines including docs + smoke test).
- `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs` —
  new corpus (~770 lines).
- `docs/architecture/adrs/ADR-0126-INTELLIGENCE-LOOP-COMPOSITION-SEAM.md`
  — new ADR.
- `docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md` —
  `implemented_by: partial — A4-5a composition shipped (v1.169.64);
  AdvisoryContext + WHY pending A4-5b; acceptance pending A4-5C` + new
  "A4-5a composition seam" section.
- `docs/architecture/README.md` — A4-5a row → CLOSED with SHA +
  handoff + corpus references.
- `.sddk/followups/a4-followups.md` — FU-A4-5A-COMPOSITION-BOUNDARY
  (P1, CLOSED) + FU-A4-5A-CYCLE rows.
- `Cargo.toml` — `version = "1.169.64"`.

## Commits (this cycle, main linear)

```
2135e60 chore(release): bump version (post-release marker; this commit)
416d983 feat(engine): A4-5a — Intelligence Loop Composition seam
```

HEAD = `2135e60` = tag `v1.169.64` SHA.

## Discovery

The cycle surfaced three structural invariants worth pinning:

1. **Identity-relevant `ContradictionReason::Custom(String)` payload**:
   the corpus P9 asserts that `Verified` ↔ `Contradicted` is a
   semantic change. The `ContradictionReason::Custom(String)` variant
   currently contributes to identity through its `Display`. This is
   identity-relevant but uses `Display` text — borderline. Flagged
   for A4-5b review (should it use a typed `Custom` payload rather
   than `String`?).
2. **`KnowledgeBasis` empty-but-different-time**: two empty bases at
   different `EventTime` produce identical `basis_hash` (the hash is
   over content, not over time). The corpus P8 (a) flip uses an
   `assertion.insert` to make the hash actually change. Worth
   documenting in the A4-5b advisory context.
3. **`ParadigmLens::ALL` registered four times**: the corpus P12
   registers all four production lenses. The A4-4bR/A4-4M migration
   matrix shows that legacy `paradigm_lens::evaluate_lens()` is now a
   pure compatibility facade and the production path is
   `AlignmentLensRegistry` + `AlignmentLensKernel`. P13 confirms the
   corpus does not call the legacy facade.

## Follow-ups

- `FU-A4-5A-COMPOSITION-BOUNDARY` (P1) — **CLOSED by this cycle**.
- `FU-A4-5A-CYCLE` — **CLOSED — cycle `p-63676b11dc0ef88f-a4-5a-intelligence-loop-composition`**.
- `FU-A3-S15-3` — stays `BLOCKS_A4-5b` only; does NOT block A4-5a.
- A4-5b is **blocked** until `FU-A3-S15-3` resolves AND A4-5a is closed
  (now satisfied). A4-5b requires a fresh cycle + new ROADMAP-SYNC.
- A4-5C `arch-spec-047` acceptance gate is `blocked_by A4-5b`.

## Next cycle

> **STOP.** Do NOT auto-open A4-5b. The user has explicitly confirmed
> the stop rule for this milestone. A4-5b requires a fresh cycle,
> a fresh ROADMAP-SYNC preflight, and resolution of `FU-A3-S15-3`
> before it can begin.

## See also

- `.sddk/cycles/p-63676b11dc0ef88f-a4-5a-intelligence-loop-composition/spec.md`
- `docs/architecture/adrs/ADR-0126-INTELLIGENCE-LOOP-COMPOSITION-SEAM.md`
- `docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md`
- `docs/architecture/README.md` — A4-5a row
- `crates/sddk-engine/src/intelligence_loop/mod.rs`
- `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`
- `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-a4-5a-intelligence-loop-composition/`
  (archive-manifest stashed here)
