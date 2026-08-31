# Priority Taxonomy

When to remediate a debt finding, relative to other work in the cycle/release pipeline.

## P0
**Definition**: Drop everything. Fix now.
**Scheduling**: Blocks release; must be resolved before any release tag.
**Scope**: A single cycle; may extend across multiple phases if remediation is large.

## P1
**Definition**: Fix in the current or next cycle.
**Scheduling**: Cycle-level commitment.
**Scope**: A single cycle in most cases.

## P2
**Definition**: Fix in a later cycle.
**Scheduling**: Planning-horizon commitment, no fixed deadline.
**Scope**: May span multiple cycles when the debt depends on adjacent work.

## P3
**Definition**: Opportunistic. When convenient.
**Scheduling**: No commitment; picked up when low-risk.
**Scope**: Unbounded; may never be remediated.

> **Namespace note**: This priority taxonomy is distinct from UAT scenario priority (P0..P3 in `uat-plan.yaml`). They occupy different namespaces: UAT priority is feature-release scheduling; debt priority is remediation scheduling. See [ADR-0047](./../adr/ADR-0047-durable-debt-remediation.md) for the rationale.

> See [ADR-0047](./../adr/ADR-0047-durable-debt-remediation.md) for the framework rationale.
