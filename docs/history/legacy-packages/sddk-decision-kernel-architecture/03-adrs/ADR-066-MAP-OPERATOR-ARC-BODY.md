# ADR-066 — Map Operator: Arc<dyn Operator> Source/Body + build_operator Resolution

**Status:** Accepted

## Context

The Map operator (DC-MAP-002) required a global refactor of how domain operators are dispatched and how Map resolves its `source` and `body` `OperatorId` references.

### Problem Statement

The legacy `dispatch(ir_op: &DomainOperator)` function:
1. Could not resolve `OperatorId` references (no access to `WorkflowIR`)
2. For `Map`, this meant it returned a degenerate `Map { source: OperatorId, body: OperatorId }` with no resolved operators
3. `Map::evaluate` had to call `ctx.ir.operators.get()` at evaluation time — violating DC-MAP-001 isolation

### Design Decision (DC-MAP-002 via Option D)

Replace `dispatch()` with a new `build_operator(ir_op: &DomainOperator, ir: &WorkflowIR) -> Result<Arc<dyn Operator>, OperatorError>` that:
1. Takes `&WorkflowIR` as context for `OperatorId` resolution
2. Recursively resolves children before storing on runtime types
3. Validates Map's body is a `Task` at construction time
4. Returns `NotImplementedInCycle16` for 7 out-of-scope variants

## Decision

### New Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  build_operator(&DomainOperator, &WorkflowIR)               │
├─────────────────────────────────────────────────────────────┤
│  Task      → Arc::new(Task::new(capability, inputs))        │
│  Sequence  → Sequence::new(children: Vec<Arc<dyn Operator>>│
│  Parallel  → Parallel::new(children, max_concurrency)       │
│  Choice    → Choice::new(branches, default)                 │
│  Map       → Map::new(...) [resolves source + body]         │
│  7 variants → Err(NotImplementedInCycle16)                  │
└─────────────────────────────────────────────────────────────┘
```

### Map Type Change

**Before:**
```rust
pub struct Map {
    pub source: OperatorId,
    pub body: OperatorId,
    pub max_concurrency: usize,
}
```

**After:**
```rust
pub struct Map {
    pub source: Arc<dyn Operator>,   // pre-resolved at construction
    pub body: Arc<Task>,            // pre-resolved, validated Task at construction
    pub max_concurrency: usize,
}
```

### Map::new Signature

```rust
impl Map {
    pub fn new(ir_op: &DomainOperator, ir: &WorkflowIR) -> Result<Self, OperatorError>
}
```

- Resolves `source` and `body` `OperatorId` via `build_operator` recursively
- Downcasts `Arc<dyn Operator>` → `Arc<Task>` for body (body must be Task — IR-level invariant)
- Returns `Err(EvalFailed("map body must be Task"))` if body is not Task

### Map::evaluate Isolation

`Map::evaluate` no longer calls `ctx.ir.operators.get()`. Source and body are already resolved at construction time.

```rust
fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
    // (a) source is pre-resolved Arc<dyn Operator> — no ctx.ir.operators.get()
    let source_outcome = self.source.evaluate(&mut source_ctx)?;
    // (b) body is pre-resolved Arc<Task>
    let body_task = &*self.body;
    // ...
}
```

## Consequences

### Positive
- DC-MAP-001 isolation: source evaluation does not mutate parent node_run
- DC-MAP-002 closure: Map body resolution happens once at construction
- Map construction fails fast if body is not Task
- Single canonical constructor (`build_operator`) for all runtime operators

### Negative
- `dispatch()` removed — all call sites updated
- `workflow_runtime.rs` now passes `&self.ir` to `build_operator`
- `Map` no longer stores raw `OperatorId` — some debugging scenarios changed

## References

- DC-MAP-001: Map source evaluation isolated from parent context
- DC-MAP-002: Map body resolution closure via Option D
- cycle-31 RED commit: `e183407`
- cycle-31 GREEN commit: `c30f051`
- cycle-31 dispatch-remove commit: `52dd2e3`
