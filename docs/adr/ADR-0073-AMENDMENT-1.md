# ADR-0073-AMENDMENT-1 — Secretary Authority Closed-Set Binding

**Amends:** ADR-0073-secretary-authority
**Status:** Accepted (Amendment)
**Date:** 2026-09-04
**Deciders:** orchestrator (auto-evaluator)
**Related:** ADR-069, HX-AUTHORITY-001

---

## Amendment

For taxonomy consistency with ADR-069, the closed-set L1 admission predicate is bound (prose-level) to `actor.kind == Agent && actor.role == "secretary"`. The closed-set itself is unchanged: the 8 auto-resolvable event classes (`provider.rate_limited`, `provider.quota.exhausted`, `host.session.error_observed`, `attempt.interrupted`, `provider.circuit.opened`, `debt.incidence.deferred`, `verifies-stale`, `dependency-blocked`) remain the only admissible triggers per ADR-0073's original table.

The authority prohibitions (`release.*` / `gate.*` / `lease.*` / `receipt.*` are orchestrator-/runtime-exclusive) remain as written in ADR-0073.

### Stage 1 enforcement deferred

Stage 1 enforcement (event-bus validator that rejects Secretary actor on prohibited event types) is explicitly deferred to `RX-SECRETARY-001/002` (runtime admission predicate) and `EVT-LEDGER-001` (for the `ActorKind::Secretary` enum variant that the runtime admission predicate requires). The `role=secretary` binding is prose-level in this amendment; the actual `role` field in `ActorRef` does not exist in code yet and is part of the deferred schema-widening work.

No `ActorKind::Secretary` variant is introduced in A-min per ADR-069 §1.

---
