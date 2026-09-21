---
id: arch-spec-019-production-readiness-convergence
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-019 — Production Readiness Convergence

## Intent

SDDK SHALL reach production readiness by closing the accepted 09/09 semantic baseline and then implementing the accepted 10/09 context-first evolution in dependency order. This specification prevents roadmap drift, silent scope loss and ambiguous “done” declarations.

It does not redefine SPEC-001..018 or the context-first domain specs. It defines how their completion is certified.

## Requirements

### PRC-001 — C7 precedes R0 completion

The 09/09 conformance closeout SHALL produce a green C7 receipt before R0 is declared complete.

The receipt SHALL prove:

- SPEC-001..018 `PASS` or `PASS_WITH_COMPAT`;
- UAT-01..22 `PASS`;
- zero unresolved MUST findings;
- clean, migrated and recovery fixtures;
- commit SHA and executable evidence references.

### PRC-002 — Compatibility is bounded

`PASS_WITH_COMPAT` is valid only when the canonical production authority is already correct and the compatibility path has:

- explicit scope;
- owner;
- executable fixture;
- read/write behavior;
- detection/architecture guard where applicable;
- objective removal trigger.

A second reachable writable authority cannot be classified as compatibility.

### PRC-003 — Accepted work has explicit disposition

Every important accepted proposal SHALL be one of:

```text
IMPLEMENTED
ADOPTED
DEFERRED(reason, trigger)
SUPERSEDED_BY(id)
REJECTED(reason, decision-ref)
```

Deleting it from a later roadmap is not a disposition.

The project SHALL maintain the disposition register at `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/06-PROPOSAL-DISPOSITION-REGISTER.md` until final convergence.

### PRC-004 — Base readiness is provider-independent

`BASE_PRODUCTION_READY` SHALL NOT require CogniCode or Chronos to be installed or available.

Missing optional/preferred intelligence SHALL be represented as `NOT_EVALUATED` / EvidenceGap or equivalent. It SHALL NOT manufacture PASS evidence.

### PRC-005 — Enhanced readiness is capability-specific

SDDK SHALL distinguish at least:

```text
BASE_PRODUCTION_READY
STATIC_ENHANCED_PRODUCTION_READY
RUNTIME_ENHANCED_PRODUCTION_READY
FULLY_ENHANCED_PRODUCTION_READY
```

The enhanced claims require runtime capability negotiation and provider-specific integration receipts.

### PRC-006 — Receipt, not status text

A production-ready claim SHALL bind to a receipt containing at least:

- SDDK version + commit;
- claimed capability profile;
- spec/architecture basis;
- storage schema/migration basis;
- command-registry basis;
- verification/UAT evidence;
- unresolved accepted risks;
- provider compatibility basis when applicable.

Roadmap percentage, milestone prose or aggregate test count cannot substitute for this receipt.

### PRC-007 — Domain boundaries remain provider-neutral

Production readiness requires architecture fitness checks proving that domain/Knowledge/Alignment/Verification semantics do not depend on provider RPC/SDK types.

### PRC-008 — Architecture succession is unambiguous

Documentation SHALL expose one current normative architecture entry point and a deterministic succession from:

```text
09/09 semantic baseline
 -> C0..C7 conformance closeout
 -> 10/09 context-first roadmap adoption/evolution
 -> readiness receipts
```

Historical packages may remain available but cannot claim competing current authority.

## Readiness gates

The detailed gates and UAT matrix are normative companions:

- `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/03-PRODUCTION-READY-GATE.md`
- `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md`

## Acceptance

This spec is satisfied when:

1. all MUST rows in the current gap register are `PASS`/valid `PASS_WITH_COMPAT` for the claimed profile;
2. the matching readiness gates are green;
3. a named receipt records executable evidence at a specific commit;
4. no important accepted proposal lacks explicit disposition.
