# ADR-0022 — `sddk-testkit`: In-Memory Test Fakes for Core Ports

**Status:** accepted (2026-08-29 — sddk-testkit crate implemented and in use)
**Date:** 2026-08-19
**Trigger:** Roadmap SDDK 2.0 Phase 1 (reduce coupling, explicit composition root)
**Reconciliation:** Bats/shellspec mentions superseded by ADR-0069 §Bats reassessment (2026-08-28). Testkit implementation completed and integrated.

---

## Context

Phase 1 MUST item: *"Move cross-cutting test setup into `sddk-testkit` builders"*.

Currently, every crate that tests `Ledger`-dependent code either:
(a) Uses `Storage::open_in_memory()` from `sddk-storage` directly, coupling to SQLite
(b) Duplicates test data builders across test files
(c) Both

Example problems found in audit (2026-08-19):

```rust
// sddk-engine/src/adoption.rs:751
&mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap()

// crates/sddk-storage/tests/chain_verification.rs
// Duplicated helper functions (genesis_chain_hash, chain_hash) across test files
```

This coupling makes it impossible to test `sddk-engine` without compiling `sddk-storage`
and all its transitive dependencies.

---

## Decision

Create a new `sddk-testkit` crate that provides:

### 1. In-Memory Fake Implementations

```rust
// sddk-testkit/src/ledger.rs
pub struct InMemoryLedger {
    cycles: BTreeMap<String, CycleRecord>,
    events: Vec<LedgerEvent>,
    sequence: AtomicU64,
}

impl InMemoryLedger {
    pub fn new() -> Self { ... }
    pub fn with_cycle(mut self, cycle: CycleRecord) -> Self { ... }
}

impl Default for InMemoryLedger {
    fn default() -> Self { Self::new() }
}

impl Ledger for InMemoryLedger {
    fn get_cycle(&self, cycle_id: &str) -> Result<CycleRecord, StorageError> { ... }
    fn list_cycle_events(&self, cycle_id: &str) -> Result<Vec<LedgerEvent>, StorageError> { ... }
    fn insert_cycle_with_event(&self, ...) -> Result<LedgerEvent, StorageError> { ... }
    // ... all Ledger methods
}
```

### 2. Event Builder Helpers

```rust
// sddk-testkit/src/builders.rs
pub struct EventBuilder {
    event_id: String,
    event_type: String,
    cycle_id: Option<String>,
    // ...
}

impl EventBuilder {
    pub fn new(event_type: &str) -> Self { ... }
    pub fn with_cycle(mut self, cycle_id: &str) -> Self { ... }
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self { ... }
    pub fn build(self) -> LedgerEvent { ... }
}

pub struct CycleBuilder { ... }
```

### 3. Test Data Fixtures

```rust
// sddk-testkit/src/fixtures.rs
pub fn empty_ledger() -> InMemoryLedger { ... }
pub fn single_cycle_ledger(cycle_id: &str) -> (InMemoryLedger, CycleRecord) { ... }
pub fn two_event_stream() -> (InMemoryLedger, Vec<LedgerEvent>) { ... }
```

### Crate Layout

```
crates/sddk-testkit/
├── Cargo.toml          # depends only on sddk-domain
├── src/
│   ├── lib.rs          # re-exports
│   ├── ledger.rs       # InMemoryLedger
│   ├── event_store.rs  # InMemoryEventStore (if needed)
│   ├── builders.rs     # EventBuilder, CycleBuilder
│   └── fixtures.rs     # canned test data
└── tests/
    └── integration.rs  # verifies fakes match real implementations
```

### Dependency Rule

`[rules]`
- `sddk-testkit` may ONLY depend on `sddk-domain`
- `sddk-engine`, `sddk-cli` may depend on `sddk-testkit` for tests
- `sddk-storage` may NOT depend on `sddk-testkit` (to avoid circular dep)
- Production code may NOT use `sddk-testkit`

---

## Consequences

- **Positive:** `cargo test -p sddk-engine --no-default-features` can run without `sddk-storage`.
- **Positive:** Eliminates duplicated `genesis_chain_hash` helper across test files.
- **Positive:** Test failures are easier to isolate (is it the fake or the real impl?).
- **Negative:** New crate means another artifact to maintain in `cargo test --workspace`.
- **Negative:** Fakes must be kept in sync when `Ledger` trait changes (add a test that
  the fake implements the same trait as the real `Storage`).
- **Neutral:** Migration is incremental — existing tests can stay as-is; new tests use fakes.

---

## Implementation Plan (P1-TK-001)

| Step | Description | File | Issue |
|------|-------------|------|-------|
| 1 | Create `crates/sddk-testkit/Cargo.toml` | `Cargo.toml` | P1-TK-001 |
| 2 | Add `InMemoryLedger` implementing `Ledger` trait | `src/ledger.rs` | P1-TK-002 |
| 3 | Add `EventBuilder`, `CycleBuilder` | `src/builders.rs` | P1-TK-003 |
| 4 | Add `fixtures.rs` with canned test data | `src/fixtures.rs` | P1-TK-004 |
| 5 | Move `adoption.rs` tests to use `InMemoryLedger` | `adoption.rs` | P1-TK-005 |
| 6 | Add `cargo check -p sddk-testkit` to CI | `justfile` | P1-TK-006 |
| 7 | Add test verifying fake matches real behavior | `tests/integration.rs` | P1-TK-007 |

---

## Exit Criteria

- [ ] `cargo tree -p sddk-testkit -e normal` shows only `sddk-domain`
- [ ] `sddk-engine` adoption tests pass with `InMemoryLedger` (no `Storage::open`)
- [ ] `cargo test -p sddk-testkit` green
- [ ] No duplicated helper functions across test files for `chain_hash` computation

---

## References

- Phase 1 SHOULD: *"Move cross-cutting test setup into `sddk-testkit` builders"*
- `Storage::open_in_memory()` — existing in-memory factory in `sddk-storage`
- `chain_verification.rs` — duplicated helpers candidate for extraction
