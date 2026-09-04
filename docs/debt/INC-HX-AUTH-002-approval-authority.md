---
id: INC-HX-AUTH-002
title: "forced-Human default in emit_approval_decision masks caller identity"
status: resolved
severity: critical
priority: P0
fingerprint: "hx-auth-002-approval-forced-human"
fingerprint_aliases: []
cluster_id: CL-HX-AP-002
created: 2026-09-04
created_by: orchestrator
owner: ARCH-HEX-001
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

## References

- [ADR-069 §4](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md#-decision-4--provenance-baseline) — provenance baseline
- [emit.rs:259](crates/sddk-engine/src/event_bus/emit.rs:259) — forced Human
- [actor_authority_baseline_tests.rs](crates/sddk-domain/tests/actor_authority_baseline_tests.rs) — regression baseline
- ARCH-HEX-001 (order 80, H0) — engine-side authority enforcement
