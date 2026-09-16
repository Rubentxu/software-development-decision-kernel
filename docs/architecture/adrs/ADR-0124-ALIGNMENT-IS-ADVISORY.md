---
id: ADR-0124-ALIGNMENT-IS-ADVISORY
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a4-0-verification-provenance-foundation
implementation_evidence:
  - "crates/sddk-domain/src/workflow_run.rs (AdvisoryContext, ContextCapsule — A3 closeout)"
  - "crates/sddk-cli/src/instruction_compiler.rs (acceptance_advisory_context_does_not_change_instruction_identity)"
  - "docs/architecture/specs/arch-spec-045-software-alignment-domain.md (contract-ready)"
  - "docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md (contract-ready)"
  - "crates/sddk-engine/src/intent_universal_concern/ (A4-4a — closed 10-member UniversalConcern + typed Intent + pure applicable_concerns() reducer; descriptive, no authority)"
  - "crates/sddk-engine/src/software_alignment/ (A4-3 — closed 7-state AlignmentState + 3-finding reducer; no_findings != ALIGNED invariant)"
superseded_by: []
related_adrs:
  - "ADR-0122-EVIDENCE-OBSERVES-SOFTWARE"
  - "ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT"
stale_after: 2027-03-15
---

# ADR-0124 — Alignment is advisory, and that is now executable rather than aspirational

## Context

The historical context-first package already declares alignment advisory, and
already carries the model worth keeping: seven assessment statuses
(`ALIGNED | TENSION | MISALIGNED | ACCEPTED | REVIEW_DUE | UNKNOWN |
NOT_APPLICABLE`), three finding kinds (`ContractViolation | AlignmentTension |
ImprovementOpportunity`) with `ArchitecturalIntentSnapshot` as an input, and the
rule that a `ContractViolation` requires an explicit contract — heuristic
disagreement is never automatically a violation.

What was missing was an **executable** guarantee. A3's closeout delivered it:

- `AdvisoryContext` is a closed typed payload (`AdvisoryKind::ParadigmObservation |
  AlignmentTension | KnowledgeStatus`) with a provenance class;
- `ContextCapsule` keeps `context_capsule_hash` **separate** from
  `effective_instruction_set_hash`;
- the separation is proven against the real `InstructionCompiler`: a different
  advisory payload changes the capsule identity and provably cannot change the
  instruction identity;
- `AdvisoryContext` has no conversion into `InstructionSource`, cannot grant a
  `Capability`, and does not reach the authority engine.

## Decision

**1. Alignment feeds `AdvisoryContext`, never normative instructions.** Only a
context with real authority can originate a `MUST`.

**2. `ContractViolation` requires an explicit contract or invariant.** A heuristic
smell is a `TENSION` or an `ImprovementOpportunity`, never a violation.

**3. Four classes never merge:** `fact != assessment != recommendation !=
authority`.

**4. Governance alone converts policy into a gate.** The loop never contains
`MISALIGNED → DENY`: alignment computes a delta and presents attention; Governance
evaluates explicit policy afterwards.

**5. No universal quality score.** Contradictions resolve to
`EvidenceResolution`-style states, never to a confidence number.

**6. A4-0 implements no alignment.** It relies on the A3-executable advisory
boundary and leaves A4-3/A4-4 to implement the domain and lenses on top of the
observation substrate.

## Consequences

- An alignment conclusion is traceable back to the relations and evidence that
  produced it, which is what keeps Software Alignment from becoming another
  "intelligent analyzer" whose opinions cannot be justified.
- The advisory boundary does not need to be re-litigated in A4-3; it is pinned.

## Alternatives considered

- **Let alignment emit normative instructions.** Rejected: it would make an
  interpretation authoritative, and it is precisely what A3's advisory fixture
  proves cannot happen.
- **Treat smells as violations.** Rejected by the historical rule and reasserted
  here: without an explicit contract there is nothing to violate.
- **Score alignment (`confidence = 0.73`).** Rejected: SDDK resolves to closed
  states and preserves contradictions.

---

## Addendum (2026-09-16): A4-4a — Intent + UniversalConcern model

**Scope:** This addendum extends the ADR to cover the new
`intent_universal_concern` module shipped in A4-4a. It does **not**
modify the original advisory boundary; it adds a *descriptive*
kernel surface on top of it.

### Intent

A4-4a establishes the **closed 10-member `UniversalConcern` vocabulary**
(Cohesion / Coupling / BoundaryIntegrity / StateSafety / EffectVisibility /
DependencyDirection / SemanticOwnership / TemporalCoupling / Testability /
Freshness) plus typed intent representation (`ProjectIntent`, `UnitIntent`,
`IntentId`, `ParadigmProfileRef`) and a pure `applicable_concerns()`
reducer.

### Decision (A4-4a)

**7. Descriptive, not prescriptive.** `applicable_concerns()` returns a
`Vec<(ApplicableConcern, Option<ApplicableReason>)>`. The answer is *what
is worth looking at for this unit, given the declared intent* — never a
capability grant, denial, or authority decision. The function is pure:
no IO, no wall clock, no global state, no mutation of `paradigm_lens`.

**8. `MISALIGNED ≠ DENY` (architectural invariant, restated).** The
`ApplicableConcern` enum has only two variants: `Applicable(c)` and
`NotApplicable(c, NotApplicableReason)`. Neither carries authority.
Authority is upstream (ADR-0123 / A4-3) and downstream (Governance,
ADR-0124 §4).

**9. Paradigm × concern relevance table is closed.** Adding a concern
or paradigm variant requires a deliberate breaking change to
`paradigm_supports_concern()` in `reducer.rs`.

**10. Anti-encroachment for A4-4a.** A4-4a ships *only*: types, reducer,
tests, lib.rs wire-up, and an ADR addendum. It does **not** ship:
`AlignmentLens` trait, any concrete lens strategy, any wiring into
`software_alignment::reduce_alignment`, any CLI surface, any
authority/capability wiring, or any mutation of the existing
`paradigm_lens` registry. These are A4-4b / A4-4M / A4-5 work.

### Implementation evidence (A4-4a)

- `crates/sddk-engine/src/intent_universal_concern/mod.rs` — public
  surface: 11 named exports, none of which are authority-shaped.
- `crates/sddk-engine/src/intent_universal_concern/types.rs` — closed
  vocabularies + typed accessors (271 lines).
- `crates/sddk-engine/src/intent_universal_concern/reducer.rs` —
  pure reducer (166 lines).
- `crates/sddk-engine/src/intent_universal_concern/tests.rs` —
  18 tests: 1 happy path, 1 UAT, 6 edge cases, 7 anti-encroachment,
  3 identity/determinism.
- `crates/sddk-engine/src/lib.rs` — single line wire-up
  (`pub mod intent_universal_concern;`).

### Consequences (A4-4a)

- A4-4b can now define `AlignmentLens` as a trait that *consumes*
  `ApplicableConcern` answers without re-deriving the vocabulary.
- A4-4M can converge the existing `paradigm_lens` registry to use the
  A4-4a `ParadigmProfileRef` as its reference surface without
  breaking A4-4a's typed identity.
- The UAT pin ("same software + different intent = different
  applicable concerns possible") is testable today; future cycles
  must not regress it.
