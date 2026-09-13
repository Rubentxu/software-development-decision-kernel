# ADR-001 — One canonical fact log

**Status:** Proposed

## Context

SDDK currently contains more than one event/ledger representation and even consistency checks between them. Long-term dual authority creates drift and forces every projection to choose a source.

## Decision

SDDK SHALL expose one logical `CanonicalEventLog` port as the sole ordered authority for domain facts. Physical migration mirrors are compatibility mechanisms only.

`EventStore`, legacy ledger interfaces and concrete SQLite tables may coexist temporarily, but one is designated canonical in each migration phase and writes MUST flow through one application service.

## Consequences

- projections consume one logical stream contract;
- cross-ledger consistency becomes a migration check, not permanent architecture;
- replay/rebuild semantics become simpler;
- append schemas require explicit compatibility/version policy.

## Rejected

Permanent dual-write with reconciliation: too much operational ambiguity for a local-first deterministic kernel.
