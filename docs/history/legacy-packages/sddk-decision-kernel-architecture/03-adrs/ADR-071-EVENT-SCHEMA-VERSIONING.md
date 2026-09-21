# ADR-071 — Event Schema Versioning Rule

**Status:** Accepted

## Context

`EventEnvelopeV1.SCHEMA_VERSION = 1` (event_envelope.rs:136) is the only version constant. No documented rule exists for: (a) what constitutes a breaking change requiring a version bump, (b) how multi-version registration works, (c) what happens to events of mixed versions in the same stream. ADR-068 guarantees round-trip and hash stability but does not formalize the bump policy. ADR-069 §5 defines the ActorRef 5-field contract. ADR-070 ships engine-side authority enforcement.

Three INC debts are resolved by EVT-LEDGER-001 (this cycle): INC-HX-AUTH-003 (provenance loss in 4 carriers), INC-HX-AUTH-004 paths 5–6 (gate receipt creation and knowledge ingest authority), and the causal/correlation gap (SPEC-028 deferral).

## Decision

### §1 Schema Version Policy

`EventEnvelopeV1` and all registered event schemas use a single integer `schema_version: u32`.

**Bump required when:**

1. A field is **removed** from the envelope or any registered event schema.
2. A field is **renamed** (same semantic content, different name).
3. A field is **retyped** (same name, different type).
4. A structural invariant enforced by `compute_content_hash()` changes.
5. A required field becomes optional, or an optional field becomes required, changing the minimum valid shape.

**Bump NOT required when:**

6. A new **optional** field is added with `#[serde(default)]` and `skip_serializing_if = "Option::is_none"`.
7. A field's internal validation rules are tightened (e.g., regex变得更严格) without changing the wire format.
8. Documentation, comments, or non-functional metadata are updated.

### §2 Multi-Version Registration

Each event type may have multiple schema versions registered simultaneously in `EventSchemaRegistry`:

```rust
// registry.rs
HashMap<(event_type: String, schema_version: u32), Arc<dyn EventSchema>>
```

- `register(schema)` inserts at `(event_type, schema.schema_version())`.
- A v2 schema does NOT replace v1; both coexist. Consumers must select the version they understand.
- `CanonicalEventValidator` accepts any `schema_version` in `SUPPORTED_SCHEMA_VERSIONS` and rejects others with `EventValidatorError::UnsupportedSchemaVersion { got, want }`.
- `EventEnvelopeV1::SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[1]` — extend this constant when v2 is introduced.

### §3 Migration Helper

When a bump is required, a `migrate_v1_to_v2(payload: &mut Value)` helper must be provided that transforms a v1 payload into a v2 payload while:

- Preserving all v1 fields that remain valid in v2.
- Adding new optional fields with their default values.
- The `compute_content_hash()` of a migrated v1 event (after migration) must be **identical** to the original v1 hash when evaluated under the v1 schema (backward compatibility).

**No `migrate_v1_to_v2` helper is required when the change is purely additive** (case 6 above) — existing v1 events remain valid without transformation.

### §4 Schema Version in Envelope

`EventEnvelopeV1.schema_version: u32` is the single source of truth for the wire-format version. The schema registry maps `(event_type, schema_version)` → schema. The validator rejects `schema_version ∉ SUPPORTED_SCHEMA_VERSIONS`.

`EventEnvelopeV1::SCHEMA_VERSION: u32 = 1` is locked at 1 for the base envelope. Per-type schema versions (e.g., `WorkflowPhaseEnteredSchema`) are tracked independently in the registry.

### §5 ActorRef Carrier Rule

Widening a carrier from `actor: String` to `actor_ref: ActorRef` (additive, case 6) does **not** require a schema version bump. The legacy `actor: String` field remains readable for replay compatibility. New writers populate `actor_ref`; readers fall back to `actor` when `actor_ref` is absent.

Evidence: ADR-069 §5 (`ActorRef` 5-field contract); `event_envelope.rs:43-57`; `crates/sddk-domain/src/models/ledger.rs`; INC-HX-AUTH-003.

### §6 ActorKind Closed Set

`ActorKind` enum has exactly three variants: `Human`, `Agent`, `System` per ADR-069 §2. No fourth variant (including `Secretary`) may be introduced without an ADR. `Secretary` remains `Agent { role: "secretary", model, definition_hash }` per ADR-0073-AMENDMENT-1.

### §7 Enforcement

The `CanonicalEventValidator` (validator.rs) enforces Stage 1: `schema_version ∈ SUPPORTED_SCHEMA_VERSIONS`. Stage 2 validates payload shape against the registered schema for that `(event_type, schema_version)` pair.

## AC Coverage

| AC ID | Description |
|--------|-------------|
| AC-EVT-LEDGER-01 | Schema versioning rule formalized; breaking vs additive distinction defined |
| AC-EVT-LEDGER-02 | Four-carrier ActorRef widening is additive per this rule |
| AC-EVT-LEDGER-03 | Causation/correlation fields added as optional (additive per this rule) |
| AC-EVT-LEDGER-04 | Replay harness + Snapshot do not introduce new event types |
| AC-EVT-LEDGER-05 | Replay equality invariant preserves content_hash |
| AC-EVT-LEDGER-06 | Cross-ledger consistency is additive |
| AC-EVT-LEDGER-07 | INC-HX-AUTH-003 resolved; INC-HX-AUTH-004 paths 5–6 closed |
| AC-EVT-LEDGER-08 | Gate receipt creation wired with AuthorityContext |
| AC-EVT-LEDGER-09 | Knowledge ingest wired with AuthorityContext |
| AC-EVT-LEDGER-10 | ApprovalState provenance fix uses canonical actor_ref.id |
| AC-EVT-LEDGER-11 | DW-IR-005 determinism tests preserved |
| AC-EVT-LEDGER-12 | No carryover regression |

## Consequences

**Gain:**
- Schema version bump rule is now formally documented and enforceable.
- Multi-version registry support enables forward-compatible event type evolution.
- Migration helper contract ensures backward replay compatibility.
- Clear distinction between additive (no-bump) and breaking (bump required) changes.
- INC-HX-AUTH-003 (4-carrier provenance) closed with additive widening.

**Risks:**
- The `serde_json/preserve_order` lint guards against HashMap non-determinism but is a build-time check, not compile-time.
- Registry test `registry_len_matches_expected_count` (registry.rs:142) must be updated when new event types are registered.

## References

- [ADR-068](docs/sddk-decision-kernel-architecture/03-adrs/ADR-068-DETERMINISTIC-IR-RUNTIME-BOUNDARY.md) — deterministic IR + hash stability invariants
- [ADR-069](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md) — ActorRef 5-field contract; ActorKind closed set
- [ADR-070](docs/sddk-decision-kernel-architecture/03-adrs/ADR-070-ENGINE-AUTHORITY-ENFORCEMENT.md) — engine authority enforcement; WritableSurface matrix
- [event_envelope.rs:136](crates/sddk-domain/src/event_envelope.rs:136) — `SCHEMA_VERSION = 1`
- [debt-report.schema.json:29](docs/debt/debt-report.schema.json:29) — `status: resolved` enum value
- INC-HX-AUTH-003 — provenance loss in 4 carriers
- INC-HX-AUTH-004 — no-parallel-authority invariant
