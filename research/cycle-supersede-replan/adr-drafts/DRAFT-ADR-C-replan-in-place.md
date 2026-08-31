# DRAFT-ADR-C — replan-in-place as a cycle operation

> **Status**: DRAFT (not accepted). Awaiting cycle-53+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-53 (depends on ADR-A)
> **Amends**: none (additive)
> **Supersedes**: none
> **Authority target**: `crates/sddk-cli/src/cycle.rs`

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

When `verify` discovers that a cycle's scope is invalid (post-mortem:
the cycle was solving the wrong problem), the framework has no formal
path back. The current options are:

- Manually edit the ledger (AGENTS.md §8 documents this as a
  last-resort).
- Open a brand-new cycle with a different scope, abandoning the prior
  cycle (loses the evidence trail).

Neither satisfies "recover forward para proceso". The prior cycle had
real evidence (proposal, spec, design, apply progress); throwing it away
violates ADR-0047 §4 ("Los artefactos se conservan por defecto").

### Decision (proposed)

Add `CycleCommand::Replan(CycleReplanArgs)` to the CLI:

1. The replan operation takes `(cycle_id, restage_to, reason,
   evidence_refs, actor)`.
2. It emits:
   - `cycle.replan.requested` (carries `restage_to`, `reason`,
     `evidence_refs`).
   - `cycle.supersede.requested` (chained — closes prior cycle with
     reason).
   - `cycle.start.requested` for the successor cycle (same scope
     binding, new name suffix `-replan-N`).
3. Total: 3 new ledger events for the prior cycle + 1 start event for
   the successor (4 events).
4. The successor cycle is bound by `(project, scope, name)` tuple per
   Wave plan §Wave 1.4. If a successor already exists with the same
   tuple, the orchestrator emits `ENGINE_AMBIGUOUS_SCOPE` and rejects.

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| C1 | Extend `Rebuild` with a `--restage` flag | Conflates snapshot-restoration with replan; violates single-responsibility |
| C2 | Use `cycle.transition` backwards (verify → tasks) | No backward transition exists; transitions are progression-only |
| C3 | Open a brand-new cycle with the same name | Scope collision (Wave plan §Wave 1.4 `ENGINE_AMBIGUOUS_SCOPE`) |
| C4 | Have the agent re-run phases manually | Shifting the Burden; violates recover-forward |

### Compatibility with current ledger

- **4 new event types** (`cycle.replan.*` + chain). Existing events
  unchanged.
- **Digest and event count preserved** per Wave plan §Wave 4 invariant.
  Replan adds exactly 4 events to the ledger (one chain + one successor
  start).
- **Successor cycle scope is bound** at replan time (not deferred). This
  prevents ambiguity collisions.
- **Backward compatibility**: a replan creates a new cycle id; the prior
  cycle is closed via supersede (ADR-A). Readers that don't yet
  recognize replan see two cycles: the original (closed, with
  supersede-receipt.json) and the new one (open, with start event).

### Authority limits

- **Lease-gated** (inherits from cycle.supersede).
- **`actor.kind` MUST be `Human` OR an AgentKind with `replan_authority`
  flag** (closed set per ADR-0073 — secretary has NO replan authority).
- **`reason` MUST be ≥ 32 chars** (cannot be gamed by an LLM with a
  one-word justification).
- **`restage_to=Apply` requires `--confirm-apply`** flag (Apply is
  destructive; other restage_to values are exploratory).
- **Replan is path-bounded**: the successor cycle must follow the same
  path (A-min, A-lite, A-full, B-direct) as the prior cycle. Path
  change is not a replan; it's a new cycle.
- **Replan counter limit**: max 5 replans per cycle id (cycle-N →
  cycle-N-replan-1 → ... → cycle-N-replan-5); the 6th attempt fails
  with `ENGINE_REPLAN_LIMIT_EXCEEDED` and forces a manual review.

### Migration path

1. **Phase 1 (this research)**: ADR-C drafted; blueprint + spec drafted.
2. **Phase 2 (cycle-53 candidate, A-min)**: implement
   `CycleCommand::Replan`, `cycle.replan.*` event schema. RED test
   first.
3. **Phase 3 (cycle-54+)**: orchestrator workflow exposes replan as a
   recovery action for process gates (combines with ADR-B + ADR-G).

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Replan used to evade verification | medium | high | Require evidence_refs (cannot be empty); reason ≥ 32 chars |
| Infinite replan loop | medium | medium | Replan counter (max 5); cycle-N-replan-6 fails |
| Successor scope collision | low | high | ENGINE_AMBIGUOUS_SCOPE (Wave plan §Wave 1.4) |
| Ledger event count growth | low | medium | Documented; per ADR-0047, durable is intentional |
| Apply-restage destructive | medium | high | `--confirm-apply` flag required for `restage_to=Apply` |

---

## Post-decision: formal sections (for adoption)

### Status

(pending acceptance)

### Date

(pending acceptance)

### Consequences

(pending)

### Implementation notes

- New variant: `CycleCommand::Replan(CycleReplanArgs)` in
  `crates/sddk-cli/src/cycle.rs:128`.
- New events: `cycle.replan.requested`, `cycle.replan.applied`,
  `cycle.replan.successor_started`.
- New error code: `ENGINE_AMBIGUOUS_SCOPE` (existing) +
  `ENGINE_REPLAN_LIMIT_EXCEEDED` (new).

### Compatibility / migration

See Phase 1–3 above.

### Revisit trigger

Revisit when:

- A replan counter limit is hit in practice (sign of chronic replan
  abuse).
- The Wave plan ships a `Goal` primitive (ADR-041) that subsumes
  replan-as-recovery.

### Implementation trace

- **cycle-53** (target): implements replan-in-place. Refer to
  `research/cycle-supersede-replan/blueprints/replan-in-place.yml` and
  `research/cycle-supersede-replan/specs/SPEC-REPLAN-001.md`.

---

## References

- `crates/sddk-cli/src/cycle.rs:128-147` (CycleCommand enum)
- `crates/sddk-cli/src/recover.rs` (35 lines, shadows to rebuild — NOT replan)
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 1.4 (scope binding)
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 4 (ledger invariant)
- `docs/adr/ADR-0047-durable-debt-remediation.md` (artefact conservation)
- `docs/adr/ADR-0073-secretary-authority.md` (AgentKind closed-set)
- `research/cycle-supersede-replan/evidence-cards/ec-css-003-replan-no-primitive.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-005-cycle-vs-hypothesis.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-010-ledger-event-count-invariant.yml`