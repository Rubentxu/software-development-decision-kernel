# SCOPE-CONTRACT — S4 — Durability (decision: STOP, pre-implementation report)

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s4-durability`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Status:** pre-implementation STOP report — awaiting operator decision.

## §1 Goal (per macro-cycle §S4)

Demonstrate that the static-evidence artefacts (`ObservationSet`,
`CoverageEvaluation`, `VerifyReceipt` entries from S3) survive
crash/reopen without loss or duplication. Maps to `PR-UAT-024`.

## §2 Storage substrate (read-only audit, no modifications)

| Component | Value | Notes |
|---|---|---|
| `LedgerEvent` model | `crates/sddk-domain/src/models/ledger.rs` | Flat struct, `payload: Value` (serde JSON). |
| Table schema | `crates/sddk-storage/src/migrations.rs` | Pre-`SqliteLedger` schema is live. |
| Event-appending API | `sddk_storage::Storage::append_event` (in `lib.rs`) | Takes `LedgerEventInput`. |
| `event_type` | string field, free-form | Not an enum; no closed vocabulary to extend. |
| `payload` | `serde_json::Value` | JSON-serializable substrate; ObservationSet serializes naturally via its existing `Serialize` derive. |

**Key observation**: the substrate already supports persisting
arbitrary JSON under a named `event_type`. Adding a new event type
(e.g. `observation.substrate.appended`) requires **no schema migration
at all** — only new application-level code that writes/reads it. The
first STOP condition ("schema migration incompatible with the live
`SqliteLedger`") **does not fire by construction**.

## §3 STOP conditions (per macro-cycle §S4)

| Condition | This slice |
|---|---|
| Durability requires a schema migration incompatible with the live `SqliteLedger` | **NOT triggered** by inspection — `payload: Value` already exists. |
| `Storage::cycle_exists` / event-log primitives cannot host static-evidence events without semantic loss | **Anticipated to trigger** — see §4. |

## §4 The semantic-loss STOP (anticipated)

### 4.1 What semantic loss looks like

`LedgerEvent.payload` is opaque JSON. To reconstruct an
`ObservationSet` on reopen, the consumer must know the **exact
schema** at the version of the persisted event. Three risks:

1. **Forward-compat breakage**: a future change to
   `SoftwareObservation`'s `Serialize` impl (rename a field, change
   a tag) silently breaks reopen of pre-change records. The
   `payload: Value` carries no version marker.
2. **Cross-version consumer**: a CLI binary reading a 1.169.95
   receipt whose `payload` was written under a future
   `SoftwareObservation` schema will fail to deserialize (or worse,
   succeed with wrong defaults).
3. **Discovery**: there is no machine-readable index of which
   `event_type` values are legal. Downstream consumers (other slices,
   ad-hoc inspection, DebVerify) cannot enumerate the substrate
   types without reading source.

### 4.2 Why this is a STOP, not a nit

`PR-UAT-024` says "survive crash/reopen without loss or duplication".
A `payload: Value` write achieves "no loss" trivially (it's just
JSON). It does **not** achieve "without semantic loss": the JSON
shape is not part of any contract.

Two ways forward; both deserve an operator decision before code is
written:

**Option A — JSON shape contract (light):**
- Pin a `schema_version: u32` next to each `event_type` in
  `LedgerEvent.payload` (or as a sibling field on `LedgerEventInput`).
- Document the schema in a frozen module doc per event type.
- Lower scope; can fit in S4. Carries the same migration risk for
  schema-version bumps, but those are explicit and bounded.

**Option B — typed event class (medium):**
- Add a sibling module to `crates/sddk-engine/src/observation/`
  (or `crates/sddk-domain/src/event_registry/`) that enumerates the
  static-evidence event classes with their typed serde. The
  consumer side parses via the typed deserializer; the JSON shape
  is owned by Rust types.
- Higher scope; the "out of scope for S4" risk is real (it touches
  the event registry which is shared with DebVerify and other
  consumers).

**Option C — defer the durability proof to a downstream cycle.**
- S4 stays `NOT_PROCEED`; the durability story is documented as
  pending; the macro-cycle closeout (S7) records it as a known
  follow-up.
- Lowest cost; honest.

## §5 Decision

**STOP and report.** Per the operator's session rule, schema-evolution
decisions for shared substrate pause the line. S4 does not proceed
without an explicit choice between A / B / C.

## §6 What this slice does NOT do

- Does not modify `LedgerEvent`, `LedgerEventInput`, or migrations.
- Does not add a new event type.
- Does not write any persistence code.
- Does not run any new test.

## §7 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| Stop report (this file) | `tests/cycle-artifacts/.../s4-durability/SCOPE-CONTRACT.md` | ✅ |
| Decision record | `tests/cycle-artifacts/.../s4-durability/RECEIPT.md` | ✅ |

## §8 References

- `tests/cycle-artifacts/.../a6-static-enhanced-readiness/SCOPE-CONTRACT.md` §S4 (macro-cycle plan).
- `crates/sddk-domain/src/models/ledger.rs` (`LedgerEventInput`, `LedgerEvent`).
- `crates/sddk-storage/src/migrations.rs` (live schema).
- `crates/sddk-storage/src/lib.rs` (`Storage::append_event`).
- `crates/sddk-engine/src/observation/types.rs` (`SoftwareObservation`'s `Serialize`).
