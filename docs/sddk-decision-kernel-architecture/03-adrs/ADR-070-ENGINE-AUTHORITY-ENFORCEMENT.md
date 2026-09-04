# ADR-070 — Engine Authority Enforcement

**Status:** PROPOSED
**Date:** 2026-09-04
**Deciders:** orchestrator (auto-evaluator), user (post-acceptance reviewer)
**Supersedes:** none
**Related Work Items:** ARCH-HEX-001 (order 80, H0); HX-AUTHORITY-001 (SHIPPED v1.82.0); EVT-LEDGER-001 (order 90); RX-SECRETARY-001/002 (order 300/310)
**Roadmap horizon:** H0

---

## §1 Context

The exit gate for `ARCH-HEX-001` requires that domain/application/adapter boundaries for H1-H3 are **clean and enforced by tests/checks**. ADR-069 §§3–6 documented the gap: the engine boundary does not enforce actor-kind policy. `emit_approval_decision` hardcodes `ActorKind::Human` at `event_bus/emit.rs:259`; `apply_transition`, `cycle_pause`, `cycle_resume`, `cycle_supersede` accept any `actor: String` without kind validation; the CLI prefix heuristic at `cycle.rs:1217-1223` is duplicated nowhere else.

Five boundary debts (PLN-LEDGER-001..004, DW-RUNTIME-002..005, DEC-PLANE-001..004) reduce to this structural defect. Three INC files owned by ARCH-HEX-001 (INC-HX-AUTH-001 writable-state, INC-HX-AUTH-002 forced-Human, INC-HX-AUTH-004 no-parallel-authority) are open and blocking H1-H3.

ADR-070 formalises the engine-side enforcement mechanism and closes the INC files. The domain layer is **not** widened. `ActorKind` stays 3-variant. `LedgerEvent`/`EventContext`/`GateReceipt`/`JournalEntry` are **not** modified.

---

## §2 Decision

### §2.1 AuthorityContext

```rust
/// Caller-supplied authority record.
#[derive(Debug, Clone)]
pub struct AuthorityContext {
    pub actor_kind: ActorKind,
    pub actor_id: String,
    pub lease_owner: Option<String>,
    pub fencing_token: Option<i64>,
}
```

`AuthorityContext` bundles the actor identity with optional lease metadata. It is constructed at CLI call sites via `AuthorityContext::for_cli(actor_id, actor_kind, lease_owner, fencing_token)` and passed to engine entry points. Engine-side validation consults the writable-surface matrix before any ledger mutation.

### §2.2 infer_actor_kind

```rust
pub fn infer_actor_kind(actor_id: &str) -> ActorKind {
    if actor_id.starts_with("user:") {
        ActorKind::Human
    } else if actor_id.starts_with("agent:") {
        ActorKind::Agent
    } else {
        ActorKind::System
    }
}
```

Locked v1.81.x prefix heuristic. Replaces the inline chain at `cycle.rs:1217-1223`. Both CLI call sites (`cycle.rs` and `approval.rs`) call this helper; no duplication.

### §2.3 WritableSurface enum

```rust
pub enum WritableSurface {
    CycleState,
    LedgerEvents,
    GateReceipts,
    PlanRevisions,
    TransitionRecords,
    FrameworkBundle,
    GithubReleases,
    KnowledgeGraphVault,
}
```

Eight surfaces matching ADR-069 §3. Each maps to admitted `ActorKind` sets via the matrix below.

### §2.4 Four engine entry points

Each of the following engine methods gains an `auth: &AuthorityContext` parameter. The method calls `auth.validate(WritableSurface::<surface>)?;` as its first statement before any ledger mutation. On mismatch, returns `EngineError::AuthorityContextRejected { surface, kind, reason }`.

| Entry point | Surface | Additional check |
|---|---|---|
| `apply_transition` | `TransitionRecords` | — |
| `cycle_pause` | `CycleState` | `auth.lease_owner` matches supplied lease |
| `cycle_resume` | `CycleState` | — |
| `cycle_supersede` | `CycleState` | `auth.lease_owner` matches supplied lease |

### §2.5 Event-bus changes

**`ApprovalDecisionInput`** gains `actor_kind: ActorKind` (caller-supplied). `emit_approval_decision` replaces `kind: ActorKind::Human` (line 259) with `kind: input.actor_kind`. Validates `actor_kind ∈ {Human, System}` at emit time.

**`ApprovalRequestedInput`** gains `actor_kind: ActorKind` (caller-supplied). `emit_approval_requested` replaces `kind: ActorKind::Agent` (line 191) with `kind: input.actor_kind`. Validates `actor_kind ∈ {Agent}` at emit time.

`EventEnvelopeV1.compute_content_hash` is **unchanged** and **invariant**. Different `actor_kind` values produce different hashes — the expected observable per ADR-068 §1.

### §2.6 INC closures

