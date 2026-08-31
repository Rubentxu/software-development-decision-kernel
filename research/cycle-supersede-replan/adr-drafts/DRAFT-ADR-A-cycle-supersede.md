# DRAFT-ADR-A — cycle supersede as a first-class operation

> **Status**: DRAFT (not accepted). Awaiting cycle-51+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-51
> **Amends**: none (additive)
> **Supersedes**: none
> **Authority target**: `crates/sddk-cli/src/cycle.rs`

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

The SDDK cycle state machine (`crates/sddk-cli/src/cycle.rs:128-147`,
`CycleCommand` enum) exposes eight operations: `Start, Status, Transition,
EvaluateGate, Rebuild, ArtifactsDir, Lock, Inventory`. None of them
"closes a cycle goal with a reason". The closest primitives are:

- `Rebuild` — restores a missing cycle snapshot from ledger events
  (`crates/sddk-cli/src/recover.rs:13-34` shadows to it). It does NOT
  change the cycle goal.
- `archive.vault.complete` — only available for BLOCKED cycles
  (`crates/sddk-cli/src/release_cmd.rs:1605`).

This gap means: when a cycle's goal becomes invalid (scope drift,
external obsolescence, post-verify replan), there is no operation to
close it durably while preserving prior evidence. An agent or human is
forced to edit the ledger manually (AGENTS.md §8 documents this).

### Decision (proposed)

Add `CycleCommand::Supersede(CycleSupersedeArgs)` to the CLI as a
first-class operation. The operation:

1. Emits `cycle.supersede.requested` and `cycle.supersede.applied` ledger
   events (additive, follows the same envelope as `cycle.start.*`).
2. Releases the cycle's lease (idempotent with prior `lease.released`
   events).
3. Writes `supersede-receipt.json` to the cycle's XDG artifact dir.
4. Optionally links to a successor cycle (required when reason =
   `ScopeInvalid` or `GoalReplaced`).
5. Does NOT touch vault or release-receipt (those are different concerns).

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| A1 | Extend `Rebuild` with a `--supersede` flag | Conflates snapshot-restoration with goal-closure; violates single-responsibility |
| A2 | Add `archive.vault.complete` for non-BLOCKED cycles | Archive is destructive (writes to vault); supersede is non-destructive |
| A3 | Have the agent edit the ledger directly | Shifting the Burden to humans; violates recover-forward |
| A4 | Use `cycle.transition` to a terminal phase | Cycle has no terminal "superseded" phase; transition implies progression, not closure-with-reason |
| A5 | Wait for ADR-041 Workflow Runtime v2 to deliver a `Goal` primitive | Wave 1 will deliver Goal semantics, but cycle-46 plan makes this a cycle-49+ dependency; the framework cannot block on it |

### Compatibility with current ledger

- **Event schema is open-ended**. New `cycle.supersede.*` events coexist with
  `phase.*` and `cycle.*` events. No existing event is renamed, deleted,
  or re-signed.
- **Digest and event count preserved** per Wave plan §Wave 4 ("recover
  preserves canonical digest and event count"). Supersede adds exactly 2
  ledger events; the cycle's existing events remain byte-identical.
- **Backward compatibility**: readers that do not know about supersede
  ignore the new events. The cycle appears "open + has a
  supersede-receipt.json artifact"; an unknown-reader can still query the
  ledger for phase progression.
- **No new failure mode for `cycle rebuild`**: rebuild ignores the
  supersede events (they are not transitions); a rebuild on a superseded
  cycle restores the prior snapshot unchanged.

### Authority limits

- **Lease-gated** (rebuild's rule preserved): the operation requires an
  unexpired lease and a fencing token. This couples supersede to
  `cycle lock acquire`, which is currently broken (GAP-6, AGENTS.md §8).
  Cycle-51+ must fix GAP-6 before ADR-A is implemented.
- **`actor.kind` MUST be `Human` or a named AgentKind with closed-set
  authority** (per ADR-0073 — secretary has NO supersede authority).
- **No `requires_approval` flag**: supersede is structural, not
  destructive.
- **Reason field validation**: reason MUST be one of the closed-set
  `SupersedeReason::{ScopeInvalid, GoalReplaced, ExternalObsolete}`.

### Migration path

1. **Phase 1 (this research)**: ADR-A drafted; blueprint + spec drafted;
   4 evidence cards referenced (`EC-CSS-001, -007, -009, -010`).
2. **Phase 2 (cycle-51 candidate, A-min)**: implement `CycleCommand::Supersede`,
   `SupersedeReason` enum, `cycle.supersede.*` events, `supersede-receipt.json`
   schema. RED test first per cycle-36 anti-tautology discipline.
3. **Phase 3 (cycle-52+)**: orchestrator workflow exposes supersede as a
   recovery action for process gates (combines with ADR-B).
4. **Phase 4 (cycle-53+)**: `sddk recover <cycle>` is updated to NOT
   replay supersede events as transitions (clarify the rebuild vs
   supersede boundary).

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Supersede used to evade archive | low | high | Audit: supersede requires successor OR external-obsolete evidence |
| `cycle lock acquire` broken at cycle-51 | high | high | Phase 2 declares GAP-6 as a hard dependency; pre-flight repro required |
| Successor cycle scope ambiguous | medium | medium | Borrow Wave plan §Wave 1.4 `goal_hash` discipline for successor scope binding |
| Ledger digest changes | low | critical | RED test: digest byte-stable across two runs of an empty supersede |

---

## Post-decision: formal sections (for adoption)

> When this draft is accepted, the following sections replace the
> "Pre-decision" section above. The cycle that adopts the ADR is
> responsible for completing them.

### Status

(pending acceptance)

### Date

(pending acceptance)

### Consequences

(pending)

### Implementation notes

(pending)

### Compatibility / migration

See Phase 1–4 above.

### Revisit trigger

Revisit when:

- A new cycle path (A-*, B-direct) adds new terminal phases.
- ADR-041 Goal primitive ships and exposes `goal.supersede` semantics
  that should supersede this ADR.
- The `cycle lock acquire` rule is generalized (e.g., no-lease mode).

### Implementation trace

- **cycle-51** (target): implements `CycleCommand::Supersede`. Refer to
  `research/cycle-supersede-replan/blueprints/cycle-supersede.yml` and
  `research/cycle-supersede-replan/specs/SPEC-SUPERSEDE-001.md`.

---

## References

- `crates/sddk-cli/src/cycle.rs:128-147` (CycleCommand enum)
- `crates/sddk-cli/src/cycle.rs:242-269` (CycleRebuildArgs)
- `crates/sddk-cli/src/recover.rs` (35 lines, shadows to rebuild)
- `crates/sddk-cli/src/cycle.rs:886` (lease.released event)
- `crates/sddk-cli/src/knowledge_ingest.rs:64` (Authority::Superseded precedent)
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 4 (ledger invariant)
- `docs/adr/ADR-0047-durable-debt-remediation.md` (debt framework)
- `docs/adr/ADR-0073-secretary-authority.md` (AgentKind closed-set)
- `docs/research/sddk-a-full-lifecycle-review-phase-research-report.md` (Phase::Review orphan)
- `research/cycle-supersede-replan/evidence-cards/ec-css-001-cycle-supersede-vs-rebuild.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-007-recovery-action-contract.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-009-phase-review-orphan.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-010-ledger-event-count-invariant.yml`