# Paradigm Lens System

## Principle

Software Alignment SHALL remain paradigm-neutral at the platform level while supporting paradigm-aware lenses selected by project intent.

SDDK must never encode:

```text
functional = good
object oriented = bad
```

or the inverse.

Instead:

```text
ProjectIntent + ParadigmProfile + Evidence → LensAssessment
```

## ParadigmProfile

A project, bounded context or software unit MAY declare one or more intended paradigms:

```text
ObjectOriented
Functional
FunctionalPure
DataOriented
EventDriven
Reactive
Hexagonal
DDD
Pipeline
ActorLike
Custom
```

Profiles can coexist. A Rust domain module may prefer ADTs/pure functions while an adapter uses stateful OO-like encapsulation.

## Object-oriented lens

Possible evidence/heuristics:

- encapsulation and invariant ownership;
- aggregate boundaries;
- tell-don't-ask vs anemic transaction scripts;
- composition vs inheritance where relevant;
- dependency inversion;
- stable interfaces and substitutability;
- connascence and change locality;
- domain behavior colocated with the data/invariants it owns.

These are assessment dimensions, not universal failures.

## Functional and pure-functional lens

Possible evidence/heuristics:

- referential transparency where declared;
- explicit effects at boundaries;
- immutable values;
- total functions / explicit error ADTs;
- deterministic transformations;
- pure core / imperative shell;
- effect isolation;
- composition over hidden mutation;
- absence of sentinel/null/stringly control flow where typed alternatives exist.

## ADT lens

Detect opportunities and regressions around:

- invalid states representable;
- boolean blindness;
- unrelated optional fields forming implicit state machines;
- stringly typed identifiers/status/action names;
- enum variants with illegal combinations;
- exhaustiveness lost through generic maps/JSON too early;
- failure semantics compressed into `bool`/`Option` when richer sum types are required.

## DSL lens

Evaluate whether a DSL:

- has an explicit typed model/AST/IR;
- separates authoring syntax from execution effects;
- validates before execution;
- supports deterministic compilation;
- keeps capabilities/authority outside mere syntax knowledge;
- makes invalid programs difficult or impossible to represent;
- has versioned compatibility and actionable diagnostics;
- can be tested without invoking external effects.

## Lens status

Use the existing Alignment vocabulary:

`ALIGNED | TENSION | MISALIGNED | ACCEPTED | REVIEW_DUE | UNKNOWN | NOT_APPLICABLE`.

No lens may write `EffectiveInstructions`, grant capabilities or bypass AuthorityEngine.

## Lateral idea: paradigm topology

The SemanticGraph can expose where paradigms are intended and where they cross:

```text
bounded context → USES_PARADIGM → FunctionalPure
adapter          → USES_PARADIGM → ObjectOriented
workflow DSL     → USES_PARADIGM → TypedDSL
```

This makes poly-paradigm architecture explicit rather than forcing one global style.
