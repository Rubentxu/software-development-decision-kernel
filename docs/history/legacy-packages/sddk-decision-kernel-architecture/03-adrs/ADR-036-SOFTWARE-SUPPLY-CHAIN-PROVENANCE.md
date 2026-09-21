# ADR-036-SOFTWARE-SUPPLY-CHAIN-PROVENANCE — Model artifact origin, SBOM and lifecycle as first-class provenance

**Status:** Accepted

## Context
Software-engineering decisions are incomplete if SDDK can explain how code was produced but not what binary/image/package was built, what it contains, where it came from and where it was promoted/deployed.

## Decision
Represent source identity, builds, artifacts, SBOMs, attestations, promotions, deployments and release decisions as typed entities/events/evidence relationships. Keep binary payloads in artifact stores; SDDK stores identity, hashes, metadata and provenance references.

## Authority
External build/signing systems may remain authoritative for the artifact/attestation itself. SDDK records verifiable references and lifecycle events; it does not silently re-author them.

## Consequences
- release workflows can gate on provenance/SBOM/policy;
- Cockpit can expose artifact lineage;
- UAT evidence can bind to the exact build under test;
- graph queries can answer origin and blast-radius questions.
