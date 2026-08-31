# ADR-0051 — `Box<dyn Operator>` → `Arc<dyn Operator>` Runtime Safety

**Status:** accepted
**Date:** 2026-08-23
**Trigger:** SDDK kernel-cycle-19 operator dyn safety

---

## Context

`Box<dyn Operator>` and `Arc<dyn Operator>` have different vtable layouts and dispatch semantics in Rust. The prior operator implementation used `Box<dyn Operator>` for dynamic dispatch. cycle-19 required a change that:

1. Maintains **identical runtime behavior** for `Operator::evaluate(&self, ctx)`
2. Enables `OperatorContext<'static>: Send + 'static` via field Arc-wrapping
3. Does NOT change the `Operator` trait signature

The key invariant (INV-11) is: `Arc<dyn Operator>` and `Box<dyn Operator>` produce **identical** `evaluate` dispatch because both resolve to `&dyn Operator` at the call site.

---

## Decision

### Mechanical Rename (WU-8a)

All `Vec<Box<dyn Operator>>` fields and local variables in `operator.rs` were mechanically renamed to `Vec<Arc<dyn Operator>>`:

```rust
// Before
pub struct Parallel {
    pub children: Vec<Box<dyn Operator>>, // in struct definition
}

// After
pub struct Parallel {
    pub children: Vec<Arc<dyn Operator>>, // mechanical rename
}

// Test construction: Box::new → Arc::new
let child: Arc<dyn Operator> = Arc::new(Task { .. });
```

**29 sites** in `operator.rs` and `operator_trait_tests.rs` were updated.

### INV-11 Lossless Type Change

The `Operator::evaluate(&self, ctx)` trait method takes `&self` (reference to the smart pointer, not the pointer itself). Both `Box<T>` and `Arc<T>` dereference to `&T` whenmethod is called, so the vtable entry resolved is identical.

```rust
// Both Box<dyn Operator> and Arc<dyn Operator> dispatch identically:
impl Operator for Parallel {
    fn evaluate(&self, ctx: &mut OperatorContext<'_>) -> Result<NodeOutcome, OperatorError> {
        //            ^^^ self: &Arc<dyn Operator> or &Box<dyn Operator>
        // Both dereference to &dyn Operator → same vtable
    }
}
```

This is verified by the `arc_dyn_operator_dispatches_correctly` test in `operator_trait_tests.rs`.

### Why Not `Box<dyn Operator + Send + Sync>`?

`Box<dyn Operator>` cannot be made `Send` without additional bounds. Even `Box<dyn Operator + Send + Sync>` is not automatically `Send` because `Box<T>` is only `Send` if `T: 'static + Send + Sync`. The `dyn Operator` trait itself requires `Send + Sync`, so `Box<dyn Operator + Send + Sync>` could theoretically be `Send`, but this changes the trait object representation in subtle ways.

`Arc<dyn Operator>` (where `dyn Operator: Send + Sync`) is provably `Send + Sync` and the natural choice for shared ownership in a concurrent context.

---

## Consequences

### Positive

- `Arc<dyn Operator>` is `Send + Sync` when `dyn Operator: Send + Sync` (which it is, per trait definition)
- Shared ownership enables `Arc::clone(&child)` in spawn closures without moving ownership
- No semantic change to operator dispatch (INV-11 preserved)
- No `Box::leak` required for dynamic dispatch

### Negative

- `Arc` has slightly higher per-clone overhead than `Box` (atomic reference count). For operator construction (rare) this is negligible.
- The mechanical rename touched 29 sites; careful review required to ensure no `Box::leak` patterns remain in operator construction.

---

## References

- cycle-19 WU-8a: mechanical rename
- cycle-19 WU-4: concurrent fan-out uses `Arc::clone(&child)` in spawn closure
- INV-11: Arc/Box dispatch equivalence
- ADR-0050: cycle-18 carryover scope for concurrent Parallel
