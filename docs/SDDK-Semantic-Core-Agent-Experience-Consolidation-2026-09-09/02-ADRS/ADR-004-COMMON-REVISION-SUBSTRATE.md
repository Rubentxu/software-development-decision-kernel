# ADR-004 — Common content-addressed revision substrate

**Status:** Proposed

## Context

PlanRevision, fork/replay and Decision Memory all need parentage, content identity, refs and diff semantics.

## Decision

Introduce a generic immutable substrate:

```text
ObjectId = sha256(canonical object envelope)
Revision<T> = { oid, parents[], payload_ref, provenance, metadata }
Ref = { namespace, name, expected_old?, new_oid }
```

Refs use compare-and-swap updates. Reflog is an append-only fact/projection depending on implementation. Structured objects use canonical serialization with schema version.

Domain layers add semantic diff/merge rules; the substrate remains domain-agnostic.

## Non-goal

Do not reimplement Git transport, packfiles, index/staging area or filesystem checkout semantics.
