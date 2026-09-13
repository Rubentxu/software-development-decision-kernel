# ADR-002 — Facts, Objects, Projections and Ephemeral state

**Status:** Proposed

## Decision

Every SDDK data model/store is classified as exactly one of: Fact, Object, Projection, Ephemeral.

No feature may introduce a persistent store without documenting its class and authority.

## Rationale

This creates a simple invariant stronger than naming conventions and prevents read models, context caches or search indexes from becoming accidental sources of truth.

## Enforcement

Architecture lint scans a machine-readable ownership registry and fails unknown/duplicate authorities after the migration ratchet is enabled.
