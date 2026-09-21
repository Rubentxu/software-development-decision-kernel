# ADR-015 — Release Channels: stable/candidate/edge/dev

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Rapid releases benefit from explicit risk/validation channels.

## Decision

Add stable, candidate, edge and dev channels, leveraging side-by-side bundles/version pins.

## Consequences

### Positive

- Safer experimentation.
- Clear promotion semantics.

### Trade-offs / risks

- More release metadata and support matrix.

## Implementation notes

Start with metadata/channel pointers, not separate build systems.

## Revisit trigger

Revisit channel count if users consistently use fewer than three.
