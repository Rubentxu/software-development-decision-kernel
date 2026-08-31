# SPEC-035 — Software Supply Chain, SBOM & Artifact Lifecycle

**Status:** Proposed

## Goal
Trace what was built, from which inputs, with which dependency inventory/attestations, how it moved through environments and which evidence authorized release.

## Core entities
- `SourceRevision`
- `BuildExecution`
- `ArtifactIdentity`
- `SbomDocument`
- `Attestation`
- `Signature`
- `Promotion`
- `Deployment`
- `Release`
- `VulnerabilityFinding`
- `PolicyEvaluation`

## Canonical lineage

```text
Repository/Commit
 → BuildExecution
 → Artifact(hash/digest)
 → SBOM
 → Attestation/Signature
 → Promotion
 → Deployment
 → UAT evidence
 → ReleaseDecision
 → Release
```

## Artifact identity
Prefer immutable identities/digests over mutable tags.

```yaml
artifact:
  kind: oci-image
  locator: registry/org/app
  digest: sha256:...
  version: 1.4.2
```

## SBOM
Store format/type, digest and artifact reference; preserve the original CycloneDX/SPDX document externally or in CAS/artifact storage. SDDK graph links components/dependencies to artifact/release when useful.

## Provenance
Capture build system, source revision, builder identity, recipe/workflow reference, timestamps and attestation refs. Support SLSA/in-toto style attestations through adapters without making one vendor mandatory.

## Lifecycle events
- `artifact.built`
- `artifact.sbom.recorded`
- `artifact.attestation.recorded`
- `artifact.signature.verified`
- `artifact.promoted`
- `artifact.deployed`
- `artifact.quarantined`
- `artifact.deprecated`
- `artifact.retired`

## Policies
Examples:
- block release without SBOM;
- require attestation from approved builder;
- deny critical vulnerability without waiver;
- require UAT Signoff bound to same artifact digest;
- forbid promoting an artifact whose source revision differs from approved release candidate.

## Cockpit lenses
- artifact lineage;
- component/vulnerability blast radius;
- environment promotion history;
- release evidence chain.
