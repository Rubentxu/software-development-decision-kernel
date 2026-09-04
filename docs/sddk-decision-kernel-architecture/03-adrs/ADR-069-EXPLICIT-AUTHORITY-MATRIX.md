# ADR-069 — Explicit Authority Matrix

**Status:** PROPOSED
**Date:** 2026-09-04
**Deciders:** orchestrator (auto-evaluator), user (post-acceptance reviewer)
**Supersedes:** none (amends ADR-0072 + ADR-0073)
**Related Work Items:** HX-AUTHORITY-001 (this cycle); ARCH-HEX-001 (order 80), EVT-LEDGER-001 (order 90), RX-SECRETARY-001 (order 300), RX-SECRETARY-002 (order 310)
**Roadmap horizon:** H0

---

## §1 Context

The current SDDK authority landscape is fragmented across prose (AGENT-EXECUTION-PROTOCOL.md), two ADRs (ADR-0072, ADR-0073), one substrate spec (SPEC-042 proposed), and shipped code whose authoritative-actor boundaries are not aligned.

`crates/sddk-domain/src/event_envelope.rs:60-69` declares a 3-variant closed set (`ActorKind::{Human, Agent, System}`) — Secretary has no enum slot, only a documented sub-role of Agent. The CLI's only typed authority boundary is a string-prefix heuristic at `crates/sddk-cli/src/cycle.rs:1217-1223`; other CLI commands (`approval grant|deny`, `cycle pause`, `cycle supersede`) accept free-form `actor: String`. The event bus hardcodes `ActorKind::Human` at `crates/sddk-engine/src/event_bus/emit.rs:259` for every `emit_approval_decision` call regardless of caller — the most critical current violation.

The exit gate names four clauses: (1) writable state, (2) approval authority, (3) provenance, (4) no-parallel-authority invariant. A-min is the right scope because (a) "accepted" in the gate is governance-not-implementation per DW-IR-002..005 precedent, (b) widening `ActorKind` / `LedgerEvent` / `EventContext` forces a schema_version bump that conflicts with the just-locked DW-IR-005 round-trip invariant at v1.81.0, (c) ADR + regression-baseline tests lock current behavior so subsequent fixes (ARCH-HEX-001, EVT-LEDGER-001) can flip them with confidence.

---

## §2 Decision §1 — Canonical 4-actor taxonomy

### Definitions

**Human** is the operator (developer, architect, QA) identified by `--actor` or `$USER`. Human is the sole admissible actor for ADR acceptance, gate outcome `Waived`, knowledge node acceptance (final approver), and release tagging (releaser). JSON serialization: `kind ∈ {human}` per `event_envelope.rs:61`.

**Agent** is the AI model-bound executor (orchestrator, leaf, secretary, advisor, evaluator). JSON serialization: `kind ∈ {agent}` per `event_envelope.rs:61`.

**System** is the CLI itself, CI, scheduler, and internal services. JSON serialization: `kind ∈ {system}` per `event_envelope.rs:61`.

**Secretary ≡ Agent{role=secretary, behavior_id, closed_set_version}** — Secretary is explicitly declared a sub-role of Agent, NOT a fifth enum variant. The Secretary's runtime identity is `Agent{role=secretary, behavior_id, closed_set_version}` per ADR-0073-AMENDMENT-1. No `ActorKind::Secretary` variant is introduced in A-min; deferred to `EVT-LEDGER-001` (order 90, H0).

### Closed-set assertion

`ActorKind` enum has exactly three variants: `Human`, `Agent`, `System`. The JSON serialization uses lowercase (`human`, `agent`, `system`) per `serde(rename_all = "lowercase")` at `event_envelope.rs:61`. No fifth JSON value is introduced in A-min.

---

## §3 Decision §2 — Writable-surface matrix

The following 8 surfaces are mutable by SDDK actors. Each row names the canonical admissible actor per surface. "Out of scope" surfaces are explicitly marked.

| Surface | Path:line | Current actor | Conflict | Proposed actor |
|---------|-----------|--------------|----------|----------------|
| Cycle state | `engine/lib.rs:1111` | caller-supplied `actor: String` | No kind check at engine boundary | System (CLI) on behalf of named caller; lease holder required |
| Ledger events (`events_v1`) | `event_bus/emit.rs` (all emitters) | caller-supplied | No kind check | System (CLI) on behalf of named caller |
| Gate receipts | `models/gate_receipt.rs:72-123` | `actor: String` + `evaluator: String` separate | No kind enforcement | System (CLI) on behalf of named evaluator |
| Plan revisions | `engine/lib.rs:1111-1156` | caller-supplied | No policy check beyond lease/plan | System (CLI) on behalf of named caller |
| Transition records | `engine/lib.rs:1111-1156` | caller-supplied | No kind check | System (CLI) on behalf of named caller |
| Framework bundle | `cli/dev/install.rs` | System (CLI) | No actor-kind tracking | System (CLI) on behalf of Human releaser |
| GitHub Releases | `cli/release_cmd.rs:834` | System (CLI) | No actor-kind tracking | System (CLI) on behalf of Human releaser |
| Knowledge graph vault | `cli/knowledge_ingest.rs:1-50` | None (no actor field) | No actor concept | System (CLI) on behalf of Human author |
| Agent scratch dirs | — | — | Out of scope for SDDK framework | Out of scope for SDDK framework |

