# Rust Assurance Reference

## Invariant expression

Consider when it improves clarity/correctness: newtypes, enums, typestate, ownership/lifetimes, constrained constructors/NonZero. Do not encode genuinely dynamic business state into type machinery merely to follow a pattern.

## Unsafe proof

Record operation, preconditions, who establishes them, layout/alignment/validity, aliasing/mutability, lifetime/liveness, concurrency assumptions and verification evidence.

## Representation

Recommend zero-copy only when workload benefit and layout/alignment/endianness/validity/aliasing/lifetime are controlled. Variable TLV/length-prefixed structures still need parser/state-machine layers.

## Async

Trace awaits, blocking in async contexts, ownership across suspension, lock scope, cancellation safety, queue bounds/backpressure and task lifetime.

## Evidence

Baseline:

```text
cargo check
cargo clippy
cargo test
```

Conditional:

```text
property tests
cargo fuzz
cargo miri test
Kani
benchmarks/profiling
```
