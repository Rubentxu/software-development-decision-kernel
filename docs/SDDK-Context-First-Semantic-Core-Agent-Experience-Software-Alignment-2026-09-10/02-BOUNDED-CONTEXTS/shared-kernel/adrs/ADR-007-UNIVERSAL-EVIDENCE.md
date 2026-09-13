# ADR-007 — Universal evidence model

**Status:** Proposed

## Decision

`EvidenceBundle`/`EvidenceArtifact` are the single core evidence model. Planning/UAT/assurance specializations attach typed relationships and metadata rather than defining parallel evidence taxonomies.

Planning-specific evidence attachment types are migrated to `EvidenceRef` plus relation (`supports`, `verifies`, `gates`, `observed_for`).

## Consequences

Redaction, content-addressing, provenance and verification are implemented once. Packs extend evidence metadata through namespaced schemas.

## 2026-09-10 amendment

CogniCode/Chronos/provider results enter SDDK through the universal Evidence model. Provider-native stores may retain large graph/trace data; SDDK stores normalized facts/digests/refs/basis needed for provenance and decision support.
