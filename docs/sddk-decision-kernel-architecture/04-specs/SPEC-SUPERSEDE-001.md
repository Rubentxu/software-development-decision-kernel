# SPEC-SUPERSEDE-001 — `cycle.supersede.*` event schema and `supersede-receipt.json`

**Status:** accepted
**Slug:** SPEC-SUPERSEDE-001
**Domain:** kernel
**Created:** 2026-08-31
**Created in cycle:** [[p-63676b11dc0ef88f/kernel-cycle-51-supersede-first-class]]
**Decision authority:** [[ADR-0079-cycle-supersede]]
**Version:** 1
**Owners:** SDDK Team
**Stale after:** 2027-08-31
**References:**
  - [[ADR-0079-cycle-supersede]]
  - [[SPEC-042-secretary-runtime]] (reused lease event pattern)

---

## 1. Scope

This spec defines:

1. The shape of the `cycle.supersede.requested` and `cycle.supersede.applied`
   ledger events.
2. The shape of `supersede-receipt.json` written to the cycle's XDG
   artifact dir.
3. The relationship between supersede events and existing ledger events.
4. The placement of the receipt in the XDG hierarchy.

This spec does **not** define:

- The CLI command surface (`crates/sddk-cli/src/cycle.rs` — see ADR-A).
- The orchestrator workflow integration (cycle-52+).
- The cycle.lock acquire dependency (GAP-6, out of scope here).

---

## 2. Event: `cycle.supersede.requested`

```json
{
  "event_id": "evt-<uuid>",
  "event_type": "cycle.supersede.requested",
  "schema_version": 1,
  "cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-transition",
  "command_id": "cycle.supersede-<uuid>",
  "actor": {
    "kind": "human",
    "id": "user:rubentxu"
  },
  "timestamp": "2026-09-15T10:00:00Z",
  "reason": "ScopeInvalid",
  "evidence_refs": [
    {
      "kind": "verify-report",
      "path": "verify-report.md"
    },
    {
      "kind": "debt-report",
      "path": "debt-report.json"
    }
  ],
  "successor_cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-replacement",
  "lease_owner": "user:rubentxu",
  "fencing_token": 12345,
  "causation_id": "evt-<uuid-of-cycle.transition-that-led-here>",
  "correlation_id": "corr-<uuid>"
}
```

### Field semantics

| Field | Type | Required | Source |
|---|---|---|---|
| `event_id` | string | yes | UUIDv4 |
| `event_type` | string | yes | literal `cycle.supersede.requested` |
| `schema_version` | u32 | yes | 1 |
| `cycle_id` | string | yes | `CycleId` |
| `command_id` | string | yes | `cycle.supersede-<uuid>` |
| `actor.kind` | enum | yes | `human` or `agent` |
| `actor.id` | string | yes | user or agent identifier |
| `timestamp` | RFC3339 | yes | deterministic |
| `reason` | enum | yes | `ScopeInvalid | GoalReplaced | ExternalObsolete` |
| `evidence_refs` | Vec | yes | min 1 entry; cannot be empty |
| `successor_cycle_id` | string | conditional | required iff reason ∈ {ScopeInvalid, GoalReplaced} |
| `lease_owner` | string | yes | from cycle.lock acquire |
| `fencing_token` | i64 | yes | from cycle.lock acquire |
| `causation_id` | string | optional | prior event in causal chain |
| `correlation_id` | string | optional | correlation across events |

### Validation rules

1. `reason` MUST be one of the closed set
   `ScopeInvalid | GoalReplaced | ExternalObsolete`.
2. `evidence_refs` MUST have at least 1 entry.
3. `actor.kind == agent` requires the agent to have
   `replan_authority=true` (per ADR-0073 closed-set).
4. `lease_owner` and `fencing_token` MUST match an active lease.
5. `successor_cycle_id` MUST NOT equal `cycle_id` (self-supersede is
   forbidden).

---

## 3. Event: `cycle.supersede.applied`

```json
{
  "event_id": "evt-<uuid>",
  "event_type": "cycle.supersede.applied",
  "schema_version": 1,
  "cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-transition",
  "command_id": "cycle.supersede-<uuid>",
  "supersede_receipt_id": "sup-<uuid>",
  "lease_released": true,
  "actor": {
    "kind": "human",
    "id": "user:rubentxu"
  },
  "timestamp": "2026-09-15T10:00:01Z",
  "causation_id": "evt-<uuid-of-cycle.supersede.requested>",
  "correlation_id": "corr-<uuid>"
}
```

### Validation rules

1. `causation_id` MUST point to a `cycle.supersede.requested` event
   within the same cycle.
2. `supersede_receipt_id` MUST exist as a file under the cycle's XDG
   artifact dir.
3. `lease_released` MUST be `true` (idempotent with prior
   `lease.released` events).

---

## 4. Receipt: `supersede-receipt.json`

### Placement

```
$XDG_DATA_HOME/sddk/projects/<project_id>/workspaces/<workspace_id>/cycles/<cycle_id>/
└── supersede-receipt.json
```

This is the **same placement** as `release-receipt.json` (per
`scripts/release.sh` step 8 convention), ensuring the receipt is
discoverable alongside other cycle artifacts.

### Shape

