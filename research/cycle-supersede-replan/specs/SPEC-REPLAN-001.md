# SPEC-REPLAN-001 — `cycle.replan.*` event schema and successor binding

> **Status**: DRAFT (not adopted). Awaiting cycle-53+ implementation.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-53 (depends on ADR-A,
> ADR-C)
> **Companion ADRs**: DRAFT-ADR-A-cycle-supersede,
> DRAFT-ADR-C-replan-in-place

---

## 1. Scope

This spec defines:

1. The shape of the `cycle.replan.requested` and `cycle.replan.applied`
   ledger events.
2. The relationship between replan events, supersede events
   (chained), and successor cycle start events.
3. The successor cycle binding rule (per Wave plan §Wave 1.4).
4. The replan counter limit (max 5 replans per cycle id).

This spec does **not** define:

- The CLI command surface (see ADR-C).
- The `DesignDecision` primitive (see ADR-E).
- The orchestrator workflow integration (cycle-54+).

---

## 2. Event: `cycle.replan.requested`

```json
{
  "event_id": "evt-<uuid>",
  "event_type": "cycle.replan.requested",
  "schema_version": 1,
  "prior_cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-transition",
  "replan_counter": 1,
  "command_id": "cycle.replan-<uuid>",
  "restage_to": "Specify",
  "reason": "post-verify scope review determined the original specification was incomplete; the new specification should address the gap identified in verify-report.md §3.2",
  "evidence_refs": [
    {
      "kind": "verify-report",
      "path": "verify-report.md"
    }
  ],
  "actor": {
    "kind": "human",
    "id": "user:rubentxu"
  },
  "timestamp": "2026-09-20T10:00:00Z",
  "lease_owner": "user:rubentxu",
  "fencing_token": 12345,
  "confirm_apply": false,
  "causation_id": "evt-<uuid-of-cycle.transition-that-led-here>",
  "correlation_id": "corr-<uuid>"
}
```

### Field semantics

| Field | Type | Required | Source |
|---|---|---|---|
| `event_id` | string | yes | UUIDv4 |
| `event_type` | string | yes | literal `cycle.replan.requested` |
| `schema_version` | u32 | yes | 1 |
| `prior_cycle_id` | string | yes | the cycle being replanned |
| `replan_counter` | u32 | yes | 1..=5; max 5 replans per cycle |
| `command_id` | string | yes | `cycle.replan-<uuid>` |
| `restage_to` | enum | yes | `Propose | Specify | Design | Tasks | Apply` |
| `reason` | string | yes | min 32 chars (cannot be gamed) |
| `evidence_refs` | Vec | yes | min 1 entry; cannot be empty |
| `actor.kind` | enum | yes | `human` or `agent` |
| `actor.id` | string | yes | user or agent identifier |
| `timestamp` | RFC3339 | yes | deterministic |
| `lease_owner` | string | yes | from cycle.lock acquire |
| `fencing_token` | i64 | yes | from cycle.lock acquire |
| `confirm_apply` | bool | yes | required `true` if `restage_to=Apply` |
| `causation_id` | string | optional | prior event in causal chain |
| `correlation_id` | string | optional | correlation across events |

### Validation rules

1. `restage_to` MUST be one of the closed set.
2. `reason.length() >= 32`.
3. `evidence_refs.len() >= 1`.
4. `replan_counter <= 5`; counter 6+ fails with
   `ENGINE_REPLAN_LIMIT_EXCEEDED`.
5. `restage_to=Apply` requires `confirm_apply=true`; otherwise fails
   with `ENGINE_APPLY_REQUIRES_CONFIRM`.
6. `actor.kind == agent` requires `replan_authority=true` (per
   ADR-0073 closed-set).
7. `lease_owner` and `fencing_token` MUST match an active lease.

---

## 3. Event: `cycle.replan.applied`

```json
{
  "event_id": "evt-<uuid>",
  "event_type": "cycle.replan.applied",
  "schema_version": 1,
  "prior_cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-transition",
  "replan_counter": 1,
  "command_id": "cycle.replan-<uuid>",
  "supersede_receipt_id": "sup-<uuid>",
  "successor_cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-transition-replan-1",
  "successor_start_event_id": "evt-<uuid>",
  "actor": {
    "kind": "human",
    "id": "user:rubentxu"
  },
  "timestamp": "2026-09-20T10:00:01Z",
  "causation_id": "evt-<uuid-of-cycle.replan.requested>",
  "correlation_id": "corr-<uuid>"
}
```

### Field semantics

| Field | Type | Required | Source |
|---|---|---|---|
| `successor_cycle_id` | string | yes | derived from prior + counter: `<prior_id>-replan-<N>` |
| `successor_start_event_id` | string | yes | the `cycle.start.requested` event for the successor |

### Validation rules

