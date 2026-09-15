---
id: ADR-0118-PARADIGM-LENS-EVALUATION
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-8-ac7-paradigm-lenses
implementation_evidence:
  - "crates/sddk-engine/src/paradigm_lens/mod.rs (public surface; advisory-boundary + INFERRED doc)"
  - "crates/sddk-engine/src/paradigm_lens/types.rs (LensFamily 4-closed; LensObservation 16-closed with family+polarity; ObservationPolarity 2-closed; LensEvaluationBasis 2-closed mapping onto AC3 EvidenceBasis; LensProvenance; LENS_VERSION)"
  - "crates/sddk-engine/src/paradigm_lens/lenses.rs (evaluate_lens; inferred_lens_assessment; AC-035-004 status rules)"
  - "crates/sddk-engine/src/paradigm_lens/probes.rs (probe_oo/functional/adt/dsl_observations)"
  - "crates/sddk-engine/src/paradigm_lens/tests.rs (24 tests incl. AC-UAT-011..015)"
  - "docs/architecture/specs/arch-spec-A3-S8-ac7-paradigm-lenses.md (cycle-bounded spec, 22 REQs)"
superseded_by: []
related_adrs:
  - "ADR-0114-PARADIGMS-AS-ALIGNMENT-LENSES"
  - "ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS"
stale_after: 2027-03-14
---

# ADR-0118 — Paradigm Lens Evaluation Is a Separate Capability from Paradigm Data

## Context

ADR-0114 established that programming paradigms are scoped *alignment lenses*
rather than global policy, and A3-S4 (AC3) delivered the paradigm **data**:
`ParadigmProfileKind`, `ParadigmLensKind`, `LensStatus`, `EvidenceBasis`,
`LensAssessment` and the `ParadigmProfileOverlay`. Its cycle spec was explicit
that AC3 is data-only and that **lens evaluation belongs to AC7**.

AC7 then has to add evaluation without breaking AC3's frozen vocabularies. Two
constraints collide:

1. `REQ-AC3-004` pins `EvidenceBasis` to **exactly 5** variants, with a test
   asserting `ALL.len() == 5`.
2. The roadmap requires that an optional LLM evaluation emit an `INFERRED`
   assessment — and `INFERRED` is **not** one of those 5.

## Decision

**1. Evaluation is a separate root module.** `crates/sddk-engine/src/paradigm_lens/`
owns the evaluation capability; `paradigm_profile` stays data-only forever. The
evaluator depends on the profile vocabulary, never the reverse.

**2. AC7 owns a 2-variant evaluation basis and maps onto AC3's frozen one.**

| AC7 `LensEvaluationBasis` | AC3 `EvidenceBasis` |
|---|---|
| `Deterministic` | `Observed` |
| `Inferred` | `Declared` |

`INFERRED` therefore lives in AC7's vocabulary, not AC3's. AC3's enum is
untouched and its variant counts are re-asserted in AC7's test suite.

**3. AC7 owns a 4-variant `LensFamily`.** AC3's `ParadigmLensKind` is 11-closed
and mirrors profile kinds; it has no `Adt` or `Dsl`. Rather than widen AC3's
ontology (which AC-035-003 forbids: "extensible without shared-kernel ontology
explosion"), AC7 defines the four lens families and maps *from* the declared
kinds.

**4. Evaluation is deterministic and advisory.** `evaluate_lens` is a pure
function over observations. It imports no authority/capability/instruction type,
writes no `EffectiveInstructions`, grants no capability, mutates no graph, and
makes no provider call. `inferred_lens_assessment` does not perform inference: it
**validates** caller-supplied provenance and rejects incomplete provenance.

**5. Assessments are scoped.** `evaluate_lens(declared_kind, anchor, ...)`
produces exactly one assessment for one declared anchor. It cannot express a
project-wide or "this codebase is not OO" verdict (AC-UAT-012).

## Consequences

- AC3's "data-only" claim remains true and testable; evaluation can evolve
  (new families, new probes) without touching the profile substrate.
- The `INFERRED` requirement is satisfied without a breaking change to a frozen
  enum, and provenance is mandatory rather than conventional.
- Two vocabularies now describe paradigms: AC3's declared intent and AC7's
  observed families. The mapping between them is explicit and one-directional.
- Probes are deliberately coarse text heuristics. They emit observations, never
  verdicts, and each has a negative control.
- A new root-level engine module requires an ADR under the repository's
  root-module fitness rule; that rule and this ADR agree that adding an
  evaluation capability is an architectural decision.

## Alternatives considered

- **Extend AC3's `EvidenceBasis` with `Inferred`.** Rejected: breaks
  `REQ-AC3-004` and a shipped test, and would let AC3's data substrate drift
  into evaluation semantics.
- **Add the lenses as a `paradigm_profile` submodule.** Rejected: it would
  falsify AC3's "data-only" boundary and entangle a frozen substrate with
  heuristic evaluation.
- **Let AC7 call an LLM.** Rejected for this cycle: it would introduce a
  provider dependency into a module whose whole value is determinism. The
  inferred path is representable and validated; wiring an actual provider is a
  later, optional slice.
- **Have AC7 write AC4's `paradigm_alignment` vector dimension.** Deferred:
  AC7 emits assessments; consuming them is a separate wiring decision.
