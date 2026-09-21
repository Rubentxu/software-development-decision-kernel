# ADR-011 — GraphStore Port with Lightweight First Adapter

**Status:** Proposed  
**Date:** 2026-08-11

## Context

It is too early to lock SDDK to a production graph database.

## Decision

Define GraphStore as a port; start with local Rust in-process graph/projection plus SQLite checkpoints/indexes.

## Consequences

### Positive

- Minimal dependencies and fast local development.
- Future backends remain possible.

### Trade-offs / risks

- Advanced graph query performance may be limited initially.

## Implementation notes

Build conformance tests before any second backend. Evaluate Kuzu/LadybugDB/etc. only with measured requirements.

## Revisit trigger

Revisit when representative repositories exceed agreed latency/memory targets.
