# Architecture Conformance Roadmap

This track is named **AC** and is embedded into the existing A2→A8 roadmap. It does not create a competing release sequence.

## Dependency map

```text
A2 context cut
  └─ AC0 ownership + dependency inventory                 [P0, A2]

A3 Knowledge/KMT
  ├─ AC1 typed ArchitecturalContract + ArchitectureClaim  [P1]
  ├─ AC2 architecture SemanticGraph overlay               [P1]
  └─ AC3 paradigm profiles + lens metadata                [P1]

A4 Alignment/Verify/DebVerify
  ├─ AC4 contract Verify + conformance delta              [P1]
  ├─ AC5 DebVerify architecture audit                     [P1]
  ├─ AC6 critical mutation probes                         [P1]
  └─ AC7 OO/FP/ADT/DSL lens assessments                   [P1]

A5 BASE production gate
  └─ AC8 SDDK self-audit + ArchitectureConformanceReceipt [P0 gate]

J5 reactive host loop
  └─ AC9 changed-units → affected-contracts → ContextDelta [P1]

A6 CogniCode
  └─ AC10 static graph/call/dependency evidence           [P1 enhanced]

A7 Chronos
  └─ AC11 runtime path/side-effect evidence               [P1 enhanced]

A8 full enhanced
  ├─ AC12 workbooks/control tower/time travel             [P2]
  ├─ AC13 counterfactual refactor planning                [P2]
  └─ AC14 proof-carrying changes + immune-system ratchets [P2]
```

## AC0 — A2 integration

No new semantics. Produce ownership map, dependency rules and state classes compatible with A2 context cut. Architecture conformance documents may guide ownership but SHALL NOT cause A3 types to be implemented early.

## AC1 — Typed contracts

Implement a minimal closed ADT for high-value constraints. Initial adapters may ingest contract declarations from repository-native specs/ADRs, but Decision Memory/revisioned objects become semantic authority after validation.

Exit: deterministic contract IDs, basis hashes and round-trip fixtures.

## AC2 — Graph overlay

Project contracts, units, decisions, specs, tests and relations into the one SemanticGraphProjection.

Exit: delete/rebuild produces equivalent architecture queries.

## AC3 — Paradigm intent

Implement `ParadigmProfile`/lens selection and graph relations without judging code yet.

Exit: profiles can differ by bounded context/software unit and survive rebuild.

## AC4 — Verify contracts

Given a change basis, compute affected contracts and execute the minimum deterministic probes. Provider-dependent probes may remain unknown.

Exit: `ArchitectureConformanceDelta` with evidence-backed statuses.

## AC5 — DebVerify architecture

Global challenge pass that looks for duplicate/shadow authorities, missing owners, bypasses, stale compatibility and contradictions even when no recent delta points at them.

Exit: no conflation with `verify --full`.

## AC6 — Mutation probes

For critical invariants, prove the guard catches an injected violation in sandbox.

Start with provider type leak, Alignment→Governance, workbook write and second canonical writer.

## AC7 — Paradigm lenses

Implement deterministic/heuristic lenses for OO, functional/pure-functional, ADT and DSL design. LLM evaluation is optional and must emit `INFERRED` assessment with provenance.

## AC8 — self-audit gate

Run SDDK against itself. Reproduce a representative subset of A0/A1 findings using native capabilities and produce a named `ARCHITECTURE-CONFORMANCE-RECEIPT`.

This is a Base production-readiness gate because the feature claims conformance over its own architecture.

## AC9..AC14

These are enhancement milestones, not blockers for Base. They activate reactivity, providers, workbooks, counterfactual planning and proof-carrying changes once the semantic core is stable.

## Priority rule

Do not let AC13/AC14 novelty delay AC1–AC8. The 80% value is typed contracts + graph + Verify/DebVerify + negative/mutation evidence + self-audit.
