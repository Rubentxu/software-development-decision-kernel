# ADR-041 — Engineering Assurance as a composable bounded context

**Status:** Proposed

## Context

Architecture, systems reasoning, performance and verification recur across SDD, UAT, Incident, Security and SDDK dogfooding.

Keeping them only as prompt prose makes results difficult to route, measure, replay and reuse. Putting them in kernel would violate the microkernel and technology-neutral direction.

## Decision

Create optional domain pack:

```text
sddk-pack-engineering-assurance
```

It owns assessments, assurance obligations, normalized findings, evidence links, technology profiles, deterministic assurance verdicts and assurance views/workflow templates.

It exposes semantic capabilities:

```text
engineering.assess
architecture.review
systems.review
performance.review
concurrency.review
representation.review
verification.plan
engineering.profile.resolve
```

## Boundary

Kernel MUST NOT gain Rust/Go/C++ types or assurance-specific scheduling logic.

The pack MUST NOT bypass governed effects.

Engineering Assurance v1 is read-only with respect to project source. Remediation is a separate governed capability requested by the consuming workflow.

## Consequences

- reusable engineering quality across packs;
- evidence-backed reviews instead of prose-only judgments;
- language knowledge evolves independently;
- SDD remains one consumer, not owner of engineering assurance.
