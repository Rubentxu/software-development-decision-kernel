---
id: INC-HX-AUTH-001
title: "writable-state surfaces lack explicit authority declaration"
status: closed
severity: high
priority: P1
fingerprint: "hx-auth-001-writable-state-8surface"
fingerprint_aliases: []
cluster_id: CL-HX-WS-001
created: 2026-09-04
created_by: orchestrator
owner: ARCH-HEX-001
closed_at: 2026-09-12
closed_by: sddk-archive (v1.168.27 ARCH-HEX-001 slice 3)
resolution_note: |
  Closed at v1.168.27 (ARCH-HEX-001 slice 3) with full lifecycle evidence.
  Slice 1 (v1.168.22) gated `apply_cycle_start` (CycleState surface).
  Slice 2 (v1.168.24) gated `sddk dev install` (FrameworkBundle surface).
  Slice 3 (v1.168.27, this) emits a script-side actor-kind authority
  receipt in `scripts/release.sh` step 9 via the new
  `scripts/release-receipt.sh` helper. The helper applies the locked
  v1.81.x prefix heuristic (matching
  `crates/sddk-engine/src/authority.rs::infer_actor_kind`) and fails
  closed on Human/Agent actors; the receipt JSON is shipped as a GH
  Release asset and surfaced in `--notes`. Together with the previously
  verified surfaces (KnowledgeGraphVault, GateReceipts, LedgerEvents,
  PlanRevisions, TransitionRecords, ApprovalPoint matrix per ADR-069 §4),
  all 8 ADR-069 §3 writable surfaces are now authority-gated.
last_updated: 2026-09-12
---

# INC-HX-AUTH-001 — writable-state surfaces lack explicit authority declaration

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Eight mutable surfaces (cycle state, ledger events, gate receipts, plan revisions, transition records, framework bundle, GitHub Releases, knowledge graph vault) currently have no formal authority declaration. ADR-069 §3 enumerates the surfaces and names the proposed canonical actor per surface, but no code enforces these declarations.

## Rationale

This is severity **high** because it degrades core functionality without workaround: the system cannot reason about who is authorized to write to critical state surfaces. Priority **P1** because the fix is well-scoped and can land in ARCH-HEX-001 (order 80, H0) alongside other engine-boundary cleanups. The gap is documented per ADR-069 §3; enforcement is deferred.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-04 | orchestrator | created | HX-AUTHORITY-001 cycle; ADR-069 §3 |
| 2026-09-11 | orchestrator (v1.168.23) | partial close: `sddk dev install` (FrameworkBundle surface) now authority-gated — System-only per WRITABLE_SURFACE_MATRIX; agent:/user: prefixed actors rejected fail-closed before prefix mutation; guard tests pin reject+admit | `install_rejects_agent_actor_on_framework_bundle_surface` + `install_admits_plain_actor_as_system_on_bundle_surface` |
| 2026-09-11 | orchestrator (v1.168.22) | partial close: `apply_cycle_start` now authority-gated (CycleState surface) — previously unwired gate receipts + transition records were already gated; remaining surfaces tracked under ARCH-HEX-001 | `Engine::apply_cycle_start` validates via `WRITABLE_SURFACE_MATRIX`; test `apply_cycle_start_is_gated_and_plan_validation_unchanged` |
| 2026-09-12 | orchestrator (v1.168.27) | closed: `scripts/release-receipt.sh` helper emits an actor-kind authority receipt for the GitHub Releases surface (ADR-069 §3 row 7), fail-closed on Human/Agent actors via the locked v1.81.x prefix heuristic. Helper wired into `scripts/release.sh` step 9; receipt JSON shipped as a release asset and summarized in `--notes`. New shell test `tests/test_release_receipt_authority.sh` pins the admit/reject paths (10 scenarios incl. prefix contract, env fallback, fail-closed file guard). All 8 ADR-069 §3 writable surfaces are now authority-gated. | `bash tests/test_release_receipt_authority.sh` (10/10 PASS); shellcheck clean; release v1.168.27 |

## References

- [ADR-069 §3](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md#-decision-2--writable-surface-matrix)
- ARCH-HEX-001 (order 80, H0) — engine-side authority enforcement
