# SPEC-CONF-002 — Canonical Event Log Cutover

## Baseline contract

Closes SPEC-001, ADR-001, M1 and M9 requirements for one canonical fact/event write path.

## Problem

The audited implementation has a new canonical event path (`events_v1`) while legacy `ledger_events` still coexists. Coexistence is acceptable during strangler migration, not as the final M9 state when both remain production-reachable write authorities.

## Required end-state

```text
command/use case
    -> CanonicalEventLog append port
        -> one authoritative event persistence path

legacy ledger readers
    -> compatibility decoder/projection
        -> canonical facts/events
```

There MUST NOT be two independently writable canonical event histories.

## Requirements

- `CanonicalEventLog` owns append semantics and replay ordering.
- one storage adapter owns authoritative append persistence;
- legacy `ledger_events` writes are removed or hard-disabled;
- if old rows must remain readable, they are imported/replayed through an idempotent migration or exposed via a read-only decoder;
- no cross-table atomicity assumption is allowed after cutover;
- all projections rebuild from the canonical stream alone;
- migrated repositories and fresh repositories produce equivalent canonical reads;
- migration is dry-runnable and restart-safe;
- export/recovery fixture exists before any destructive table removal.

## Invariants

- same canonical facts -> same deterministic projection state;
- mirror divergence cannot alter canonical reads;
- legacy event storage cannot accept new domain facts after cutover;
- event ids/order used in Explanation/WHY remain stable or have an explicit mapping receipt.

## Acceptance

1. architecture scan finds exactly one event append authority;
2. attempting a legacy write fails at compile-time or runtime boundary;
3. clean repo and migrated repo pass identical event/projection fixtures;
4. delete projection storage + rebuild succeeds from canonical events;
5. failure injection during migration is idempotently recoverable.
