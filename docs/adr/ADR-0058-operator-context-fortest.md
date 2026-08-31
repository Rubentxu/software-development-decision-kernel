# ADR-0058 — OperatorContext::for_test constructor (cycle-24)

**Status:** accepted
**Date:** 2026-08-24
**Cycle:** 24 (A-lite)
**Trigger:** cycle-20 debt-verify ARCH-OPERATORCONTEXT-DEDUP (P2 medium)

---

## Context

cycle-20 debt-verify found 28+ duplicated `OperatorContext { ... }` construction sites
across 4 files in sddk-engine. The duplication is mechanical: same defaults
(ScratchGraphStore + Clock + NoopTaskExecutor + pending_sender: None)
with only the runtime-critical fields (node_run, ir, run) varying per call.

## Decision

Add `OperatorContext::for_test(node_run, ir, run)` constructor with sensible defaults.

### Implementation

```rust
impl OperatorContext<GraphStoreBox> {
    pub fn for_test(
        node_run: Arc<Mutex<NodeRun>>,
        ir: Arc<WorkflowIR>,
        run: Arc<WorkflowRun>,
    ) -> Self {
        Self {
            node_run,
            ir,
            run,
            store: Arc::new(Mutex::new(GraphStoreBox {
                inner: Box::new(ScratchGraphStore),
            })),
            clock: Clock::default(),
            executor: Arc::new(sddk_domain::NoopTaskExecutor),
            pending_sender: None,
        }
    }
}
```

### Store choice: ScratchGraphStore

The original proposal referenced a `MockGraphStore` type that does not exist.
The actual canonical no-op store is `ScratchGraphStore` (already `pub` at
`crates/sddk-engine/src/operator.rs:161`), used identically by:
- Runtime spawned context (`workflow_runtime.rs:601`)
- Child context (`operator.rs:711, :796`)
- All test sites

This makes `ScratchGraphStore` the canonical `GraphStore` for non-persistent
contexts. 4 per-file `MockStore` test structs + 13 inline test `MockStore` structs
become dead code and are removed.

## Consequences

### Positive
- 7/30 sites deduplicated (~-300 LOC)
- Single source of truth for test context defaults
- Future operator additions get the helper for free
- `for_test` name signals "test-only" — discouraged in production

### Negative
- Dead MockStore code removal requires touching multiple files
- Existing tests that defined their own MockStore need verification

### Trade-offs accepted
- `for_test` is `pub` (visibility needed for cross-module use in tests)
- Doc comment explicitly warns against production use

## INV Preservation

- INV-1..INV-12 unchanged (pure refactor)
- INV-10 Arc<Mutex<NodeRun>> field type unchanged (ADR-0054)
- INV-2 dyn GraphStore preserved (Box<dyn GraphStore + Send>)
- Child Pure contract preserved (children use store independently)

## References

- cycle-20 debt-report.md §ARCH-OPERATORCONTEXT-DEDUP
- ADR-0054 (INV-10 Arc<Mutex<NodeRun>>)
- ADR-0057 (cycle-23 tick extraction — internal user of OperatorContext)
- cycle-23 HANDOFF
