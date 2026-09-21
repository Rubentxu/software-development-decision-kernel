---
id: arch-spec-028-permission-interruption-bridge
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-028 — Permission and Interruption Bridge

## Intent

SDDK SHALL cooperate with host-native permission/interruption mechanisms without creating duplicate approval authorities or mapping advisory architecture opinions into hard execution control.

## Requirements

### PIB-001 — Single user approval flow

When no SDDK policy governs an action, normal host permission UX remains authoritative. The integration SHALL NOT add a second approval ceremony.

### PIB-002 — Modes

Permission mediation SHALL expose explicit modes:

```text
OBSERVE
ADVISE
ENFORCE
```

`ADVISE` is the default. `ENFORCE` requires explicit configuration/policy authority.

### PIB-003 — Alignment cannot deny

Software Alignment observations/tensions MAY decorate context/approval information but SHALL NOT independently deny host permission or release execution.

### PIB-004 — Governed denial

When an explicit AuthorityEngine policy denies an SDDK-governed effect, the adapter MAY map that result to a host denial with provenance/explanation.

### PIB-005 — Semantic interruption

SDDK SHALL emit semantic intent such as `InterruptSafely`, not host method names. Adapters map that intent to `soft_interrupt` or equivalent only when negotiated.

### PIB-006 — No destructive emulation

If safe interruption is unavailable, the adapter SHALL report unsupported/deferred rather than kill/cancel unrelated host state to imitate the capability.

### PIB-007 — Attention discipline

Normal Alignment tension SHALL not trigger interruption. Interrupt-level behavior is reserved for explicit material invalidation/cancellation/supersession conditions.

## Acceptance

`AW-UAT-050..053` and `AW-UAT-070..073`.
