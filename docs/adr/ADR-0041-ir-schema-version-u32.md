# ADR-0041 — Schema Version as `u32` Constant Per IR Type

**Status:** accepted
**Date:** 2026-08-19
**Trigger:** kernel-workflow-ir-contracts (v1.29.0) — monotonic schema evolution

---

## Context

`WorkflowIR`, `ExecutionGraphRevision`, and related types need a schema version identifier to support forward migration. The question is whether to use `semver::Version` or a simple `u32` constant.

Using semver feels natural ("version") but introduces complexity: a full semver parser, potential for non-monotonic version strings (e.g., `2.0.0` < `1.10.0` lexicographically), and dependency on a third-party crate.

---

## Decision

Each IR type carries an explicit `pub const SCHEMA_VERSION: u32 = N` constant. Schema migration is monotonic: each migration step increments the integer. The `architecture-rules.yaml` schema version stays as a `String` (`"1.1.0"`) since that follows YAML conventions and is only used by the rules evaluator.

```rust
// WorkflowIR
pub const SCHEMA_VERSION: u32 = 1;

// ExecutionGraphRevision
pub const SCHEMA_VERSION: u32 = 1;
```

Migration logic in `migrations.rs` (cycle 2) will use integer comparison for monotonically ordered upgrades.

---

## Consequences

- **Positive:** Integer comparison is fast and unambiguous — no parsing needed
- **Positive:** Matches `EventEnvelopeV1::SCHEMA_VERSION` (already `u32`)
- **Positive:** Zero dependency on semver crate
- **Negative:** Schema evolution policy must be documented externally (humans track meaning of each integer)
- **Negative:** Non-obvious mapping from integer to human-readable change description

---

## References

- `SCHEMA_VERSION` constant in `crates/sddk-domain/src/workflow_ir.rs`
- `SCHEMA_VERSION` constant in `crates/sddk-domain/src/graph.rs`
- `architecture-rules.yaml` schema version remains `String "1.1.0"` (YAML convention)
