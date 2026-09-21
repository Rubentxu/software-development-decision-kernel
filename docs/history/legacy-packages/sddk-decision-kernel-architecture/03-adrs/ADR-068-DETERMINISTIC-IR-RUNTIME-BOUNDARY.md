# ADR-DW-IR-005 — Deterministic IR and Compiler-Boundary Invariants

**Status:** Accepted

## Context

DW-IR-004 shipped typed operator I/O contracts (`OperatorContractProjectionV1` in `NormalizedPlanV1.operator_contracts`) and closed the 8-variant `OperatorContractError` set. DW-IR-005 builds on that foundation to prove the four invariants named by the spine's exit gate: **round-trip**, **hash stability**, **invalid-plan rejection**, and **deterministic compilation**.

Two conflicts motivated this work:

- **C-1** (resolved): EXECUTION-SPINE.yaml:144-150 declares a four-clause exit gate but no SPEC/ADR enumerated testable REQs for these invariants. The spec phase drafted 19 REQs across 22 scenarios.
- **C-2** (resolved): "compiler-boundary invariants" was ambiguous — there is no `CompiledPlanV1` type. The IR→runtime boundary is the inline `build_operator()` function in `crates/sddk-engine/src/operator.rs:1726`.

## Decision

### 1. Round-trip invariants (byte-exact + identity-exact)

The following types MUST round-trip byte-exactly via `serde_json`:

| Type | Key invariant | Test |
|------|--------------|------|
| `WorkflowIR` | `compute_content_hash()` unchanged | `workflow_ir_proptest.rs:289-295` (existing) |
| `NormalizedPlanV1` | `plan_identity()` stable + `operator_contracts` projection match | `plan_revision.rs:test_stability_roundtrip_serde_with_operator_contracts` (new) |
| `PlanRevisionLineageV1` | `revision_id` chain intact | `plan_revision.rs:617-655` (existing) |
| `OperatorContractProjectionV1` | byte equality + per-operator projection match | `operator_contract.rs:1316-1329` (existing) |
| `EventEnvelopeV1` | `compute_content_hash()` unchanged | `event_envelope.rs:roundtrip_preserves_content_hash` (new) |
| `LedgerEventInput` | JSON payloads (`payload`, `state_before`, `state_after`) byte-exact | `models/ledger.rs:tests::ledger_event_input_roundtrip_preserves_payload` (new) |

### 2. Hash stability invariants

| Invariant | Requirement | Test |
|-----------|-------------|------|
| REQ-IRDT-HS-01 | `compute_content_hash()` / `plan_identity()` independent of BTreeMap insertion order | `workflow_ir_proptest.rs:184-242` (existing) |
| REQ-IRDT-HS-03 | `serde_json` built WITHOUT `preserve_order` / `indexmap` | `serde_json_feature_guard.rs:serde_json_preserve_order_disabled` (new) |
| REQ-IRDT-HS-04 | Two IRs with identical `compute_content_hash()` produce observationally equivalent runtime trees | `build_operator_ir_permutation_equivalence.rs:build_operator_equivalent_ir_permutation_equivalence` (new) |
| REQ-IRDT-HS-05 | No `HashMap` field in IR canonical form structs | `hashmap_audit.rs:no_hashmap_in_ir_canonical_forms` (new) |

### 3. Invalid-plan rejection invariants

All malformed inputs MUST return structured `Result::Err` with ZERO panics (verified via `std::panic::catch_unwind`):

| Scenario | Error type | Test |
|----------|------------|------|
| Malformed JSON | `serde_json::Error` | `invalid_plan_rejection.rs:malformed_json_rejected_no_panic` |
| Missing required field | `OperatorContractError::MissingRequiredField` | `invalid_plan_rejection.rs:missing_required_field_rejected` |
| Extra field in strict schema | `OperatorContractError::ExtraFieldDisallowed` | `invalid_plan_rejection.rs:extra_field_strict_schema_rejected` |
| Dangling OperatorId in Sequence | `OperatorError::EvalFailed(msg)` with operator ID | `build_operator_tests.rs:build_operator_eval_failed_for_missing_operator_id` (existing) |
| Empty lineage | `PlanRevisionError::EmptyLineage` | `invalid_plan_rejection.rs:empty_lineage_rejected` |
| `SchemaDialectUnknown` serialization-safe | Byte-exact round-trip | `operator_contract_error_tests.rs:schema_dialect_unknown_serializes_to_valid_json` (new) |

