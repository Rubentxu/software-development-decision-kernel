# ADR-041 — Engineering Assurance as a composable bounded context

**Status:** Proposed

## Context

SDDK is a Software Development Decision Kernel, not a Rust framework and not a Specification-Driven Development kernel. Its extension model already expects SDD, UAT, Incident, Security and other domains to compose over generic runtime contracts.

Engineering workflows repeatedly need reusable reasoning and verification across domains:

- architecture boundaries and dependency direction;
- state and domain invariants;
- control-flow and resource-liveness reasoning;
- concurrency and async-boundary review;
- performance budgets and hot-path analysis;
- representation, parsing and memory-safety review where applicable;
- verification strategy selection;
- evidence-backed findings and remediation obligations.

Today these concerns are spread across SDD verification lenses, debt analysis, language skills and specialist agents. Encoding them directly into the kernel would violate the microkernel direction. Encoding them only as skills would make them difficult to route, measure, govern, replay and project.

## Decision

Create **Engineering Assurance** as an optional `domain` pack, provisionally named:

```text
sddk-pack-engineering-assurance
```

The bounded context owns:

- engineering assessments;
- findings;
- assurance obligations/invariants;
- evidence links;
- technology profile selection;
- verification plans;
- deterministic verdict aggregation;
- assurance-specific workflow templates;
- assurance views/projections.

The pack exposes semantic capabilities such as:

```text
engineering.assess
architecture.review
systems.review
performance.review
concurrency.review
representation.review
verification.plan
```

The pack MUST NOT own unrestricted code mutation. Remediation uses existing governed capabilities such as patch/file/git capabilities through normal Proposal → Policy → Grant → Execute → Verify → Receipt flow.

## Kernel boundary

The kernel MUST NOT gain:

- `Rust*`, `Go*`, `Cpp*` domain types;
- Netstack-specific concepts;
- assurance-specific scheduling semantics;
- assurance-specific side-effect bypasses;
- assumptions that every project is systems software.

The kernel only sees generic pack, workflow, capability, event, evidence and projection contracts.

## Why a pack instead of only a skill

A skill is appropriate for reasoning instructions. Engineering Assurance additionally has durable semantics:

```text
Assessment
Finding
Obligation
Evidence
Profile
Verdict
```

and reusable workflows/events/views. That meets the threshold for a bounded context under ADR-034.

## Why not a core pack

Engineering Assurance is valuable but not required for every SDDK workflow. It is therefore a `domain` pack, composable by SDD/UAT/Incident/Security rather than kernel authority.

## Consequences

### Positive

- Cross-pack reuse of architecture/systems reviews.
- Language/runtime knowledge can evolve independently.
- Findings become measurable and traceable instead of prose-only.
- Adaptive workflows can request only the assurance capabilities justified by risk.
- SDDK can dogfood the same contracts used by external projects.

### Negative

- Introduces a new bounded context and schemas.
- Existing SDD verify/debt outputs need adapters to avoid duplicate concepts.
- Provider manifests and profile selection require governance to prevent skill explosion.

## Rejected alternatives

### Put all rules in `rust-patterns`
Rejected: conflates generic engineering reasoning with one implementation language and provides no durable capability/evidence contract.

### Add engineering checks directly to kernel
Rejected: violates SDD-agnostic/domain-agnostic microkernel direction.

### Create separate packs per language
Rejected initially: language is a specialization profile, not a bounded context. Split only if a future language domain genuinely develops independent workflows/events/authority.
