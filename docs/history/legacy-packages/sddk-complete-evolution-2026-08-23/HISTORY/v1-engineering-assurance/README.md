# SDDK Engineering Assurance Evolution

**Baseline reviewed:** `Rubentxu/sddk-framework` `main`, SDDK 1.37.1, 2026-08-23.

This proposal evolves SDDK with a reusable **Engineering Assurance** bounded context without turning the kernel into a Rust-specific or systems-specific framework.

The core idea is:

> **Skills teach how to reason; capabilities express semantic work; profiles specialize that work for a technology; packs own bounded-context semantics; the kernel remains deterministic and domain-agnostic.**

## Decision summary

1. Introduce an optional `sddk-pack-engineering-assurance` **domain pack**.
2. Keep generic systems reasoning as a compact skill, not kernel logic.
3. Introduce technology profiles; Rust is the first reference profile, not a product constraint.
4. Reuse SDDK capability routing instead of hard-coding reviewers or models.
5. Require evidence-backed findings and deterministic verdict aggregation.
6. Integrate with existing SDD, UAT, Incident, Security and future packs through capabilities and event schemas.
7. Project assurance state into the Active Graph and Cockpit; never make those projections authoritative.
8. Dogfood the Rust profile on SDDK's own kernel architecture, especially focused ports, event invariants, concurrency boundaries and verification strategy.
9. Do not add zero-copy, `unsafe`, async restrictions or formal methods to SDDK merely because they are useful in systems programming; activate those checks only when the subject and profile make them relevant.

## Proposed documents

### ADRs
- `ADR-041-ENGINEERING-ASSURANCE-BOUNDED-CONTEXT.md`
- `ADR-042-REASONING-CAPABILITY-PROFILE-SEPARATION.md`
- `ADR-043-EVIDENCE-BACKED-ASSURANCE.md`

### Specifications
- `SPEC-042-ENGINEERING-ASSURANCE-PACK.md`
- `SPEC-043-ENGINEERING-PROFILE-PROTOCOL.md`
- `SPEC-044-ASSURANCE-EVIDENCE-CONTRACT.md`
- `SPEC-045-SYSTEMS-REVIEW-WORKFLOW.md`

### Roadmap / implementation
- `ROADMAP-ENGINEERING-ASSURANCE-INTEGRATION.md`
- `ENGINEERING-ASSURANCE-IMPLEMENTATION-PLAN.md`
- `ENGINEERING-ASSURANCE-FITNESS-FUNCTIONS.md`

### Proposed runtime skills
- `systems-reasoning/SKILL.md`
- `rust-systems-reasoning/SKILL.md`
- supporting local references

## Relationship with existing architecture

This evolution assumes and extends the current decisions:

- ADR-020: SDDK = **Software Development Decision Kernel**.
- ADR-025: capability-based routing.
- ADR-031: governed side effects.
- ADR-034: pack microkernel.
- ADR-038 / SPEC-038: invariant-driven adaptive SDD.
- SPEC-020: Capability Registry.
- SPEC-031: Governed Capabilities.
- SPEC-039: Workflow Pattern Algebra.

No existing kernel authority is replaced.

## Recommended numbering

The reviewed architecture index currently ends at **ADR-040** and **SPEC-041**, therefore this proposal starts at **ADR-041** and **SPEC-042**. Treat the numbering as provisional if concurrent work lands first.