```json
{
  "schema_version": 1,
  "supersede_receipt_id": "sup-<uuid>",
  "cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-transition",
  "command_id": "cycle.supersede-<uuid>",
  "actor": {
    "kind": "human",
    "id": "user:rubentxu"
  },
  "timestamp": "2026-09-15T10:00:00Z",
  "reason": "ScopeInvalid",
  "reason_detail": "post-verify scope review determined the cycle was solving the wrong problem (see verify-report.md §3.2)",
  "evidence_refs": [
    {
      "kind": "verify-report",
      "path": "verify-report.md"
    }
  ],
  "successor_cycle_id": "p-52b95ef55999f9de/cycle-44-build-remediate-replacement",
  "lease_state_at_supersede": {
    "owner": "user:rubentxu",
    "fencing_token": 12345,
    "expires_at_ms": 1726390800000
  },
  "event_ids": [
    "evt-<uuid-of-cycle.supersede.requested>",
    "evt-<uuid-of-cycle.supersede.applied>",
    "evt-<uuid-of-lease.released>"
  ],
  "supersede_event_count": 3,
  "causation_chain": [
    "evt-<uuid-of-prior-cycle.transition>",
    "evt-<uuid-of-cycle.supersede.requested>",
    "evt-<uuid-of-cycle.supersede.applied>"
  ],
  "downstream_effects": {
    "lease_released": true,
    "cycle_status": "superseded",
    "vault_writes": false,
    "release_receipt_unchanged": true
  },
  "signature": {
    "algorithm": "ed25519",
    "key_id": "sddk-local-2026",
    "value": "<hex>"
  }
}
```

### Validation rules

1. `schema_version == 1`.
2. `event_ids` MUST contain at least the `cycle.supersede.requested` and
   `cycle.supersede.applied` events; may contain the `lease.released`
   event.
3. `supersede_event_count == event_ids.len()`.
4. `causation_chain` is a chain (each event's `causation_id` equals the
   previous).
5. `downstream_effects.vault_writes == false` (supersede does not write
   to vault).
6. `signature` MUST verify against the local key (fail-closed).

---

## 5. Ledger invariants preserved

Per Wave plan §Wave 4 ("recover preserves canonical digest and event
count"):

- Supersede adds **exactly 2 events** (`cycle.supersede.requested`,
  `cycle.supersede.applied`) when the lease is already held and
  released atomically.
- If the lease is released as part of supersede, add **1 more event**
  (`lease.released`). Total: 3.
- The cycle's existing events are unchanged.
- The supersede-receipt.json is a new artifact; digest of artifact dir
  changes but ledger digest does not.

### Test invariant

```rust
#[test]
fn supersede_preserves_ledger_digest() {
    // Capture digest before supersede
    let pre_digest = ledger.digest();

    // Run supersede (cycle is closed + lease released)
    run_supersede(...)?;

    // Capture digest after
    let post_digest = ledger.digest();

    // Existing events unchanged
    assert_eq!(pre_digest.events[0..N], post_digest.events[0..N]);

    // Exactly 2 new events appended
    assert_eq!(post_digest.events.len(), pre_digest.events.len() + 2);
}
```

---

## 6. Compatibility with existing readers

### Readers that do not know about supersede

- The cycle appears as `OPEN/Build` (or whichever phase) with no
  progression events after the last `phase.*` event.
- A reader that iterates events sees the cycle "stuck"; this is the
  intended signal for human triage.

### Readers that know about supersede

- The cycle status becomes `superseded` after
  `cycle.supersede.applied`.
- The successor cycle (if any) is bound by `successor_cycle_id`.

### No breaking changes

- Existing event schemas unchanged.
- Existing JSON output formats unchanged.
- The supersede-receipt.json is additive (new file in artifact dir).

---

## 7. Open questions

1. **Successor cycle binding at supersede time vs at start time**: is
   the successor created in the same transaction as the supersede, or
   deferred? **Decision (proposed)**: same transaction (atomicity
   requirement).
2. **What happens to `apply` receipts after supersede?**: they remain
   valid as historical evidence. The supersede marks the cycle as
   closed but does not invalidate prior receipts. **Decision
   (proposed)**: receipts are immutable; supersede adds the
   `supersede-receipt.json` alongside.
3. **What if `cycle.lock acquire` is broken (GAP-6)?**: supersede
   cannot run. **Decision (proposed)**: GAP-6 must be fixed before
   cycle-51 ships ADR-A.

---

## 8. References

- `research/cycle-supersede-replan/blueprints/cycle-supersede.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-001-cycle-supersede-vs-rebuild.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-007-recovery-action-contract.yml`
- `research/cycle-supersede-replan/evidence-cards/ec-css-010-ledger-event-count-invariant.yml`
- `crates/sddk-cli/src/cycle.rs:886` (`lease.released` event shape)
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 4
  (ledger invariant)
- `docs/adr/ADR-0073-secretary-authority.md` (AgentKind closed-set)
- `docs/adr/ADR-0047-durable-debt-remediation.md` §4 (artefact conservation)

---

## Changelog

- 2026-09-02 | promoted from research package draft to canonical spec | status: accepted | promoted by cycle-51 | valid_from=2026-09-02 | valid_to=∞