1. `causation_id` MUST point to a `cycle.replan.requested` event within
   the same transaction.
2. `supersede_receipt_id` MUST exist (from chained supersede per
   SPEC-SUPERSEDE-001).
3. `successor_cycle_id` MUST NOT collide with any existing
   `(project, scope, name)` tuple (per Wave plan §Wave 1.4 — fail with
   `ENGINE_AMBIGUOUS_SCOPE`).

---

## 4. Successor cycle binding

The successor cycle is created in the same transaction as the replan.
Its `cycle.start.requested` event carries:

```json
{
  "event_type": "cycle.start.requested",
  "cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-transition-replan-1",
  "scope_binding": "<inherited from prior cycle>",
  "replan_of": "p-52b95ef55999f9de/cycle-44-build-remediate-transition",
  "replan_counter": 1,
  ...
}
```

### Naming rule

`<prior_id>-replan-<N>` where `N` is `replan_counter`.

Examples:

| Replan N | Successor cycle id |
|---|---|
| 1 | `<prior>-replan-1` |
| 2 | `<prior>-replan-2` |
| ... | ... |
| 5 | `<prior>-replan-5` (last allowed) |
| 6 | fails with `ENGINE_REPLAN_LIMIT_EXCEEDED` |

### Path inheritance

The successor cycle follows the **same path** (A-min, A-lite, A-full,
B-direct) as the prior cycle. Path change is not a replan; it's a new
cycle.

---

## 5. Ledger event count

Per Wave plan §Wave 4 invariant, replan adds exactly **4 events** to the
ledger:

1. `cycle.replan.requested`
2. `cycle.supersede.requested` (chained)
3. `cycle.supersede.applied` (chained)
4. `cycle.start.requested` (for the successor)

The cycle's existing events remain unchanged.

### Test invariant

```rust
#[test]
fn replan_preserves_ledger_digest() {
    let pre_digest = ledger.digest();

    run_replan(...)?;

    let post_digest = ledger.digest();

    // Existing events unchanged
    assert_eq!(pre_digest.events[0..N], post_digest.events[0..N]);

    // Exactly 4 new events appended
    assert_eq!(post_digest.events.len(), pre_digest.events.len() + 4);
}
```

---

## 6. Replan counter limit

The replan counter is per cycle id, not per cycle lineage. A successor
cycle (`<prior>-replan-1`) has its own counter starting at 1.

```text
cycle-44 ──► cycle-44-replan-1 ──► cycle-44-replan-1-replan-1
                                          ▲
                                          counter=1 (own lineage)
```

If `replan_counter > 5`, the operation fails with
`ENGINE_REPLAN_LIMIT_EXCEEDED`. The orchestrator should emit a
diagnostic and require manual intervention (cycle supersede via ADR-A).

### Reason for the limit

A high replan counter signals chronic hypothesis instability. The
limit forces a human to step back and consider whether the cycle's
goal itself is wrong (in which case `cycle.supersede` with reason
`GoalReplaced` is the correct path).

---

## 7. Compatibility

### With existing ledger

- 4 new event types (additive); no existing event renamed.
- Cycle manifest gains `replan_of: Option<CycleId>` and `replan_counter:
  u32` (additive fields, default `None`/`0`).
- Digest and event count preserved.

### With existing cycle operations

- `cycle rebuild` on a successor cycle restores from its own events;
  replan events are NOT replayed.
- `cycle supersede` on a successor cycle is allowed (independently of
  replan).
- `cycle transition` on a successor cycle is the standard workflow.

### With `DesignDecision` (ADR-E)

When ADR-E ships, replan MAY also carry a `decision_id` field linking
the new cycle to a fresh design decision. **For this spec, replan does
not require `decision_id`**; it is independent of ADR-E.

---

## 8. Open questions

1. **Is `replan_counter` per cycle id or per lineage?** Per cycle id
   (each cycle has its own counter, starting at 1). **Decision
   (proposed)**: per cycle id.
2. **Can replan itself trigger replan?** Yes, but each is bounded by
   its own counter. **Decision (proposed)**: yes, with counter reset on
   each successor.
3. **What if the prior cycle was superseded before replan?** Cannot
   happen; replan requires the prior cycle to be open. **Decision
   (proposed)**: prior cycle MUST be in `OPEN` status.

---

## 9. References

- `research/cycle-supersede-replan/blueprints/replan-in-place.yml`
- `research/cycle-supersede-replan/specs/SPEC-SUPERSEDE-001.md`
- `research/cycle-supersede-replan/evidence-cards/ec-css-003-replan-no-primitive.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-005-cycle-vs-hypothesis.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-010-ledger-event-count-invariant.yml`
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 1.4
  (scope binding)
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 4
  (ledger invariant)
- `docs/adr/ADR-0073-secretary-authority.md` (AgentKind closed-set)