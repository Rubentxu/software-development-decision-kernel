# ADR-006 — Durable ContextBasis/Capsule/Delta and session bindings

**Status:** Proposed

## Context

ContextCompiler, ContextBridge and session binding semantics exist, but product paths still depend partly on in-memory/default-null persistence and prompt-driven recovery.

## Decision

Make ContextBasis/Capsule/Delta and SessionBinding durable through the canonical storage/CAS boundary. Add one bootstrap application service that converges adoption, resolves active work, compiles context and returns a hypermedia representation.

Full transcript stays host-owned.

## Consequences

- restart/reconnect no longer implies re-reading the repository;
- staleness becomes enforceable;
- context delivery can use progressive disclosure;
- storage migrations and retention policy become explicit concerns.

## Rejected

- storing transcripts in SDDK;
- using Engram/files as canonical lifecycle state;
- a new vector database as mandatory context store.
