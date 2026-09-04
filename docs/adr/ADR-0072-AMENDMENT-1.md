# ADR-0072-AMENDMENT-1 — Secretary Budget Naming Alignment

**Amends:** ADR-0072-secretary-budgets
**Status:** Accepted (Amendment)
**Date:** 2026-09-04
**Deciders:** orchestrator (auto-evaluator)
**Related:** ADR-069, HX-AUTHORITY-001

---

## Amendment

For taxonomy consistency with ADR-069, the Secretary's runtime identity is renamed from the prose-level `secretary role` to `Agent{role=secretary, behavior_id, closed_set_version}`. This aligns the ADR-0072 naming with the canonical 4-actor taxonomy established in ADR-069: Secretary is `Agent` with `role=secretary`, not a distinct actor kind.

This amendment changes only the actor-naming alignment with the canonical 4-actor taxonomy; it does **NOT** alter ADR-0072's budget-composition decision (composition of `Budgets.cycle` per ADR-0068/0070), the per-call / cycle-budget structure, or the advisory record shape. No new domain `Budgets.agent` is introduced.

The `behavior_id` and `closed_set_version` fields are prose-level descriptors for the Secretary's behavioural definition and the version of its closed-set L1 (per ADR-0073). These fields do not exist in code yet — they are part of the deferred `EVT-LEDGER-001` and `RX-SECRETARY-001/002` scope.

---