Enforcement deferred to `ARCH-HEX-001` (engine-side authority checks) and `EVT-LEDGER-001` (LedgerEvent schema widening).

---

## §4 Decision §3 — Approval-point matrix

The following ≥12 approval points name the canonical admissible actor. Each point documents the current gap where applicable.

| Approval point | Path:line | Current actor | Gap | Canonical actor |
|----------------|-----------|--------------|-----|----------------|
| Cycle transition | `engine/lib.rs:1111-1156` | caller-supplied `actor: String` | Dual-writer: CLI + direct engine path | Lease holder (Human or Agent) |
| Cycle lock (lease acquire/release) | `cycle_pause.rs:22-43, 64-83` | `lease_owner: String` | Lease fence only | Lease holder |
| Cycle supersede | `cycle_supersede.rs:47-49, 71-90` | CLI + engine dual-path | No policy check | Lease holder |
| Gate waiver | `gate_receipt.rs:64-67` | `actor: String` same as evaluator | Manual vault write | Human only |
| Plan mutation | `engine/lib.rs:1111-1156` | caller-supplied | No policy check | Agent (orchestrator) |
| Knowledge node acceptance | `knowledge_ingest.rs:1-50` | None | No actor field | Human author |
| ADR acceptance | Vault write | Human | Manual | Human only |
| Release tagging | `release_cmd.rs:834` | System (CLI) | No kind check | Human releaser |
| Contract authoring | `cli/approval.rs:168-245` | `actor: String` | No kind check | Agent (orchestrator) |
| Approval decision | `emit.rs:259` | **FORCED Human** | Most critical violation: no caller check | Caller-supplied (default Human only when CLI caller is Human-typed) |
| Approval request | `emit.rs:191` | Forced Agent | No caller check | Agent (orchestrator) only |
| Secretary auto-resolution | ADR-0073 | ADR-0073 prose only | No code enforcement | `Agent{role=secretary}` within closed-set L1 |
| Secretary escalation | ADR-0073 | ADR-0073 prose only | No code enforcement | Orchestrator/Human resolves |

`emit_approval_decision` at `emit.rs:259` hardcodes `kind: ActorKind::Human` regardless of caller — this is the most critical current violation. Fix deferred to `ARCH-HEX-001`.

Secretary closed-set L1 bound to `actor.kind == Agent && actor.role == "secretary"` per ADR-0073-AMENDMENT-1. Stage 1 enforcement deferred to `RX-SECRETARY-001/002`.

---

## §5 Decision §4 — Provenance baseline

### ActorRef field set (canonical carrier)

`EventEnvelopeV1.actor: ActorRef` carries exactly five fields:

```
kind: ActorKind        — actor discriminator (human|agent|system)
id: String            — stable identifier within kind namespace
definition_hash       — optional; hash of behavioural definition (prompts, skills)
policy_hash           — optional; hash of applied policy bundle
model: Option<String> — optional; model identifier for agent actors
```

Evidence: `event_envelope.rs:43-57`. Adding a field requires an ADR.

### CLI prefix-string mapping (locked as v1.81.x contract)

`crates/sddk-cli/src/cycle.rs:1217-1223`:

| Prefix | ActorKind |
|--------|-----------|
| `user:*` | `ActorKind::Human` |
| `agent:*` | `ActorKind::Agent` |
| anything else | `ActorKind::System` |

Changing this mapping requires an ADR. Other CLI commands (`approval grant|deny`, `cycle pause`, `cycle supersede`) do NOT use this mapping today — this is a known gap.

### Known gaps

The following types carry `actor: String` only (no `kind`) at the engine boundary:

| Type | File:line | Owner |
|------|-----------|-------|
| `LedgerEvent.actor` | `models/ledger.rs:23-38` | EVT-LEDGER-001 |
| `EventContext.actor` | `engine/lib.rs:434-445, 1442-1463` | EVT-LEDGER-001 |
| `GateReceipt.actor` | `models/gate_receipt.rs:108-123` | EVT-LEDGER-001 |
| `JournalEntry` | `projections/journal.rs:13-32` | EVT-LEDGER-001 |
| `ApprovalState.actor` (from payload) | `projections/approval.rs:152-156, 168-171` | EVT-LEDGER-001 |

Deferred to `EVT-LEDGER-001` for typed-actor widening.

---

## §6 Decision §5 — No-parallel-authority invariant

### Invariant

**Exactly one canonical writer and one canonical approver per decision point.**

For each approval point in §4, there must be a single canonical path. Today this invariant is violated by dual-writer paths.

### Acknowledged gaps

