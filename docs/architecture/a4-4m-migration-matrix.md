# A4-4M M0 — Migration Matrix (AC7 → Generic AlignmentLens)

> **Cycle:** `p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence`
> **Gate:** mandatory before ANY `paradigm_lens/` production edit.
> **Status:** DRAFT for review — pin tests listed in §5 accompany it.

## 1. Inventories

### AC7 side (`crates/sddk-engine/src/paradigm_lens/`, 1312 LOC incl. tests)

| Symbol | Kind | Notes |
|---|---|---|
| `LensFamily` | enum(4) | OO / Functional / Adt / Dsl; maps `ParadigmLensKind` (11) → family |
| `LensObservation` | enum(16) | 4 per family; static `polarity()` + static `locator()` |
| `ObservationPolarity` | enum(2) | Supports / Contradicts |
| `LensEvaluationBasis` | enum(2) | Deterministic / Inferred |
| `LensProvenance` | struct | evaluator, model, input_digest, lens_version |
| `LensEvaluation` | struct | assessment + basis + provenance + used_observations |
| `evaluate_lens` | fn | deterministic evaluation; `status_from_polarities` inside |
| `inferred_lens_assessment` | fn | validates caller provenance; ZERO external consumers |
| `status_from_polarities` | fn | the legacy status motor |
| `probe_*_observations` | fn(4) | source → typed Vec<LensObservation> |
| `evidence_ref()` | method | EvidenceRef(Planning, static locator) — **provenance only, never parsed back** |

### Kernel side (A4-4a/b/bR surfaces)

| Symbol | Kind |
|---|---|
| `UniversalConcern` | enum(10) |
| `ApplicableConcern` | enum: Applicable / NotApplicable (intent layer only) |
| `SoftwareObservation` | struct: subject, stance(Affirms/Denies), evidence, origin, basis, freshness, producer |
| `ObservationSubject` | enum(4): Relation / Unit / Contract / Knowledge |
| `EvidencePosture<T>` | enum(4): Supported / Contradicted / Conflicted / Insufficient(InsufficientGap) |
| `ObservationTargetRef` | enum(4) |
| `AlignmentLens` / `LensContribution` / `LensContributionId` | trait / struct / content-addressed id |
| `AlignmentLensRegistry` / `AlignmentLensKernel` | registry + total-function kernel |

## 2. M0.1 — LensObservation → canonical observation (typed, lossless)

**Key structural fact:** AC7's `LensObservation` variants are *static*
(the variant IS the semantic discriminant; `polarity()` and `locator()`
are pure functions of the variant). Therefore the lossless translation
does NOT need to round-trip through any string:

```text
probe(source) → Vec<LensObservation>          (typed, already exists)
      ↓ translation fn (one match, exhaustive)
SoftwareObservation {
    subject:  ObservationSubject::Unit(target_ref),      // typed; NO synthetic relation
    stance:   polarity→stance (Supports→Affirms, Contradicts→Denies),
    evidence: observation.evidence_ref(),                // provenance citation only
    origin:   ObservationOrigin (deterministic probe id),
    basis:    ObservationBasis (static analysis),
    freshness: None,                                     // never fabricated
    producer: "ac7.probe.<family>",
}
      ↓ ObservationSet
resolve_subject(Unit(target)) → EvidencePosture<Unit-target>
      ↓ production lens reads the TYPED observation stream (M1)
LensContribution (LensEvidenceResolution)
```

The semantic discriminant survives twice, redundantly and typed:
(1) the `ObservationStance` carries the polarity; (2) the producer tag
+ evidence locator carry the *dimension*. A production lens never needs
to parse the locator because the probe→observation translation is
exhaustive over the closed 16-variant enum: the lens receives the
typed `LensObservation` provenance via the observation's `origin`/
`producer` fields OR (preferred, M1) lenses are implemented *over the
typed probe output* and emit contributions per dimension. No
`contains("hidden_mutation")` string matching anywhere.

**Verdict: lossless translation possible with current primitives. NO
substrate extension required.**

## 3. M0.2 — LensObservation → UniversalConcern (16 rows, pinned)

See `spec.md` §M0.2 table. Each row maps to ≥1 closed `UniversalConcern`
variant; NO new concern is invented; NO row is ambiguous enough to
force unmapped escalation. The mapping is pinned by a test asserting
the exhaustive table (compile-time: `match` without wildcard; runtime:
table equality against the pinned expectation).

