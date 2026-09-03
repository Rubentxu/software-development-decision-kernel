# STATUS — Evolutivo de Workflows Dinámicos

> **Design source:** `evolutivo-workflows-dinamicos-integracion-roadmap.md`
> **Current assessment baseline:** v1.70.0 / 2026-09-03
> **Canonical roadmap:** `sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
> **Canonical backlog:** `sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md`

## Current disposition

The dynamic-workflow evolution remains the principal technical design dossier for the next runtime evolution, but **its local horizon/cycle numbering is not the repository planning authority**.

Its useful ideas are now distributed across the canonical H0-H5 sequence:

| Theme | Assessment | Canonical destination |
|---|---|---|
| Workflow IR hardening | `PARTIAL` | H0 `DW-IR` |
| typed execution scope | `REMAINING` | `DW-IR-001` |
| transition/predicate contract | `REMAINING` | `DW-IR-002` |
| graph revision/hash/provenance | `PARTIAL` | H0/H2 |
| persisted WorkflowRun | `PARTIAL` | H2 `DW-RUNTIME` |
| generated DAG execution | `REMAINING` | H2 `DW-RUNTIME` |
| generated frontier / dynamic next | `PARTIAL` | H3 `DEC-PLANE` |
| reactive triggers/replan substrate | `PARTIAL` | H4 `RX-SECRETARY` |
| Secretary integration | `REMAINING/BLOCKED` | H4 after Decision Plane + authority |
| Map | `PARTIAL` | H5 `DW-OPERATORS` |
| Reduce / Join semantics | `REMAINING` | H5 `DW-OPERATORS` |
| real operator output semantics | `PARTIAL` | H0 contracts + H5 completion |
| Workflow Laboratory | `REMAINING` | H5 `LAB-WORKFLOW` |
| Planning Ledger | `REMAINING`, promoted | H1 `PLN-LEDGER` |

## Important sequencing refinements

The repository-wide canonical order is now:

```text
H0 IR/governance hardening
→ H1 Planning SSOT
→ H2 persisted generated workflow MVP
→ H3 unified Decision Plane
→ H4 human/reactive authority
→ H5 advanced operators + lab
→ H6 assurance/learning
```

This preserves the strongest rule from the original dossier: **durable dynamic execution before smarter supervision**.

## MVP boundary

The H2 vertical slice must remain intentionally narrow:

- Sequence;
- bounded Parallel;
- Conditional/deterministic gates;
- persisted WorkflowRun;
- durable execution state;
- deterministic replay/resume.

Do not pull full Map/Reduce/Join or Secretary into this vertical slice.

## Planning Ledger refinement

The Planning Ledger is promoted ahead of the runtime MVP because the repository has demonstrated a concrete governance failure mode: planning prose can remain stale while implementation advances. H1 should make semantic Work Items and dependencies machine-readable before further large evolutions multiply.

## Identity rule

References such as `cycle-58`, `cycle-59`, etc. in this dossier are historical/design-local labels only. New implementation planning must use canonical semantic IDs such as `DW-RUNTIME-001` and bind them to concrete cycles/runs separately.

## Execution rule

Continue to use `evolutivo-workflows-dinamicos-integracion-roadmap.md` for detailed architecture and acceptance ideas. Use `ROADMAP.md` and `BACKLOG.md` for priority, status, dependencies and official execution order.
