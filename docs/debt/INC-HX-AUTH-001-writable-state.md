---
id: INC-HX-AUTH-001
title: "writable-state surfaces lack explicit authority declaration"
status: open
severity: high
priority: P1
fingerprint: "hx-auth-001-writable-state-8surface"
fingerprint_aliases: []
cluster_id: CL-HX-WS-001
created: 2026-09-04
created_by: orchestrator
owner: ARCH-HEX-001
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

## References

- [ADR-069 §3](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md#-decision-2--writable-surface-matrix)
- ARCH-HEX-001 (order 80, H0) — engine-side authority enforcement
