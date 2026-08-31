# DRAFT-ADR-E — Cycle vs Hypothesis: separation of concerns

> **Status**: DRAFT (not accepted). Awaiting cycle-54+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-54 (depends on ADR-A)
> **Amends**: none (additive)
> **Supersedes**: none
> **Authority target**: `crates/sddk-domain` (new types)

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

The runtime has 10 phases
(`crates/sddk-domain/src/cycle.rs::Phase::{Explore, Specify, Design,
Plan, Build, Verify, Uat, Review, Release, Archive}`), but the prompt
layer executes only `tasks → apply → verify → debt-verify → release →
archive` — no `Phase::Review` executor exists
(`docs/research/sddk-a-full-lifecycle-review-phase-research-report.md`
L1.S11 documents this).

A cycle represents a **product goal**. A design decision represents a
**hypothesis about how to achieve that goal**. Today, a failed
hypothesis (a wrong design) requires the entire cycle to be abandoned —
even though the goal itself may still be valid. This violates
"recover forward para proceso" at the most fundamental level: the
framework confuses goal and means.

### Decision (proposed)

Introduce a `DesignDecision` domain primitive in
`crates/sddk-domain/src/decision.rs`:

```rust
pub struct DesignDecision {
    pub decision_id: DecisionId,           // stable
    pub cycle_id: CycleId,                  // bound to one cycle
    pub definition_hash: Sha256,            // sha256(definition)
    pub supersedes: Option<DecisionId>,     // chain of decisions
    pub status: DecisionStatus,             // Active | Superseded | Failed
    pub evidence: Vec<ArtifactRef>,
    pub created_at: OffsetDateTime,
    pub superseded_at: Option<OffsetDateTime>,
    pub superseded_by: Option<DecisionId>,
}

pub enum DecisionStatus {
    Active,       // currently in use
    Superseded,   // replaced by a newer decision in the same cycle
    Failed,       // hypothesis was wrong; no successor
}
```

The cycle manifest carries a `current_decision_id` field. When a
hypothesis fails:

1. Emit `decision.failed` ledger event (decision_id, cycle_id, reason).
2. Optionally emit `decision.superseded` to chain to a successor
   decision (without abandoning the cycle).
3. The cycle continues; only the `current_decision_id` changes.

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| E1 | Use existing `RepairAction` enum | Too narrow (NodeCreation, PlainTextRewrite only); not general enough |
| E2 | Add a `DesignHypothesis` type in `crates/sddk-domain` | "Hypothesis" implies testability; "Decision" is the actionable artifact |
| E3 | Defer to ADR-041 Goal primitive | Cycle-46 plan makes Goal delivery a cycle-49+ dependency; the framework cannot block on it for ad-hoc decisions |
| E4 | Implement as a workflow-level state (not domain) | Loses typed access from CLI; harder to evolve |

### Compatibility with current ledger

- **New domain type**, no existing type renamed.
- **New event types**: `decision.created`, `decision.failed`,
  `decision.superseded`, `decision.activated`.
- **Cycle manifest gains `current_decision_id: Option<DecisionId>`** —
  additive field, default `None` (backward compatible).
- **Digest and event count preserved**: decision events are new events;
  no existing event is renamed.

### Authority limits

- **Decisions are bound to one cycle** — a decision cannot be
  cross-cycle (cycles have separate scope).
- **Supersession chain is finite**: depth ≤ 10. Beyond 10, the cycle is
  flagged as "decision churn" and a human review is required.
- **`decision.failed` does NOT close the cycle** — it marks the
  hypothesis as failed; the cycle may continue with a new decision or be
  closed via supersede (ADR-A).

### Migration path

1. **Phase 1 (this research)**: ADR-E drafted; evidence card referenced.
2. **Phase 2 (cycle-54 candidate, A-full)**: implement `DesignDecision`
   type, decision events, cycle manifest field. RED test first.
3. **Phase 3 (cycle-55+)**: orchestrator workflow emits decision events
   automatically when an apply phase fails (with explicit human
   confirmation).
4. **Phase 4 (cycle-56+)**: re-evaluate `Phase::Review` (currently
   orphan) — with the new decision primitive, Review becomes a
   *decision review* (Active decision under review) rather than a
   phase.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Decision churn abuse | medium | medium | Depth ≤ 10; flag at threshold |
| Decision supersession chain breaks | low | high | SHA-256 chain; integrity check on load |
| Migration breaks existing manifests | low | medium | `current_decision_id: Option<DecisionId>` defaults to `None` |

---

## Post-decision: formal sections (for adoption)

### Status

(pending acceptance)

### Date

(pending acceptance)

### Consequences

(pending)

### Implementation notes

- New file: `crates/sddk-domain/src/decision.rs`.
- New events: `decision.created`, `decision.failed`,
  `decision.superseded`, `decision.activated`.
- Modified manifest: `CycleManifest.current_decision_id: Option<DecisionId>`.

### Compatibility / migration

See Phase 1–4 above.

### Revisit trigger

Revisit when:

- ADR-041 Goal primitive ships and exposes a more general primitive.
- Decision churn becomes chronic (sign of hypothesis instability).

### Implementation trace

- **cycle-54** (target): implements DesignDecision. Refer to
  `research/cycle-supersede-replan/evidence-cards/ec-css-005-cycle-vs-hypothesis.yml`
  and `research/cycle-supersede-replan/evidence-cards/ec-css-009-phase-review-orphan.yml`.

---

## References

- `docs/research/sddk-a-full-lifecycle-review-phase-research-report.md` (Phase::Review orphan)
- `crates/sddk-domain/src/cycle.rs::Phase` (10 phases)
- `crates/sddk-cli/src/knowledge_ingest.rs:64` (Authority::Superseded precedent)
- `docs/adr/ADR-0047-durable-debt-remediation.md` §4 (artefact conservation)
- `docs/adr/ADR-041-WORKFLOW-RUNTIME-V2.md` (Goal primitive, future)