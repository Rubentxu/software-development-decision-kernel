# SPEC-016 — Local-First Storage and GraphStore Ports

**Status:** Proposed

## 1. Storage authorities

- Git repository: source/code artifacts chosen by the user;
- SDDK ledger: operational history;
- CAS: immutable content-addressed artifacts;
- SQLite indexes/projections: rebuildable local state;
- graph store: rebuildable world-model projection;
- knowledge files: durable authored knowledge where explicitly required.

## 2. Paths

Continue the zero-intrusion/XDG model. Operational databases, caches, forks and generated reports should live in SDDK user/project state directories unless explicitly exported.

## 3. GraphStore port

The graph API MUST be storage-neutral. Initial implementation SHOULD use a lightweight in-process Rust representation (e.g. petgraph) plus SQLite persistence/checkpoints if needed.

Alternative graph backends such as Kuzu/LadybugDB/Falkor/Neo4j-like systems are deferred adapters until concrete query/performance requirements justify them.

## 4. EventStore port

Likewise, event persistence SHOULD be behind an application port. SQLite is the default local-first adapter. A future remote/team store can implement the same contract without leaking database types into application services.

## 5. Conformance suites

Every EventStore/GraphStore adapter MUST pass a common conformance suite covering ordering, idempotency, rebuild, transaction/failure behavior and compatibility.
