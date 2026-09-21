# HANDOFF — sddk-framework — 2026-08-25 (cycle-31)

> **Cycle:** `kernel-cycle-31-dispatch-global-refactor` (DC-MAP-002)
> **Released as:** v1.10.0
> **HEAD:** `23949bc` (ADR-0066)
> **Tag:** v1.10.0

## Drift carry-over (not resolved in this cycle)

None.

## Last closed cycle

`kernel-cycle-31-dispatch-global-refactor` (v1.10.0) — Map dispatch global refactor.

## Current state (cargo test / clippy)

```
cargo test -p sddk-engine         ✓ green (128 lib + 24 map + 7 build_op + 4 runtime = 163 tests)
cargo test -p sddk-engine --lib   ✓ 128 passed
cargo clippy -p sddk-engine       ✓ 0 errors
cargo fmt --all                   ✓ clean
```

## What changed (4 commits)

1. `feat(cycle-31): RED — build_operator shell + Map source/body Arc<dyn Operator>` — 3 test files added (991 lines), build_operator returns NotImplementedInCycle16
2. `feat(cycle-31): GREEN Map build_operator construction — Arc<dyn Operator> source/body` — full implementation
3. `feat(cycle-31): REMOVE dispatch() — replaced by build_operator` — dispatch removed, all tests updated
4. `docs(adr): ADR-066 Map Arc body + build_operator resolution`

## Key technical changes

### Map now stores Arc types (not OperatorId)

```rust
pub struct Map {
    pub source: Arc<dyn Operator>,   // pre-resolved at construction
    pub body: Arc<Task>,             // pre-resolved, validated Task at construction
    pub max_concurrency: usize,
}
```

### Map::new signature

```rust
pub fn new(ir_op: &DomainOperator, ir: &WorkflowIR) -> Result<Self, OperatorError>
```

### build_operator replaces dispatch()

```rust
pub fn build_operator(ir_op: &DomainOperator, ir: &WorkflowIR) -> Result<Arc<dyn Operator>, OperatorError>
```

## Next cycle (suggested)

`kernel-cycle-32-*` — continue Map implementation or start new cycle.

## Recovery cheat sheet

```bash
# Rollback this cycle
git reset --hard <pre-cycle-SHA> && git tag -d v1.10.0

# Verify dispatch() is gone
grep -r "pub fn dispatch" crates/sddk-engine/src/

# Verify build_operator exists
grep -r "pub fn build_operator" crates/sddk-engine/src/
```
