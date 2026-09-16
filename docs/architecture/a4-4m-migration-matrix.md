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
