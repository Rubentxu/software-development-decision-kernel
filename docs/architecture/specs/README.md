# Architecture Specifications Index

This directory contains repository-native architecture specifications.

## Normative succession

```text
2026-09-09 semantic-core + agent-experience baseline
  -> C0..C7 conformance closeout
  -> C7 PASS certified at 0c2ca56 / SDDK 1.169.19
  -> 2026-09-10 context-first roadmap
  -> 2026-09-14 production-readiness / providers / agentic layers
  -> 2026-09-14 architecture-conformance + graph-intelligence evolution
  -> per-profile readiness receipts
```

The C7 receipt is:

`docs/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/08-BASELINE-CONFORMANCE-09-09/09-09-CONFORMANCE-RECEIPT.md`

Current convergence roadmap:

`docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`

Architecture Conformance evolution:

`docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/`

## Status rule

Document presence is not implementation evidence. A spec remains `proposed` until the receipt/acceptance process named by its roadmap closes. `PASS` claims belong to receipts at exact commits.

## Baseline arch-spec-001..018

The 09/09 baseline is conformance-certified by C7 at `0c2ca56` with every SPEC-001..018 row `PASS` or legitimate `PASS_WITH_COMPAT` and UAT-01..22 green. File-frontmatter promotion, where desired, follows ADR-0001 and is distinct from the receipt itself.

## Production convergence / providers

- `arch-spec-019-production-readiness-convergence.md`
- `arch-spec-020-storage-schema-ownership.md`
- `arch-spec-021-intelligence-provider-boundary.md`

## Agentic Workspace / host integration

- `arch-spec-022-agentic-workspace-boundary.md`
- `arch-spec-023-host-capability-negotiation.md`
- `arch-spec-024-agentic-session-binding.md`
- `arch-spec-025-reactive-host-event-bridge.md`
- `arch-spec-026-context-delta-delivery.md`
- `arch-spec-027-structured-agent-work.md`
- `arch-spec-028-permission-interruption-bridge.md`
- `arch-spec-029-workspace-locality.md`
- `arch-spec-030-agentic-integration-api-sdk.md`
- `arch-spec-031-jcode-anti-corruption-layer.md`

## Architecture Conformance + Graph Intelligence

- `arch-spec-032-architectural-contracts.md`
- `arch-spec-033-architecture-semantic-graph-overlay.md`
- `arch-spec-034-architecture-conformance-verification.md`
- `arch-spec-035-paradigm-lens-system.md`
- `arch-spec-036-adt-and-typed-dsl-modeling.md`
- `arch-spec-037-reactive-conformance-loop.md`
- `arch-spec-038-architecture-mutation-and-counterfactual-probes.md`
- `arch-spec-039-proof-carrying-changes.md`
- `arch-spec-040-architecture-conformance-workbooks.md`
- `arch-spec-041-sddk-architecture-self-audit.md`

### Activation mapping

| Specs | Roadmap |
|---|---|
| 032–033 | AC1/AC2 in A3 |
| 035–036 | AC3/AC7 across A3/A4 |
| 034 | AC4/AC5 in A4 |
| 038 | AC6 Base critical probes; AC13 post-Base counterfactual |
| 037 | A4 foundation + J5 reactive integration |
| 039 | J5/A8 proof-carrying changes |
| 040 | Base read views + A8 richer workbooks/time travel |
| 041 | AC8 / A5 Base self-audit gate |

## Architectural guardrails

Across specs 019..041:

- one Canonical Event Log authority;
- one SemanticGraphProjection, rebuildable and non-authoritative with respect to source facts;
- one AuthorityEngine decision path for governed effects;
- Alignment is advisory;
- missing evidence/provider never becomes PASS;
- provider/host types terminate at adapters;
- Workbooks are projections;
- programming paradigms are scoped lenses, not global policy;
- DSL knowledge is not capability/authority;
- advanced counterfactual/proof-carrying features may not delay the AC1..AC8 Base value path.
