# STATUS — Human-Agent Collaboration Evolution Pack

> **Original baseline:** v1.50.0
> **Current assessment baseline:** v1.70.0 / 2026-09-03
> **Canonical roadmap:** `../sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
> **Crosswalk:** `../sddk-decision-kernel-architecture/02-roadmap/EVOLUTION-CROSSWALK.md`

## Current disposition

This pack is **retained as a design dossier**, not as an independent execution roadmap.

Its central ideas remain valuable, especially explicit authority, HITL, immutable human decisions, semantic resumability and rehydration. However, several prerequisites have shipped since the pack was written, and some proposed abstractions should now converge on the common Decision Plane rather than create Human-Agent-specific runtime state.

## Classification

| Capability | Status | Canonical destination |
|---|---|---|
| authority reconciliation / matrix | `REMAINING` | H0 `HX-AUTHORITY` |
| CurrentRunView | `REMAINING`, shape refined | H3 `DEC-PLANE-001` / `HX-CURRENT-RUN` |
| HumanDecision domain contracts | `REMAINING` | H4 `HX-DECISION-001` |
| HumanDecisionPort | `REMAINING` | H4 `HX-DECISION-002` |
| risk-sensitive HITL policy | `REMAINING` | H4 `HX-DECISION-003` |
| CLI/agent decision adapters | `REMAINING` | H4 `HX-DECISION-004` |
| cycle pause/resume substrate | `SHIPPED` | v1.70.0; do not reimplement |
| semantic cold-start resume | `REMAINING` | H4 `HX-RESUME` |
| rehydration plan | `REMAINING` | H4 `HX-RESUME-002` |
| collaboration observability | `REMAINING` | H4/H5, later EA |
| optimization/labs | `DEFERRED` | H5/H6 |

## Important refinement

`CurrentRunView` must be a projection of the shared persisted runtime/Decision Plane. It must not become a second Human-Agent-specific source of workflow truth.

Human decisions and future Secretary proposals must use one authority chain:

```text
policy
  → current run / decision context
  → request or bounded proposal
  → authorization / human decision
  → immutable receipt
  → workflow transition
```

## What in this pack is already preconditioned by newer releases

Compared with v1.50.0, the repository now has stronger substrate for this work:

- state-driven active-cycle context inference;
- declared-workflow-aware `cycle next`;
- actionable recovery hints;
- pause/resume with explicit paused lifecycle and receipts.

These reduce the amount of bespoke collaboration plumbing required, but they do not complete HumanDecision/HITL or semantic cold-start rehydration.

## Execution rule

Do not execute this pack slice-by-slice using its original sequence as a standalone program. Instead:

- authority work enters H0;
- CurrentRun requirements enter H3;
- HumanDecision/HITL/resume enter H4;
- metrics/optimization enter H5/H6.

The detailed files in this directory remain valid sources of acceptance criteria and design rationale where they do not conflict with the canonical roadmap/backlog.