| Decision point | Gap | Owner |
|----------------|-----|-------|
| Approval decision | `sddk approval grant\|deny` (CLI; forced Human per `emit.rs:259`) AND direct `event_store.append(emit_approval_decision(...))` from any code path | ARCH-HEX-001 |
| Cycle transition | `sddk cycle transition` (CLI; lease fence + plan revalidation) AND direct `engine::apply_transition` from any code path | ARCH-HEX-001 |
| Cycle pause/resume | `sddk cycle pause` (CLI) AND direct engine path | ARCH-HEX-001 |
| Gate receipt | Engine path + manual CLI | ARCH-HEX-001 |
| Knowledge ingest | CLI + manual vault write | ARCH-HEX-001 |
| Secretary closed-set | ADR-0073 prose prohibits `release.*` / `gate.*` / `lease.*` / `receipt.*` but no code enforcement | RX-SECRETARY-001/002 + EVT-LEDGER-001 |

Deferred to `ARCH-HEX-001` (engine-side authority checks), `RX-SECRETARY-001/002` (runtime admission predicate for Secretary), and `EVT-LEDGER-001` (enum variant + event schema).

---

## §7 Consequences

### Positive

- Canonical 4-actor taxonomy explicitly named and aligned with current 3-variant `ActorKind` enum
- Secretary declared as `Agent{role=secretary}` sub-role; no fifth enum variant introduced in A-min
- 8 writable surfaces enumerated with proposed actors
- 13 approval points enumerated (≥12 required)
- ActorRef 5-field contract locked; CLI prefix mapping locked as v1.81.x contract
- No-parallel-authority invariant declared with gap assignments to downstream H0 cycles

### Negative

- No runtime enforcement in A-min; gaps are documented, not fixed
- `ActorKind::Secretary` deferred; Secretary closed-set enforcement deferred to RX-SECRETARY-001/002
- Forced-Human in `emit_approval_decision` remains the most critical gap (P0/critical per INC-HX-AUTH-002)

### Deferred

- `ActorKind::Secretary` enum variant → EVT-LEDGER-001
- LedgerEvent/EventContext/GateReceipt/JournalEntry typed-actor widening → EVT-LEDGER-001
- Engine-side authority checks (approval, cycle, gate) → ARCH-HEX-001
- Secretary runtime admission predicate → RX-SECRETARY-001/002

---

## §8 Alternatives considered

**(a) A-lite: ADR only, no test baseline.** Rejected — without regression-baseline tests, ARCH-HEX-001 and EVT-LEDGER-001 cannot flip current behavior with confidence.

**(b) A-full: ADR + runtime enforcement in one cycle.** Rejected — schema_version bump on EventEnvelopeV1 and LedgerEvent conflicts with the just-locked DW-IR-005 round-trip invariant at v1.81.0.

**(c) Docs-only (no INC files).** Rejected — the four exit-gate clauses each require a durable debt record per ADR-0047; INC files are the mechanism.

---

## §9 Cross-references

- [ADR-0072-AMENDMENT-1](docs/adr/ADR-0072-AMENDMENT-1.md) — Secretary budget naming alignment
- [ADR-0073-AMENDMENT-1](docs/adr/ADR-0073-AMENDMENT-1.md) — Secretary closed-set L1 binding
- [INC-HX-AUTH-001](docs/debt/INC-HX-AUTH-001-writable-state.md) — writable-state surfaces gap
- [INC-HX-AUTH-002](docs/debt/INC-HX-AUTH-002-approval-authority.md) — forced-Human approval decision
- [INC-HX-AUTH-003](docs/debt/INC-HX-AUTH-003-provenance.md) — provenance loss at engine boundary
- [INC-HX-AUTH-004](docs/debt/INC-HX-AUTH-004-no-parallel-authority.md) — no-parallel-authority gaps
- ARCH-HEX-001 (order 80, H0) — engine-side authority enforcement
- EVT-LEDGER-001 (order 90, H0) — typed-actor event schema widening
- RX-SECRETARY-001/002 (order 300/310) — Secretary runtime admission predicate
- SPEC-HX-AUTHORITY-001 — this cycle's specification
- AGENT-EXECUTION-PROTOCOL §14 — operator identity contract

---

## §10 Acceptance criteria references

| AC | Decision section | Coverage |
|----|-----------------|----------|
| AC-HX-AUTH-01 | §1 | Canonical 4-actor taxonomy; closed-set assertion |
| AC-HX-AUTH-02 | §2 | 8 writable surfaces with proposed actors |
| AC-HX-AUTH-03 | §3 | 13 approval points; forced-Human documented as broken |
| AC-HX-AUTH-04 | §4 | ActorRef 5-field contract + CLI prefix lock + gap enumeration |
| AC-HX-AUTH-05 | §5 | No-parallel-authority invariant + gaps table |
| AC-HX-AUTH-06 | ADR-0072/0073 amendments | Secretary bound to Agent sub-role |
| AC-HX-AUTH-07 | Test file | ≥6 regression-baseline tests |
| AC-HX-AUTH-08 | INC files | 4 INC files, one per exit-gate clause |
