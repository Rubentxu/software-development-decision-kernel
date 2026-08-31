# SDDK 2.0 — Architecture Consolidation Package

**Baseline repository:** `Rubentxu/software-development-decision-kernel`  
**Pinned commit:** `eb5117e6cd4366ceb205a5b2dde4195aa396d32f`  
**Pinned release:** `v1.9.1`  
**Package date:** 2026-08-11

This package consolidates the architectural ideas and improvement proposals discussed around SDDK, including the deep repository review and the ideas adapted from ActiveGraph. It is intentionally implementation-oriented: normative specifications, ADRs, migration strategy, executable roadmap, machine-readable backlog, schemas, examples, architecture rules, graph/view models and diagrams.

## Core thesis

SDDK should evolve into a **local-first, event-sourced software-engineering control plane** where:

- the **deterministic kernel** owns workflow, policy, authorization, capabilities, side effects and receipts;
- an **append-only event ledger** is the durable authority for operational history;
- a **reactive knowledge/evidence graph** is a rebuildable projection of the ledger, never a competing source of truth;
- agents and reactive behaviors **reason and propose**, but do not directly perform governed side effects;
- humans remain first-class authorities for approvals, UAT and acceptance;
- domain capabilities are delivered as **packs** around a deliberately small core;
- evidence, provenance, replay, forks and semantic diffs make agentic engineering inspectable and testable.

## Package map

| Directory | Purpose |
|---|---|
| `specs/` | Normative product and architecture specifications |
| `adrs/` | Proposed architecture decision records |
| `roadmap/` | Ordered implementation plan, migration, spikes, risks and deferred ideas |
| `schemas/` | Draft JSON Schemas for core contracts |
| `examples/` | Example packs, events, policies, views, receipts and workflows |
| `diagrams/` | Mermaid source diagrams |
| `data/` | Baseline inventory, event catalog, ontology, architecture rules and decision matrix |
| `templates/` | Templates for future ADRs, specs and deferred ideas |

## Recommended reading order

1. `specs/SPEC-000-vision-principles.md`
2. `specs/SPEC-001-target-architecture.md`
3. `roadmap/ROADMAP.md`
4. `roadmap/MIGRATION.md`
5. `adrs/ADR-001-event-ledger-authority.md` through `ADR-018-document-governance.md`
6. `specs/SPEC-002-common-event-protocol.md`
7. `specs/SPEC-004-reactive-knowledge-graph.md`
8. `specs/SPEC-008-uat-bounded-context.md`
9. `specs/SPEC-009-fork-replay-diff-promote.md`
10. `roadmap/BACKLOG.yaml`

## Scope discipline

The package recommends one consolidation cycle in which **new top-level product domains are frozen**. Work in that cycle should reduce coupling, establish stable extension seams and move existing capabilities behind packs/ports. New ideas remain allowed as `DEFERRED` items with explicit revisit triggers.

## Design maxim

> Agents reason. Graphs connect. Policies decide. Capabilities act. Humans govern. The ledger remembers.