Note on family→concern orientation: AC7 assesses *families* against a
declared profile; A4-4 assesses *concerns*. The bridge is per-observation
(each of the 16 names a concrete design dimension), not per-family.
The paradigm identity (which family's lens) lives in the
`AlignmentLens` implementation (M1) and in `LensContributionId`'s lens
identity — NOT in the concern vocabulary.

## 4. M0.3 — legacy status equivalence (compatibility projection)

```text
EvidencePosture::Supported    → LensStatus::Aligned
EvidencePosture::Contradicted → LensStatus::Misaligned
EvidencePosture::Conflicted   → LensStatus::Tension
EvidencePosture::Insufficient → LensStatus::Unknown
```

`NotApplicable`: produced ONLY by the intent layer
(`ApplicableConcern::NotApplicable(c, reason)`), corresponding to AC7's
undeclared-family case (`family_for_kind → None`). The kernel never
returns it (pinned since A4-4b, re-pinned here for the facade).

Equivalence with legacy rules (AC-035-004):

| Legacy rule | New path |
|---|---|
| undeclared family → NotApplicable | intent layer: concern not in `applicable_concerns()` |
| zero observations → Unknown | `resolve_subject` → `Insufficient{NoObservation}` → projection Unknown |
| all Supports → Aligned | `Supported` → Aligned |
| all Contradicts → Misaligned | `Contradicted` → Misaligned |
| mixed → Tension | `Conflicted` (preserves BOTH sets) → Tension |

Order sensitivity: legacy `status_from_polarities` is count-based
(order-independent for status). The kernel's posture is set-based
(order-independent for class). Equivalent.

## 5. Pin tests accompanying this matrix (M0 gate evidence)

To be added under `crates/sddk-engine/tests/a4_4m_m0_migration_proof.rs`:

| Pin | Asserts |
|---|---|
| M0-P01 | translation fn is exhaustive over `LensObservation::ALL` (no wildcard match) |
| M0-P02 | for every variant: stance == polarity projection (Affirms⟺Supports) |
| M0-P03 | for every variant: subject is `Unit(_)` — never a fabricated `SoftwareRelation` |
| M0-P04 | for every variant: `evidence_ref()` locator is provenance-only; translating does NOT read it (compilation-level: translation takes the typed enum) |
| M0-P05 | concern mapping table == pinned expectation, all 16 rows |
| M0-P06 | posture→LensStatus projection: the 4 rows of §4 |
| M0-P07 | Insufficient(no obs) → Unknown == legacy zero-observations Unknown |
| M0-P08 | mixed support/contradict → Conflicted → Tension == legacy mixed Tension |
| M0-P09 | `NotApplicable` cannot be constructed from any `EvidencePosture` (type-level) |
| M0-P10 | order permutation of observations does not change posture class |

## 6. M0.4 — inferred path / consumers verdict

Local evidence (workspace grep + framework bundle grep):

- `evaluate_lens`: consumed by `tests/ac7_lens_over_ac3_profile.rs`,
  `tests/ac8_full_chain_receipt.rs`, `src/architecture_receipt/tests.rs`
  → **LEGACY_READ_COMPAT**: facade with removal trigger, zero evaluation
  logic of its own (M4).
- `inferred_lens_assessment` + `LensEvaluationBasis::Inferred` path:
  **DEAD** (no consumer outside paradigm_lens) → DELETE in M10.
- `status_from_polarities`: **REPLACE** — dies as authority in M5.
- `probe_*_observations`: **KEEP_CANONICAL-transitional** — probes keep
  emitting typed observations; their output lands in `ObservationSet` (M2).

## 7. M0 verdict

**GO.** The typed translation is lossless with current primitives:
no locator parsing, no synthetic relations, no new concerns, no new
taxonomy, no AC7 heuristic changes. Proceed to M1–M10 under the same
budget, per spec.

## 8. M11 — Concern-Preserving ParadigmLens Evaluation (corrective slice)

> **Cycle:** `p-63676b11dc0ef88f/a4-4mr-concern-preserving`
> **Closes:** `docs/debt/FU-A4-4M-CONCERN-PRESERVATION.md` (P1)
> **Status:** CLOSED — see release tag in
> `docs/handoff/HANDOFF-2026-09-17-a4-4mr-concern-preserving.md`.

The production `ParadigmLens::evaluate(&LensInput)` (the four lenses
in `crates/sddk-engine/src/alignment_lens/paradigm.rs`) iterated
`self.concerns`, overwrote `out` on each iteration, and returned only
the **last** contribution. As a result the contribution's `concern`
field carried the lens's terminal declared concern, not the
caller-requested concern in `LensInput::applicable`. The contribution
identity was likewise wrong because `derive_contribution_id` hashed
the (incorrect) concern into the content address.

A4-4M did not catch the bug because no pin in
`crates/sddk-engine/tests/a4_4m_convergence_pins.rs` exercises a
production `ParadigmLens` through `AlignmentLensKernel::evaluate` —
the corpus covers registry composition (M3), legacy motor removal
probes (M5), legacy facade equivalence (M7), provenance (M9), and
inferred-path deletion (M10). The M7 corpus invokes the
`evaluate_lens` LEGACY_READ_COMPAT facade, which does not
instantiate `AlignmentLens` and therefore never hit the buggy
`for concern in self.concerns` loop.

A4-4MR fixes the bug locally in `ParadigmLens::evaluate`:

- `let concern = input.concern();` is the ONLY concern the lens
  considers.
- If `!self.concerns.contains(&concern)`: return
  `LensEvaluationOutcome::Refused(LensError::LensRejected { id,
  self.id, concern })`.
- Otherwise assemble exactly ONE `LensContribution` carrying
  `contribution.concern == concern` and identity
  `derive_contribution_id(self.id, versions::V1, concern, ...)`.

The legacy `paradigm_lens::evaluate_lens()` compatibility facade is
unchanged — it has never instantiated `AlignmentLens` and never
called `AlignmentLensKernel`. Its `LensStatus` is a substrate-posture
projection, not a concern-tagged answer. The production-vs-facade
boundary is documented in the `paradigm_lens/lenses.rs` module
header.

Pin corpus: `crates/sddk-engine/tests/a4_4mr_concern_preserving.rs`
(16 pins across seven pin families: exhaustive positive,
aggregate-for-declared/aggregate-for-undeclared, defence-in-depth
refusal, multi-lens invariants, posture-class falsification,
content-identical determinism, registry proof UAT, and explicit
`derive_contribution_id` match). The corpus is verified to fail
(13/16 red) against the pre-fix code, then to pass (16/16 green)
against the post-fix code.
