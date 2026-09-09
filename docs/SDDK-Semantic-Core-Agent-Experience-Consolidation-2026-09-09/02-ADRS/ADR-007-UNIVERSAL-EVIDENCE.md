# ADR-007 — Universal evidence model

**Status:** Proposed

## Decision

`EvidenceBundle`/`EvidenceArtifact` are the single core evidence model. Planning/UAT/assurance specializations attach typed relationships and metadata rather than defining parallel evidence taxonomies.

Planning-specific evidence attachment types are migrated to `EvidenceRef` plus relation (`supports`, `verifies`, `gates`, `observed_for`).

## Consequences

Redaction, content-addressing, provenance and verification are implemented once. Packs extend evidence metadata through namespaced schemas.