### 4. Deterministic compilation invariants

| Invariant | Requirement | Test |
|-----------|-------------|------|
| REQ-IRDT-DC-01 | `WorkflowCompiler::compile` deterministic across 1000 random inputs | `compiler_determinism.rs:127-146` (existing, proptest) |
| REQ-IRDT-DC-02 | Same as REQ-IRDT-HS-04 (equivalent IR → equivalent runtime) | `build_operator_ir_permutation_equivalence.rs` (new) |
| REQ-IRDT-DC-03 | `Sequence.children` order matches IR declaration order | `build_operator_ir_permutation_equivalence.rs:sequence_declaration_order_preserved` (new) |
| REQ-IRDT-DC-04 | `Choice.branches` iteration is BTreeMap sorted-key order | `build_operator_ir_permutation_equivalence.rs:choice_branches_sorted` (new) |

### 5. Conflict C-1 resolution

The exit gate wording "compiler-boundary invariants" is interpreted as two sub-invariants:
- **(a)** `WorkflowCompiler::compile(manifest) -> WorkflowIR` determinism (already HELD via proptest)
- **(b)** `build_operator(ir_op, ir) -> Arc<dyn Operator>` determinism (new coverage in this cycle)

No new `CompiledPlanV1` type is introduced.

### 6. Conflict C-2 resolution

"Compiled plan" = `WorkflowIR` itself (which has `compute_content_hash()`). The `build_operator()` function is the IR→runtime constructor and is the subject of the new determinism tests.

## Consequences

**Gain:**
- All four exit-gate invariants are proven with tests
- IR canonical forms are protected against `HashMap` / `IndexMap` regressions
- Invalid-plan paths are proven panic-free
- `operator_contracts` field (DW-IR-004 deliverable) is explicitly round-trip tested at the `NormalizedPlanV1` level
- `LedgerEventInput` round-trip closes the storage-layer hash gap

**Carryovers:**
- FIND-000001 (Map fake-variant in `validate_output`): deferred to H6/DW-OPERATORS-001
- FIND-000002 (`OperatorContractError` → `OperatorError::EvalFailed` bridge collapses structured errors): deferred to H6/DW-OPERATORS-001
- FIND-000005 (`SchemaDialectUnknown` forward-only variant): documented, negative-case test added, not a blocker

**Risk:**
- The `serde_json/preserve_order` lint is a build-time check, not a compile-time guard. A future dependency bump that enables the feature would be caught only at CI lint time.

## Alternatives considered

**(a) Introduce `CompiledPlanV1` as a new type** — Rejected: exceeds A-min scope; `WorkflowIR` already has `compute_content_hash()` and is sufficient.

**(b) Skip invalid-plan integration test** — Rejected: the exit gate explicitly requires "invalid-plan... tests pass."

**(c) Skip HashMap audit (leverage existing BTreeMap discipline)** — Rejected: the spec phase explicitly added REQ-IRDT-HS-05 as a CI guard.

## Cross-references

- [DW-IR-001](.) — execution scope
- [DW-IR-002](.) — transition AST
- [DW-IR-003](.) — plan revision lineage (dependency)
- [DW-IR-004](.) — typed operator I/O contracts (dependency)
- SPEC-037 — Dynamic Workflow Compiler
- SPEC-039 — Workflow Pattern Algebra
- ADR-024 — Generic Workflow Template, IR and Runtime Algebra
- ADR-037 — Dynamic Workflow Compilation and Evented Graph Expansion
- ADR-043 — Change-Scoped Verification Service
- ADR-067 — Typed Operator I/O (direct predecessor)
