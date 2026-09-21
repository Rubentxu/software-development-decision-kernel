# SPEC-003 — Event Ledger and Projections

**Status:** Proposed

## 1. Authority model

The durable event ledger is the source of truth for SDDK operational history. Graphs, status tables, search indexes, dashboards and reports are projections.

Git remains authoritative for source-code history; the SDDK ledger records how SDDK observed and acted on Git states.

## 2. Storage

Initial implementation SHOULD use SQLite under the existing local-first/XDG state model.

Required properties:

- append-only event table;
- transactionally allocated stream sequence;
- WAL mode where appropriate;
- content hash and optional previous-event hash chain per stream;
- projection checkpoint table;
- snapshot support as an optimization, never the historical authority;
- migration metadata.

## 3. Projection contract

A projection MUST be deterministic for a fixed event stream and projection version.

```rust
trait Projection<E> {
    type State;
    fn version(&self) -> ProjectionVersion;
    fn apply(&mut self, event: &E) -> Result<(), ProjectionError>;
    fn checkpoint(&self) -> Checkpoint;
}
```

## 4. Rebuild

Every projection MUST support a rebuild from event sequence 1 or from a verified snapshot + tail. `sddk dev projection rebuild <name>` SHOULD exist before a projection is considered production-ready.

## 5. Projection examples

- workflow/cycle state;
- knowledge graph;
- UAT status;
- release assurance;
- capability audit;
- search/index;
- metrics/analytics;
- HTML/JSON reports.

## 6. No hidden mutable authority

A projection MUST NOT contain business facts that cannot be reproduced from events or explicit external source snapshots. If a projection enriches from an external source, the acquisition result must be captured as evidence/artifact/event first.

## 7. Compaction

Compaction MAY archive old event segments after creating verified snapshots, but MUST not silently destroy lineage required by active receipts, forks, releases or evidence chains.
