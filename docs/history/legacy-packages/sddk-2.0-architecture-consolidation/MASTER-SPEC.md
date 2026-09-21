# SDDK 2.0 — Master Architecture Specification

This document is the compact integration view. Detailed normative wording lives in `specs/` and `adrs/`.

## Target product

SDDK becomes a **local-first, event-sourced, evidence-governed software-engineering control plane** for human + agent workflows.

## Authority stack

1. **Git** — source-code history and repository contents.
2. **SDDK deterministic kernel** — workflow transitions, policy, capability authorization and receipts.
3. **Event ledger** — durable operational history of accepted SDDK facts.
4. **Projections** — workflow state, graph, analytics, search and reports rebuilt from events.
5. **Agents/behaviors** — reasoning and proposal producers, never implicit authority.

## Core execution equation

```text
Intent/Command
 -> validation
 -> policy/approval
 -> capability
 -> postcondition verification
 -> evidence + receipt
 -> event ledger
 -> projections
 -> graph patterns
 -> observations/proposals
 -> back through deterministic application boundary
```

## Platform contracts

### Common Event Protocol
One versioned envelope carries causal metadata, actors, subjects, evidence refs, hashes, frame/fork IDs and domain payload.

### Evidence
Claim/Observation + Evidence + Oracle/Review + Verdict/Decision + Receipt.

### Packs
Small kernel. Packs declare `requires`, `integrates_with`, `provides`, `conflicts_with`. UAT is first extraction; Cognicode is the recommended first bridge proof.

### Reactive graph
Ledger-derived graph with bounded views, pattern subscriptions and relation behaviors. No direct governed effects.

### Forks
Durable shared-prefix forks support replay, structural/semantic diff, A/B evaluation and fail-closed promotion.

### Moldable Explorer
The same entities are projected into C4 architecture, verification, UAT, evidence, agent, release, graph and timeline views.

## First engineering move

Do **not** begin by implementing the graph UI. First establish inward ports, event semantics, evidence and pack authority. Otherwise SDDK risks adding a beautiful new projection on top of unresolved coupling.

## First bounded-context move

Extract UAT only after the application/event/evidence seams exist. This prevents a cosmetic file split that preserves the same architecture problems.

## First graph move

Read-only rebuildable projection -> queries/why -> deterministic patterns -> proposals -> relation behaviors -> staleness. Agentic behaviors come later.

## First trust move

Extend existing local validation/release supply-chain work into signed local gate receipts that can be independently verified for exact commit + gate set + evidence.
