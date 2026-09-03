# SPEC-043 — Engineering Profile Protocol

**Status:** Proposed

## Goal

Specialize Engineering Assurance for languages, runtimes and project shapes without leaking those details into workflow capability IDs or the kernel.

## Profile schema

```yaml
profile_id: engineering.rust.v1
schema_version: 1
kind: language
applies_when:
  any_files:
    - Cargo.toml
  ecosystems:
    - rust
extends:
  - engineering.systems.v1

knowledge:
  skills:
    - systems-reasoning
    - rust-patterns
    - rust-systems-reasoning

dimensions:
  architecture:
    enabled: true
  state_invariants:
    enabled: true
  concurrency:
    enabled: true
  representation:
    enabled: conditional
  performance:
    enabled: conditional
  verification:
    enabled: true

evidence_providers:
  compiler:
    commands: ["cargo check"]
  lint:
    commands: ["cargo clippy --workspace"]
  tests:
    commands: ["cargo test --workspace"]
  memory:
    optional: ["cargo miri test"]
  fuzz:
    optional: ["cargo fuzz"]
  formal:
    optional: ["kani"]

rules:
  - id: rust.invalid-state-representation
    dimension: state_invariants
  - id: rust.unsafe-boundary-proof
    dimension: representation
  - id: rust.async-boundary-justification
    dimension: concurrency
```

## Generic systems profile

`engineering.systems.v1` contains language-neutral reasoning dimensions:

1. constraints;
2. invariants;
3. boundaries and dependency direction;
4. representation;
5. execution/control flow;
6. resource ownership/liveness;
7. concurrency/backpressure;
8. performance budgets;
9. failure/recovery;
10. verification strategy.

It MUST NOT mention Rust-specific tools or types.

## Rust reference profile

Rust v1 applies additional rules.

### Type/invariant modeling

Prefer compile-time expression when practical:

- newtypes;
- enums;
- typestate;
- ownership/lifetimes;
- non-zero/range constrained types.

This is a review heuristic, not a mandate to encode every business rule at compile time.

### Unsafe

For every material unsafe boundary, seek evidence for:

- preconditions;
- validity;
- alignment;
- aliasing;
- lifetime;
- mutation/concurrency;
- safe caller contract.

### Zero-copy

Zero-copy checks activate only when representation/performance makes them relevant. The profile MUST NOT recommend zero-copy by default.

### Async/concurrency

Prefer explicit, traceable control flow. Review whether suspension belongs to domain semantics or merely an infrastructure adapter. Do not impose "no async in core" as a universal rule.

### Verification ladder

Select evidence proportionally:

```text
type/compiler guarantees
→ unit/integration
→ property tests
→ fuzzing
→ Miri/sanitizers
→ model/formal checking where tractable
```

Kani is optional and targeted, never a blanket gate.

## Polyglot projects

Profile resolver returns a set:

```yaml
profiles:
  - profile_id: engineering.rust.v1
    scopes: ["backend/**"]
  - profile_id: engineering.typescript.v1
    scopes: ["frontend/**"]
```

Cross-language architecture capabilities consume all applicable profiles but keep one semantic capability ID.

## Determinism

Profile resolution SHOULD use deterministic repository signals. An LLM can propose additional profiles, but policy/validation records why they apply.

## Versioning

Profile IDs are immutable by major schema semantics. Breaking rule meaning requires a new profile version.
