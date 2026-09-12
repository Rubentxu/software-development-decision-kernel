---
id: INC-HX-AUTH-002
title: "forced-Human default in emit_approval_decision masks caller identity"
status: closed
severity: critical
priority: P0
fingerprint: "hx-auth-002-approval-forced-human"
fingerprint_aliases: []
cluster_id: CL-HX-AP-002
created: 2026-09-04
created_by: orchestrator
owner: ARCH-HEX-001
closed_at: 2026-09-12
closed_by: sddk-archive (v1.168.27 ARCH-HEX-001 reconciliation)
resolution_note: |
  Closed after evidence-driven reconciliation. The original INC's claim that
  `emit.rs:259` hardcodes `kind: ActorKind::Human` in every
  `emit_approval_decision` call no longer holds: in the current
  `crates/sddk-engine/src/event_bus/emit.rs`, `emit_approval_decision` builds
  the `ActorRef` from `ApprovalDecisionInput.actor_kind` (caller-supplied),
  and the function-level validator (lines 379-386) explicitly rejects Agent
  actors and admits Human + System, fail-closed. The actor-kind is no longer
  forced at the engine boundary. Three regression-pin tests in
  `crates/sddk-engine/tests/event_authority.rs` lock the admit/reject
  semantics (`emit_approval_decision_accepts_human`,
  `emit_approval_decision_accepts_system`,
  `emit_approval_decision_rejects_agent`). The CLI caller in
  `crates/sddk-cli/src/approval.rs::run_approval_decision` constructs the
  input via `infer_actor_kind(args.actor)` so the user-supplied --actor id
  drives the decision rather than a hardcoded constant. The forced-Human
  behavior was eliminated as part of the wider ARCH-HEX-001 engine-side
  authority enforcement (ADR-070, see v1.168.22 and v1.168.24). The original
  baseline regression test `emit_approval_decision_forces_human` named in
  this INC was retired when its invariant flipped.
last_updated: 2026-09-12
---

# INC-HX-AUTH-002 — forced-Human default in emit_approval_decision masks caller identity

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`crates/sddk-engine/src/event_bus/emit.rs:259` hardcodes `kind: ActorKind::Human` in every `emit_approval_decision` call, regardless of the actual caller identity. This means approval decisions are always attributed to "Human" even when the actual actor is an Agent or the CLI itself. This is the most critical current authority violation.

## Rationale

This is severity **critical** because it breaches the security boundary: approval decisions are always attributed to Human regardless of the actual caller, making it impossible to audit who actually granted or denied an approval. Priority **P0** because it must be resolved before any release tag — it is a release blocker per SEVERITY.md. The baseline test `emit_approval_decision_forces_human` in `actor_authority_baseline_tests.rs` locks this behavior so ARCH-HEX-001 can flip it.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-04 | orchestrator | created | HX-AUTHORITY-001 cycle; emit.rs:259; INC-HX-AUTH-002 |
| 2026-09-12 | orchestrator (v1.168.27) | closed: caller-supplied actor_kind now flows through to `EventEnvelopeV1.actor.kind`; function-level validator rejects Agent; three regression-pin tests lock admit/reject in `crates/sddk-engine/tests/event_authority.rs`. The `emit_approval_decision_forces_human` baseline was retired when its invariant flipped during the wider ARCH-HEX-001 engine-side authority enforcement. | `emit_approval_decision_accepts_human`, `emit_approval_decision_accepts_system`, `emit_approval_decision_rejects_agent`; ADR-070 |

## References

- [ADR-069 §4](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md#-decision-4--provenance-baseline) — provenance baseline
- [emit.rs:259](crates/sddk-engine/src/event_bus/emit.rs:259) — forced Human
- [actor_authority_baseline_tests.rs](crates/sddk-domain/tests/actor_authority_baseline_tests.rs) — regression baseline
- ARCH-HEX-001 (order 80, H0) — engine-side authority enforcement
