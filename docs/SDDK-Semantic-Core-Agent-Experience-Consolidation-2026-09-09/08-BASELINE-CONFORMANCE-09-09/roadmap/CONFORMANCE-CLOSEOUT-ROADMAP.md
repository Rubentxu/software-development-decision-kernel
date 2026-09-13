# 09/09 Conformance Closeout Roadmap

This roadmap is a **prerequisite to R0** of the 2026-09-10 context-first roadmap. It is deliberately finite: no new product features.

## C0 — Freeze and executable traceability baseline

**Goal:** know exactly what “done” means before changing code.

Deliverables:
- populate SPEC-001..018 crosswalk;
- map UAT-01..22 to concrete existing/new tests;
- inventory every deprecated symbol/path from M9;
- capture baseline dependency graph and installed CLI fixtures;
- mark each finding PASS/PASS_WITH_COMPAT/FAIL/UNKNOWN.

Exit:
- zero unowned requirement;
- zero UAT without planned executable verification;
- no production refactor before baseline evidence snapshot is committed.

## C1 — Canonical fact/event + Evidence convergence

Implements SPEC-CONF-002 and SPEC-CONF-003.

Deliverables:
- one event append authority;
- read-only/idempotent legacy migration path;
- `PlanningEvidenceKind` writes removed;
- universal evidence relation migration;
- architecture ratchets become blocking.

Exit:
- no second reachable event/evidence write authority;
- projection rebuild and migrated-repo tests green.

## C2 — Lifecycle convergence

Implements SPEC-CONF-004.

Deliverables:
- approval/UAT/retry/recovery truth lives on Run/Authority/evidence;
- Cycle summary derived;
- deprecated runtime Cycle variants stop receiving writes;
- compatibility rendering parity.

Exit:
- UAT-03 and UAT-04 green;
- static guard rejects new runtime-specific Cycle writes.

## C3 — Revision / Decision Memory convergence

Implements SPEC-CONF-005.

Deliverables:
- classify all Decision Memory revision primitives;
- reuse common OID/ref/CAS/ancestry primitives where semantically identical;
- ADR narrow exceptions where semantics truly differ;
- migration/recovery fixtures.

Exit:
- one implementation per generic revision invariant;
- M4 race/recovery/diff scenarios green.

## C4 — AuthorityEngine cutover

Implements SPEC-CONF-006.

Deliverables:
- enumerate effectful call sites;
- route every governed effect through AuthorityEngine;
- legacy authority entrypoints frozen/internalized/removed;
- bypass architecture guard.

Exit:
- UAT-04/UAT-16/UAT-22 green;
- zero direct governed-effect bypass.

## C5 — Command registry + agent asset convergence

Implements SPEC-CONF-007.

Deliverables:
- eliminate authoritative handwritten CLI syntax duplication;
- executable examples from typed command substrate;
- remove obsolete monolithic prompts/cheat sheets after parity;
- agent asset static scanner becomes blocking.

Exit:
- UAT-13..22 green;
- CLI drift mutation test fails as expected;
- no deprecated semantic name in active agent asset.

## C6 — Documentation/governance reconciliation

Implements SPEC-CONF-008.

Deliverables:
- reconcile spec statuses;
- one normative architecture entry point;
- explicit supersession banners;
- compatibility allowlist with expiry/removal trigger.

Exit:
- docs drift check green;
- no `proposed` status for implemented normative specs without reason.

## C7 — Full clean + migrated conformance run

Implements SPEC-CONF-009.

Run:
- T0..T6 checks as applicable;
- UAT-01..22;
- clean repository adoption/workflow;
- migrated repository workflow;
- projection deletion/rebuild;
- fresh-process recovery;
- installed CLI examples;
- architecture dependency graph/deprecated path scan.

Exit:
- all SPEC-001..018 PASS/PASS_WITH_COMPAT;
- UAT-01..22 PASS;
- zero unresolved MUST finding;
- `09-09-CONFORMANCE-RECEIPT.md` signed by CI evidence/commit hash.

## Dependency

```text
C0 -> C1 -> C2 -> C3 -> C4 -> C5 -> C6 -> C7 -> R0
```

Parallelization is allowed only when it does not obscure ownership or cause two temporary authorities to gain new consumers.
