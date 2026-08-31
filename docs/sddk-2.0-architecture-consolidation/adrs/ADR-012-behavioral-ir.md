# ADR-012 — Behavioral IR and Content-Bound Agent Identity

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Permissions, agent Markdown, prompts and manifests can drift and an agent ID alone does not identify changed behavior.

## Decision

Introduce machine-readable Behavioral IR and bind execution identity to prompt/skill/policy/model hashes.

## Consequences

### Positive

- Less semantic drift.
- Better auditability and reproducibility.

### Trade-offs / risks

- Migration requires generators/validators and careful authoring UX.

## Implementation notes

Begin as sidecar metadata for selected agents; generate permission inventory and receipts before making it the only authoring source.

## Revisit trigger

Revisit canonical authoring format after two release cycles of sidecar use.
