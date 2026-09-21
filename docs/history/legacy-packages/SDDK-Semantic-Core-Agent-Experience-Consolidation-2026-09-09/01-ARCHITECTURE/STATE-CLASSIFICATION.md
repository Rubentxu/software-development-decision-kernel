# State classification standard

## Purpose

Prevent future ambiguity about source of truth by requiring every new model/store to declare a state class.

## Classes

### Fact

An immutable occurrence that changes what SDDK knows about the world.

Examples: work item activated, capability executed, approval granted, decision accepted, run completed.

Rules:
- append-only;
- globally or stream-order identifiable;
- canonical serialization/versioning;
- explicit actor and provenance where relevant;
- cannot be reconstructed from projections.

### Object

Immutable content-addressed material.

Examples: evidence bytes, large result payload, MemoryTree, normalized plan payload.

Rules:
- immutable bytes;
- digest identity;
- schema envelope for structured objects;
- may be garbage-collected only by explicit retention/reachability policy.

### Projection

Derived state optimized for queries/read UX.

Examples: GraphState, RunStateView, metrics, FTS index, backlog summary.

Rules:
- rebuildable;
- versioned projector;
- checkpoint optional;
- never accepted as sole proof of an irreversible action.

### Ephemeral

Task/session material that can be regenerated or discarded.

Examples: ContextCapsule, provider routing candidate, prompt assembly, temporary ranking.

Rules:
- no source-of-truth claims;
- persistence is cache/diagnostic only;
- staleness is expected and explicit.

## Required metadata for architecture review

Any new storage-backed feature documents:

```text
state_class:
authority:
source_of_truth:
lifecycle:
rebuild_from:
retention:
owner_module:
extension_boundary:
```
