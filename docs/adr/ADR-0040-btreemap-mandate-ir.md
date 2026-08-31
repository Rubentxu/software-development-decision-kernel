# ADR-0040 — BTreeMap Mandate for IR Collections

**Status:** accepted
**Date:** 2026-08-19
**Trigger:** kernel-workflow-ir-contracts (v1.29.0) — determinism requirement for content-addressed IR

---

## Context

`WorkflowIR` and `ExecutionGraphRevision` must be content-addressable: the same logical IR must produce the identical `compute_content_hash()` output across process restarts, clones, and serialization roundtrips.

HashMap iteration order is non-deterministic in Rust (hash seed, bucket ordering). Using `HashMap` anywhere in an IR type would make `compute_content_hash()` produce different values across runs — violating the core invariant of the content-addressing scheme.

---

## Decision

All collection fields in `WorkflowIR`, `WorkflowTemplate`, `ExecutionGraphRevision`, and all operator types use `BTreeMap<K, V>` and `BTreeSet<T>` exclusively. `HashMap` is **forbidden** in these modules.

Enforcement:
- A `compile_fail` doc-test in `workflow_ir.rs` prevents accidental `HashMap` import
- `serde_json::Map` defaults to `BTreeMap` (feature flag `preserve_order` is not enabled)
- JSON serialization uses `serde_json::to_vec` which respects `BTreeMap` key ordering

---

## Consequences

- **Positive:** `compute_content_hash()` is deterministic — same IR always produces the same hash
- **Positive:** JSON wire format is ordered and human-readable for debugging
- **Positive:** Mirrors the existing `proposal.rs::hash_structural` pattern
- **Negative:** `BTreeMap` insertion is O(log n) vs O(1) for `HashMap` — negligible for IR sizes in v1.29.0
- **Negative:** Developers unfamiliar with `BTreeMap` may accidentally use `HashMap` — mitigated by `compile_fail` guard

---

## References

- `WorkflowIR::compute_content_hash()` in `crates/sddk-domain/src/workflow_ir.rs`
- `ExecutionGraphRevision::compute_digest()` in `crates/sddk-domain/src/graph.rs`
- ADR-025 (BTreeMap as universal IR collection — same decision, prior cycle)
