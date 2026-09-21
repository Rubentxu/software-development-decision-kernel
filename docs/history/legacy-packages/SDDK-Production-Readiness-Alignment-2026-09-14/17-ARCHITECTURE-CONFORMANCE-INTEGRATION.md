# Architecture Conformance Integration Amendment

This document links the Production Readiness program to:

`../SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/`

It does not create another roadmap authority. The AC milestones are sub-milestones of A2→A8.

## Mapping

```text
A2  → AC0 ownership/dependency inventory only
A3  → AC1 contracts + AC2 graph overlay + AC3 paradigm intent
A4  → AC4 Verify + AC5 DebVerify + AC6 mutation + AC7 paradigm/ADT/DSL lenses
A5  → AC8 SDDK self-audit / ArchitectureConformanceReceipt
J5  → AC9 reactive changed-unit contract loop
A6  → AC10 CogniCode static evidence
A7  → AC11 Chronos runtime evidence
A8  → AC12 workbooks/time travel + AC13 counterfactual + AC14 proof-carrying/ratchets
```

## Priority

AC1..AC8 are the high-value path. AC13/AC14 are explicitly P2 and SHALL NOT delay Base readiness.

## Production gate amendment

`BASE_PRODUCTION_READY` requires an `ARCHITECTURE-CONFORMANCE-RECEIPT` once arch-spec-041 is implemented. The receipt must work with providers absent and must dogfood SDDK itself at a named commit.

## ActiveGraph inspiration

The design borrows event-sourced graph, small-core/layered vocabulary, behavior-local context and optional integration ideas from ActiveGraph, but SDDK keeps explicit Workflow/Run orchestration and its own Authority/Decision model.

Reference basis: `yoheinakajima/activegraph-packs@6639a5385518ad49f74813373c85cf96eff9adc0`.
