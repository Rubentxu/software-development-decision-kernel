# RECEIPT — S4 — Durability (STOP, pre-implementation report)

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s4-durability`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Baseline (released):** `v1.169.95` → `9688ebb` (S1 close)
> **Cycle lead:** orchestrator (this session, auto-run)
> **Status:** **CLOSED — `STOP`**, push pending release flow.

## §1 Scope adherence

| Hard constraint | Status | Evidence |
|---|---|---|
| C1: no fabrication | ✅ | Storage substrate read from source; no inferred or assumed structure. |
| C2: no schema migration attempted | ✅ | Zero storage-layer code touched. |
| C3: STOP condition honored before code is written | ✅ | Report in SCOPE §4 precedes any code. |
| C4: workspace test green | ✅ | Untouched. |

| STOP condition (per macro-cycle §S4) | Triggered? |
|---|---|
| Durability requires a schema migration incompatible with the live `SqliteLedger` | **No** — `payload: Value` already exists; new event type needs no migration. |
| `Storage::cycle_exists` / event-log primitives cannot host static-evidence events without semantic loss | **Yes (anticipated)** — see SCOPE §4. Schema-evolution contract decision required. |

Per the operator's session rule, the second STOP pauses the line
until the schema-evolution contract is decided. S4 closes in `STOP`
state with three concrete options for the operator.

## §2 Evidence

### 2.1 Storage substrate (read-only)

- `LedgerEvent.payload: serde_json::Value` — opaque JSON, no version marker.
- `LedgerEvent.event_type: String` — free-form, no enum.
- `Storage::append_event(input: &LedgerEventInput)` — the write API.
- No `SoftwareObservation` version marker anywhere in the path.
- `SoftwareObservation` already derives `Serialize` (observation/types.rs line 282+) so JSON round-trip works at this exact version.

### 2.2 Implication for PR-UAT-024

A "no loss" durability story is achievable with `payload: Value`
(simple JSON write). A "no semantic loss" durability story is
**not** achievable without one of:

- **Option A** — JSON shape contract: `schema_version: u32` per event type, document frozen per event type.
- **Option B** — typed event class: register the static-evidence
  event in `event_registry` so consumer-side deserialize is type-driven.
- **Option C** — defer S4 to a downstream cycle; record as known follow-up.

### 2.3 Diff vs macro-cycle

None. The macro-cycle STOP conditions are honored; the slice
decision is the operator's.

## §3 Files changed by this slice

| Path | Δ | Role |
|---|---|---|
| `tests/cycle-artifacts/.../slices/s4-durability/SCOPE-CONTRACT.md` | nuevo | STOP report with three options. |
| `tests/cycle-artifacts/.../slices/s4-durability/RECEIPT.md` | nuevo | This file. |

**Zero source files modified. Zero tests added. Zero migrations applied.**

## §4 Honest limits

1. **The audit is read-only and at this slice's depth.** A deeper
   audit would also examine `InMemoryLedger`, `Storage::reopen`,
   and the `SqliteLedger` migration history. The above is sufficient
   to justify the STOP; deeper audit belongs to whichever option
   (A/B/C) is chosen.
2. **`SoftwareObservation`'s `Serialize` derive is assumed stable
   for the duration of one operator-decision cycle.** If a parallel
   PR mutates it, the option-A contract negotiation changes.
3. **The STOP is not a fault report.** It is a documented pause
   pending a schema-evolution contract decision. S4 makes no claim
   about the durability story's correctness — it records that the
   story cannot be honestly told without the contract decision.

## §5 Next slices

- **S5 — real CogniCode EXT**: closed in `NOT_EVALUATED` state this session.
- **S6 — fake relocation**: closed in `NOT_PROCEED` state this session.
- **S7 — closeout integrated report**: can proceed once the operator
  records a decision on S4 (A/B/C). S7's job is to surface this STOP
  alongside S5 and S6 in the macro-cycle closeout.
