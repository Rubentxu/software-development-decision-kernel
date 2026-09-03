# SPEC-043 — Engineering Profile Protocol

**Status:** Proposed

## Goal

Apply technology-specific engineering rules without leaking technology into workflow intent or kernel ontology.

## Profile example

```yaml
profile_id: engineering.rust.v1
schema_version: 1
kind: language
applies_when:
  any_files: [Cargo.toml]
extends:
  - engineering.systems.v1
knowledge:
  skills:
    - systems-reasoning
    - rust-patterns
    - rust-systems-reasoning
evidence_providers:
  compiler: ["cargo check"]
  lint: ["cargo clippy --workspace"]
  tests: ["cargo test --workspace"]
  optional:
    - "cargo miri test"
    - "cargo fuzz"
    - "kani"
```

## Generic systems profile

Contains language-neutral dimensions:

```text
constraints
invariants
boundaries
control flow
resource liveness
concurrency/backpressure
representation
performance budgets
failure/recovery
verification strategy
```

## Rust profile rules

Activate only where relevant:

- invalid-state/type modeling;
- ownership/lifetime obligations;
- unsafe proof boundaries;
- async cancellation/liveness;
- representation/layout/zero-copy;
- Miri/fuzz/Kani evidence.

"No async in core", zero-copy and formal verification are never universal rules.

## Resolution

```text
explicit override
→ deterministic project manifests/toolchain
→ scope detection
→ generic fallback
```

Polyglot projects may return several scoped profiles.

## Architecture invariant

Adding `engineering.<technology>.v1` cannot require a new WorkflowIR operator, kernel event primitive, CycleState variant or language enum in kernel.
