# Workflow Pack Catalog

## Kernel principle
A pack supplies domain-specific orchestration/invariants over Workflow Runtime v2. It may provide canonical templates, adaptive templates and document/view projections.

## Initial proof set

### `sddk-sdd`
Software change development. Provides both `sdd.reference` and experimental `sdd.adaptive`.

### `sddk-uat`
Human + automated acceptance validation with evidence and sign-off.

### `sddk-incident`
Triage, diagnosis, containment, remediation and post-incident verification.

If these run without kernel special cases, including dynamic graph expansion for SDD/incident discovery, the abstraction is healthy.

## Workflow classes

### Deterministic
Known control logic; minimal cognition. Examples: release bookkeeping, SBOM generation, artifact promotion.

### Adaptive
Stable invariants with dynamic depth/decomposition. Examples: SDD, bugfix, incident, migration, architecture.

### Exploratory
Plan/decomposition emerges from discovery. Examples: research, unfamiliar legacy analysis.

## Next packs
Keep prior candidates: bugfix, security, release, dependency-upgrade, db-migration, architecture, research.

## Composition
Use SubWorkflow/capabilities rather than importing internal implementation. Packs may expose templates and invariants, while the Workflow Compiler selects pattern compositions from the common algebra.
