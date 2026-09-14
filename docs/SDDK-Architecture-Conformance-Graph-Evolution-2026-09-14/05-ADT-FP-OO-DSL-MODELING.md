# ADTs, FP, OO and DSL Modeling Guidance

## Purpose

This document turns paradigm ideas into SDDK modeling opportunities without imposing implementation fashion.

## ADTs as executable domain boundaries

Prefer closed algebraic models where the domain is actually closed:

```rust
enum VerificationResult {
    Verified(VerifiedEvidence),
    Contradicted(Contradiction),
    Unknown(EvidenceGap),
    Stale(StaleBasis),
}
```

This is preferable to combinations such as:

```text
passed: bool
error: Option<String>
stale: bool
provider_missing: bool
```

when those combinations allow contradictory states.

### Candidate SDDK domains

- contract kinds;
- claim/verification outcomes;
- evidence status;
- provider capability negotiation;
- paradigm profiles/lenses;
- counterfactual impact;
- mutation probe result;
- conformance receipt disposition.

## Functional core / effectful shell

A valuable default for Verification/Alignment is:

```text
collect Evidence     ← effects/providers/filesystem
        ↓
pure normalization
        ↓
pure claim evaluation
        ↓
pure assessment/diff
        ↓
write receipt/event  ← effect boundary
```

This yields deterministic fixtures, replayability and easier mutation tests.

## OO/domain encapsulation where useful

Objects/aggregates remain useful when ownership and lifecycle are the important abstraction:

- revision ref with CAS invariant;
- authority policy registry;
- provider session lifecycle;
- agentic session binding;
- stateful gateway/resource manager.

The lens should ask whether encapsulation protects a real invariant, not whether code contains `struct`/methods.

## Typed DSL architecture

SDDK SHOULD support DSLs through a reusable compiler architecture:

```text
Authoring DSL
   ↓ parse/build
Typed AST/ADT
   ↓ validate
Normalized IR
   ↓ plan/compile
ExecutablePlan / VerificationPlan / QueryPlan
   ↓ authority/effects
Runtime adapters
```

Potential DSLs:

- architecture contract DSL;
- graph query/WHY DSL;
- verification-plan DSL;
- paradigm/lens configuration DSL;
- policy DSL only if future evidence justifies it;
- pack-defined typed domain DSLs through Extension Platform.

## Internal vs external DSL

Use internal typed builders when host-language type safety gives leverage. Use external declarative DSLs where human portability/configuration matters. Both SHALL compile to the same typed IR rather than owning separate execution semantics.

## Capability safety

Knowing a DSL command is not authority to execute it:

```text
DslProgram
  → validated IR
  → requested effects
  → AuthorityEngine
  → Allow / Deny / RequireApproval
```

This reuses `Skill != Capability` and Command Registry principles.

## Property-based and law-oriented tests

High-value laws include:

- parse/print or build/normalize stability;
- deterministic compilation;
- semantic equivalence of internal/external syntax;
- exhaustive ADT handling;
- invalid state generators rejected;
- pure evaluator result invariant under input ordering where ordering is irrelevant.
