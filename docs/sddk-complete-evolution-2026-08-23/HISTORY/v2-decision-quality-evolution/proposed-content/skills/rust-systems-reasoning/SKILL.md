---
name: rust-systems-reasoning
description: "Trigger: Rust systems review, unsafe, zero-copy, async boundaries, lifetimes, fuzzing, Kani. Add Rust-specific assurance obligations."
license: MIT
metadata:
  author: "Rubentxu"
  version: "1.0"
---

## Activation Contract

Use only when a systems/architecture review targets Rust and Rust-specific correctness/performance mechanisms matter.

## Hard Rules

- Load `systems-reasoning` for generic procedure.
- Reuse `rust-patterns`; do not duplicate it.
- Zero-copy, unsafe and formal methods are conditional tools.
- Require explicit evidence for unsafe/layout claims.
- Do not impose "no async in core"; require a traceable reason for suspension/ownership complexity.

## Decision Gates

| Surface | Check |
|---|---|
| unsafe/FFI/casts | safety proof obligations |
| binary/layout/zero-copy | layout/alignment/validity/lifetime |
| async/channels/locks | cancellation, liveness, backpressure |
| parser/state machine | property/fuzz evidence |
| compact critical invariant | consider Miri/Kani/model checks |

## Execution Steps

1. Apply generic systems review.
2. Map relevant invariants to Rust types/ownership where useful.
3. Inspect activated Rust surfaces only.
4. Recommend proportional compiler/test/fuzz/Miri/Kani evidence.
5. Return Rust-specific deltas.

## Output Contract

Return Rust-specific obligations, evidence gaps and findings without repeating generic assessment.

## References

- `references/rust-assurance.md`
