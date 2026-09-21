---
id: arch-spec-020-storage-schema-ownership
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/01-GAP-AND-DRIFT-REGISTER.md#pr-gap-005
supersedes_history: false
---

# arch-spec-020 — Storage Schema Ownership

## Problem

Multiple storage components may legitimately expose different persistence ports over the same SQLite database. They MUST NOT independently own the definition, migration order or bootstrap invariants of shared tables.

The production-readiness audit found that shared-schema assumptions around project/bootstrap state had diverged between storage paths. This class of error can pass isolated store tests while failing only when one component consumes a database initialized by another.

## Decision

SDDK SHALL have exactly one authoritative schema/migration owner per physical database.

The concrete owner may be the existing storage migration layer or a successor selected during implementation. The invariant is ownership, not a mandated type name.

```text
       Schema/Migration Owner
               |
      versioned migrations
               |
       prepared connection/db
          /            \
 Event Store          Other Stores
```

## Requirements

### SSO-001 — One migration sequence

All production openings of the same database SHALL use the same ordered migration set and schema version semantics.

A store may validate required tables/columns but SHALL NOT maintain a competing migration history for shared schema.

### SSO-002 — Bootstrap invariants are schema-level contracts

Rows/foreign-key parents required by more than one store SHALL have one documented creation/bootstrap contract.

A component SHALL NOT rely on a partial row shape that another component cannot safely consume.

### SSO-003 — Store constructors do not create hidden competing authority

A store constructor/open method may request an initialized connection/database from the owner. If compatibility requires a convenience `open(path)` API, that facade SHALL delegate to the same schema owner rather than reimplement migrations/bootstrap.

### SSO-004 — Migrations remain deterministic and recoverable

Migration behavior SHALL be:

- versioned;
- ordered;
- idempotent where replay is intentionally supported;
- transactional where SQLite and migration semantics permit;
- explicit on unsupported/future schema versions;
- covered by crash/reopen or interrupted-migration fixtures for critical boundaries.

### SSO-005 — Foreign keys are tested across store boundaries

Tests SHALL exercise databases initialized through the real production owner and then consumed through each relevant store. Isolated in-memory DDL copied into a test is insufficient as the only proof.

### SSO-006 — Canonical Event Log ownership remains separate from schema ownership

One schema owner does not make every store a canonical event authority.

`events_v1`/CanonicalEventLog append semantics continue to be owned by the event-store port/adapter. Schema ownership only governs physical database structure and migration/bootstrap consistency.

### SSO-007 — No domain leakage

SQLite schema/migration concepts remain storage/infrastructure concerns. Domain/application code consumes ports/receipts and does not depend on SQLite table shapes.

## Required fixtures

1. **fresh database:** schema owner initializes a new DB; event + approval + other core stores operate without manual bootstrap;
2. **migrated database:** oldest supported schema migrates through the exact production chain;
3. **reopen:** initialized DB reopens without destructive re-bootstrap;
4. **foreign-key integration:** event/audit append referencing project/run/actor prerequisites succeeds or fails with a semantic error, never an accidental partial-schema mismatch;
5. **future schema:** unsupported newer schema produces explicit incompatibility;
6. **migration interruption:** critical migration boundary has a defined recovery outcome;
7. **projection rebuild:** rebuilding projections does not rerun or fork schema authority.

## Fitness rule

CI SHOULD include a static/structural check that rejects new independent migration runners/DDL authorities for the canonical SQLite database outside the approved storage schema owner or its explicit test fixtures.

## Acceptance

This spec is `PASS` when:

- a single migration/bootstrap authority can be named in code;
- every production store opening the canonical DB delegates to it or consumes its initialized connection contract;
- no shared table has two independent production DDL/migration definitions;
- the required fixtures pass on fresh and migrated repositories;
- the ownership choice is reflected in architecture/responsibility documentation.
