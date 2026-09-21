# ADR-DW-IR-004 — Typed Operator Input/Output/Error Contracts

**Status:** Accepted

## Context

The DW-IR-003 sibling module (`plan_revision.rs`) establishes the lineage anchor for `NormalizedPlanV1` semantic equivalence. DW-IR-004 extends that foundation by adding typed I/O contracts to every `Operator` variant in `crates/sddk-domain/src/workflow_ir.rs:360-443`.

Two canonical gaps motivated this decision:

- **C-1** (`SPEC-037:21`): `WorkflowIR` output mentions "schemas" without enumerating them. The absence meant operators carried no typed input/output boundary — inputs were bare `BTreeMap<String, serde_json::Value>` and Map's output was the v1.29.0-era placeholder `outputs["items"]: serde_json::Value::Array`.
- **C-2** (STATUS.md line 28 / EVOLUTION-CROSSWALK.md line 40): "operator real output semantics PARTIAL → H0 contracts + H6 DW-OPERATORS-001" — the H0/H6 boundary was implicit and needed explicit enumeration.

This ADR commits to the H0 scope: typed input contracts, typed output contracts, a closed-set error contract, JSON Schema as the schema language, a version stamp, and lineage integration via `NormalizedPlanV1.operator_contracts`.

## Decision

### 1. Schema language — `SchemaDialect` closed set

The schema language for all operator I/O contracts is **JSON Schema draft-07** (`JsonSchemaDraft07`). The closed set of `SchemaDialect` is:

```
JsonSchemaDraft07
```

Adding any dialect (e.g. `JsonSchemaDraft202012`) requires a future ADR that bumps `OPERATOR_CONTRACT_SCHEMA_VERSION`. No other dialect is valid under this ADR.

### 2. Version stamp — `OPERATOR_CONTRACT_SCHEMA_VERSION`

`pub const OPERATOR_CONTRACT_SCHEMA_VERSION: u32 = 1`

Bumping the constant is an ADR-level action. The constant participates in `NormalizedPlanV1.plan_identity` via the `OperatorContractProjectionV1` field (REQ-OPVER-001).

### 3. `OperatorContractError` — closed-set error contract

`OperatorContractError` lives in `crates/sddk-domain/src/operator_contract.rs` (domain module). It is **distinct** from `OperatorError` in `crates/sddk-engine/src/operator.rs:472-485` (runtime engine error). The H0/H6 boundary enforces that `OperatorError` is untouched.

The enum has **exactly 8 variants** with `#[serde(rename_all = "snake_case")]`:

```
UnsupportedSchemaVersion { got: u32, want: u32 }
UnknownOperatorVariant { variant: &'static str }
InputContractViolation { operator_id: OperatorId, variant: &'static str, field: String, reason: &'static str }
OutputContractViolation { operator_id: OperatorId, variant: &'static str, field: String, reason: &'static str }
MissingRequiredField { operator_id: OperatorId, variant: &'static str, field: String }
ExtraFieldDisallowed { operator_id: OperatorId, variant: &'static str, field: String }
SchemaSourceMismatch { operator_id: OperatorId, expected: ContentHash, actual: ContentHash }
SchemaDialectUnknown { dialect: String }
```

A compile-time guard `crate::assert_variant_count_eq!(OperatorContractError, 8, [...])` enforces the variant count. Adding or removing a variant requires an ADR that updates both the guard and this decision.

### 4. Lineage integration — `operator_contracts: BTreeMap<OperatorId, OperatorContractProjectionV1>`

`NormalizedPlanV1` gains a new field:

```rust
pub operator_contracts: BTreeMap<OperatorId, OperatorContractProjectionV1>
```

- **Semantic fields** (participate in `plan_identity`): `version`, `dialect`, `source`, `static_fields` (keys and per-key `source`), `required_fields`, `accepts_extra_fields`.
- **Non-semantic fields** (excluded from `plan_identity`): `description` (on both `OperatorInputSchema` and `OperatorOutputSchema`). These are omitted from the projection before canonical serialization, mirroring how `ir_id`/`prompt_hash`/`model_hash` are excluded in `compute_content_hash`.

The `OperatorContractProjectionV1` projection type is `PartialEq + Eq + Serialize + Deserialize` with deterministic maps/sets.

Legacy IRs (without typed contracts) default `operator_contracts` to an empty map via `#[serde(default)]` — they remain lineage-stable.

## Consequences

**Gain:**
- Typed I/O contracts per operator variant with default schemas matching the table in SPEC section 6.
- Closure of the H0/H6 boundary: S-1..S-6 (typed contracts) are H0; S-7 (`OperatorError`) is untouched; S-8..S-12 (durable Map fan-out, Reduce, JoinAny/JoinAll, durable child output model) are H6.
- The exit gate "Runtime operators no longer rely on undefined placeholder output semantics at the contract boundary" is provably satisfied.
- Two IRs with identical semantic content but differing only in `description` produce identical `plan_identity`.
- Two IRs where one has `operator_contracts` populated and the other has it empty produce different `plan_identity`.

**Risk:**
- Engine pre/post-evaluation adds latency on every operator evaluation. This is acceptable for H0 (contracts are simple); H6 optimizations (caching, lazy validation) are deferred.

## Alternatives considered

**(a) Leave inputs as `serde_json::Value` and document the risk.** Rejected — does not close the exit_gate; the "undefined placeholder semantics" concern applies equally to inputs and outputs.

**(b) Adopt `JsonSchemaDraft202012` immediately.** Rejected — not justified by current evidence. Draft-07 is stable, widely implemented, and sufficient for the H0 contract surface.

**(c) Skip the closed-set error contract and use `Box<dyn Error>` or stringly errors.** Rejected — violates DW-IR-003 equivalence discipline. The error type must be deterministically serializable so that equivalent IRs produce identical error signatures.

## Cross-references

- [DW-IR-001](.) — execution scope
- [DW-IR-002](.) — transition AST
- [DW-IR-003](.) — plan revision lineage (sibling module, dependency)
- SPEC-037 — mentioned "schemas" without enumeration (C-1)
- ADR-024 — 12-variant operator algebra
- ADR-037 — Dynamic Workflow Compilation
- ADR-041 — Workflow Runtime v2 three-level model
- ADR-043 — Change-Scoped Verification Service
- [SPEC-043](../04-specs/SPEC-043-CHANGE-SCOPED-VERIFICATION-SERVICE.md) — change-scoped verification service contract
