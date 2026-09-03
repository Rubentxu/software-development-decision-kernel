# STATUS — Evolutivo de Workflows Dinámicos

> **Design source:** `evolutivo-workflows-dinamicos-integracion-roadmap.md`
> **Current assessment baseline:** v1.70.0 / 2026-09-03
> **Canonical roadmap:** `sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
> **Execution spine:** `sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`
> **Canonical backlog:** `sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md`

## Current disposition

The dynamic-workflow evolution remains the principal technical design dossier for Workflow IR, persisted generated execution, reactive behavior and advanced operators, but its local horizon/cycle numbering is not repository planning authority.

Its useful ideas map to the canonical line as follows:

| Theme | Assessment | Canonical destination |
|---|---|---|
| Workflow IR hardening | `PARTIAL` | H0 `DW-IR` |
| typed execution scope | `REMAINING` | `DW-IR-001` |
| transition/predicate contract | `REMAINING` | `DW-IR-002` |
| graph revision/hash/provenance | `PARTIAL` | H0/H2 |
| persisted WorkflowRun | `PARTIAL` | H2 `DW-RUNTIME` |
| generated bounded DAG execution | `REMAINING` | H2 `DW-RUNTIME` |
| generated frontier / dynamic next | `PARTIAL` | H3 `DEC-PLANE` |
| reactive trigger/replan substrate | `PARTIAL` | H5 `RX-SECRETARY` |
| Secretary integration | `REMAINING` | H5 after Decision Plane + AgentHost/context + human authority |
| Map | `PARTIAL` | H6 `DW-OPERATORS-002` |
| Reduce / Join semantics | `REMAINING` | H6 `DW-OPERATORS-003..004` |
| real operator output semantics | `PARTIAL` | H0 contracts + H6 `DW-OPERATORS-001` |
| Workflow Laboratory | `REMAINING` | H6 `LAB-WORKFLOW` |
| Planning Ledger | `REMAINING`, promoted | H1 `PLN-LEDGER` |

## Important sequencing refinement

The repository-wide path now continues all the way to GA:

```text
H0 deterministic foundations
→ H1 Planning SSOT
→ H2 persisted generated workflow MVP
→ H3 Decision Plane
→ H4 AgentHost + Context Compiler
→ H5 human/reactive authority
→ H6 advanced operators + lab
→ H7 assurance + UAT
→ H8 adaptive SDD
→ H9 graph + cockpit
→ H10 governed learning
→ H11 multi-pack proof
→ H12 production hardening + GA
```

This preserves the most important rule from the dossier: **durable dynamic execution before smarter supervision**.

## H2 MVP boundary

The generated-workflow MVP remains intentionally narrow:

- Sequence;
- Conditional/deterministic gates;
- bounded Parallel;
- persisted `WorkflowRun`;
- durable node/run state;
- deterministic replay/resume.

Do not pull full Map/Reduce/Join or Secretary into this vertical slice.

## Planning Ledger refinement

Planning Ledger remains H1 because SDDK has already demonstrated a real governance failure mode: prose can remain stale while code advances. The ledger must make semantic Work Items, dependencies and execution bindings machine-readable before another long evolution creates more planning drift.

## Identity rule

References such as `cycle-58`, `cycle-59`, etc. in the original dossier are historical/design-local labels only. New work uses semantic IDs such as `DW-RUNTIME-001` and binds them separately to concrete cycle/run IDs.

## Execution rule

Use `evolutivo-workflows-dinamicos-integracion-roadmap.md` for detailed technical architecture and acceptance ideas.

Use `EXECUTION-SPINE.yaml` to decide **what happens next**. The agent must not infer ordering from this dossier, and must not skip ahead to reactive/advanced-operator work while an earlier canonical Work Item is non-terminal.
