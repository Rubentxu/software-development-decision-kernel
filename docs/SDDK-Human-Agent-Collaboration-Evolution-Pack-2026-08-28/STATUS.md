# STATUS — Human-Agent Collaboration Evolution Pack

> **Original baseline:** v1.50.0
> **Current assessment baseline:** v1.70.0 / 2026-09-03
> **Canonical roadmap:** `../sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
> **Execution spine:** `../sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`
> **Crosswalk:** `../sddk-decision-kernel-architecture/02-roadmap/EVOLUTION-CROSSWALK.md`

## Current disposition

This pack is retained as a **design dossier**, not as an independent execution roadmap.

Its central ideas remain valuable: explicit authority, HITL, immutable human decisions, semantic resumability and rehydration. However, several prerequisites shipped after v1.50.0 and the collaboration model must now converge on the common Decision Plane rather than create Human-Agent-specific workflow truth.

## Classification

| Capability | Status | Canonical destination |
|---|---|---|
| authority reconciliation / matrix | `REMAINING` | H0 `HX-AUTHORITY-001` |
| CurrentRunView | `REMAINING`, shape refined | H3 `DEC-PLANE-001` |
| HumanDecision contracts + port | `REMAINING` | H5 `HX-DECISION-001` |
| risk-sensitive HITL + adapters | `REMAINING` | H5 `HX-DECISION-002` |
| cycle pause/resume substrate | `SHIPPED` | v1.70.0; do not reimplement |
| semantic cold-start resume / rehydration | `REMAINING` | H5 `HX-RESUME-001` |
| collaboration observability | `REMAINING` | H5/H6/H7 |
| optimization/labs | `DEFERRED` | H6+ |

## Important refinement

`CurrentRunView` is a projection of the shared persisted runtime/Decision Plane. It must not become a second Human-Agent-specific source of workflow truth.

Human decisions and Secretary proposals share one authority chain:

```text
policy
  → current run / decision context
  → request or bounded proposal
  → authorization / human decision
  → immutable receipt
  → workflow transition
```

## Newer substrate already available

Compared with v1.50.0, SDDK now has:

- state-driven active-cycle context inference;
- declared-workflow-aware `cycle next`;
- actionable recovery hints;
- pause/resume with explicit paused lifecycle and receipts.

These reduce bespoke collaboration plumbing, but do not complete HITL or semantic rehydration.

## Execution rule

Do not execute this pack slice-by-slice using its original sequence.

Use the exact semantic order in `EXECUTION-SPINE.yaml`:

- H0: `HX-AUTHORITY-001`;
- H3: `DEC-PLANE-001..004` provides shared current-run/decision semantics;
- H4: AgentHost/context substrate required for robust agent collaboration;
- H5: `HX-DECISION-*`, `HX-RESUME-*`, `RX-SECRETARY-*`;
- H6/H7: collaboration observability, lab and assurance.

The detailed files in this directory remain sources of acceptance criteria and design rationale where they do not conflict with the canonical execution spine.