| INC | Status transition | Lifecycle evidence |
|---|---|---|
| INC-HX-AUTH-001 writable-state | `open → closed` | 2026-09-04; ARCH-HEX-001; engine AuthorityContext validates CycleState + TransitionRecords surfaces |
| INC-HX-AUTH-002 approval-authority | `open → closed` | 2026-09-04; ARCH-HEX-001; ApprovalDecisionInput.actor_kind caller-supplied; emit_approval_decision uses input.actor_kind |
| INC-HX-AUTH-004 no-parallel-authority | `open` (sub-lifecycle) | 4 of 7 paths closed by ARCH-HEX-001: (1) approval decision, (2) approval request, (3) cycle transition, (4) cycle pause/resume. Remaining 3 paths deferred to EVT-LEDGER-001 + RX-SECRETARY-001/002 |
| INC-HX-AUTH-003 provenance | **unchanged** | Owned by EVT-LEDGER-001 |

---

## §3 Writable-Surface Matrix

| Surface | Admitted ActorKind |
|---|---|
| `CycleState` | Human, Agent, System |
| `LedgerEvents` | Human, Agent, System |
| `GateReceipts` | System |
| `PlanRevisions` | Human, Agent |
| `TransitionRecords` | Human, Agent, System |
| `FrameworkBundle` | System |
| `GithubReleases` | System |
| `KnowledgeGraphVault` | Human |

Implementation: `crates/sddk-engine/src/authority.rs::WRITABLE_SURFACE_MATRIX`.

---

## §4 Consequences

### Positive

- Engine boundary enforces actor-kind policy at runtime (fail-closed).
- Forced-Human in `emit_approval_decision` is replaced by caller-supplied provenance.
- CLI is the canonical writer for all four dual-writer paths; non-CLI callers are rejected.
- DRY violation in CLI prefix heuristic eliminated.
- 4 of 7 INC-HX-AUTH-004 paths closed.
- H1 ledger work unblocked (trustworthy `actor.kind` on approval events).

### Negative

- Engine entry-point signatures gain a required parameter (API churn for any future non-CLI caller).
- Tests that call engine methods directly must be updated to supply `AuthorityContext::for_test()`.

### Neutral

- `LedgerEvent.actor` stays `String` (INC-HX-AUTH-003, deferred to EVT-LEDGER-001).
- `ActorKind` stays 3-variant (no `Secretary`; deferred to EVT-LEDGER-001).
- `GateReceipt`, `EventContext`, `JournalEntry` are not modified.

---

## §5 Alternatives considered

| Alternative | Why rejected |
|---|---|
| Governance-only ADR (no code) | Exit gate requires "enforced by tests/checks"; INC-HX-AUTH-002 (P0) stays open |
| Add `ActorKind::Secretary` to domain enum | Breaks ADR-069 3-variant contract; schema_version bump conflicts with DW-IR-005 invariants |
| Remove engine entry points entirely | Architecture stays the same; only trust boundary moves |

---

## §6 Cross-references

- [ADR-069](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md) — Explicit Authority Matrix (precedent)
- [ADR-068](docs/sddk-decision-kernel-architecture/03-adrs/ADR-068-DETERMINISTIC-IR-RUNTIME-BOUNDARY.md) — Deterministic IR Runtime Boundary
- [INC-HX-AUTH-001](docs/debt/INC-HX-AUTH-001-writable-state.md) — writable-state surfaces
- [INC-HX-AUTH-002](docs/debt/INC-HX-AUTH-002-approval-authority.md) — forced-Human default
- [INC-HX-AUTH-004](docs/debt/INC-HX-AUTH-004-no-parallel-authority.md) — no-parallel-authority invariant
- [ARCH-HEX-001](SPEC-ARCH-HEX-001.md) — Engine Authority Enforcement (spec)
- [exploration-report.md](exploration-report.md) — investigation findings

---

## §7 Acceptance Criteria Mapping

| AC | Criterion | Evidence |
|---|---|---|
| AC-ARCH-HEX-01 | Layer purity | `grep -RE 'use sddk_(engine\|...)' crates/sddk-domain/src` → 0 matches |
| AC-ARCH-HEX-02 | Writable-state registry | `authority.rs` exports all required items; `WRITABLE_SURFACE_MATRIX` has 8 rows |
| AC-ARCH-HEX-03 | Forced-Human closure | `emit_approval_decision` uses `input.actor_kind`; round-trip test passes with `ActorKind::Agent` |
| AC-ARCH-HEX-04 | Engine AuthorityContext | 4 entry points accept `auth: &AuthorityContext`; `EngineError::AuthorityContextRejected` on mismatch |
| AC-ARCH-HEX-05 | Dual-writer elimination (paths 1-4) | Non-CLI callers with wrong `actor_kind` are rejected at engine boundary |
| AC-ARCH-HEX-06 | INC closures | Frontmatter `status: closed` for INC-001 & INC-002; INC-004 has "4 of 7 paths closed" annotation |
| AC-ARCH-HEX-07 | DRY prefix heuristic | `infer_actor_kind` is the single canonical helper; CLI sites call it |
| AC-ARCH-HEX-08 | Determinism preservation | `EventEnvelopeV1.compute_content_hash` unchanged; `roundtrip_preserves_content_hash` PASS |
| AC-ARCH-HEX-09 | No carryover regression | FIND-000001..005 stay OPEN; `cargo clippy --workspace --all-targets -- -D errors` passes |
| AC-ARCH-HEX-10 | Regression-baseline preservation | 8 tests in `actor_authority_baseline_tests.rs` continue to PASS unchanged |

---

> **Acceptance:** This ADR is PROPOSED on creation and becomes ACCEPTED when the verify phase confirms all 10 ACs green and the cycle archive is written.
