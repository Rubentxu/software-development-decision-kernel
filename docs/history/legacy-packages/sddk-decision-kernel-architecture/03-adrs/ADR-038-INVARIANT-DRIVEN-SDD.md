# ADR-038 — Make SDD Invariant-Driven Rather Than Phase-Driven

**Status:** Proposed

## Context
The current full SDD path preserves valuable quality guarantees but materializes many concerns as separate sequential agent phases: explore, propose, spec, design, tasks, apply, verify, debt verification, archive, release. Stronger models and structured handoff/state mechanisms make some boundaries redundant and increase handoff entropy.

## Decision
Introduce experimental `sdd-adaptive` where SDD quality is defined by invariants, not a mandatory sequence of named phases.

The semantic authority for a change is `ChangeContract`, containing intent, scope, requirements, acceptance, constraints, decisions, risks, verification obligations, evidence and work decomposition.

High-level stages:

```text
SHAPE → BUILD ⇄ CONVERGE → INTEGRATE
```

`explore`, `proposal`, `spec`, `design`, `tasks` remain capabilities and document projections invoked when needed.

A-full remains the reference workflow until controlled evaluation shows adaptive is at least as safe/effective.

## Consequences
- Fewer mandatory handoffs.
- Same documentation can still be generated for humans.
- Workflow depth adapts to uncertainty/risk.
- Requires robust ChangeContract schemas and convergence checks.
