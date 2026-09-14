---
id: arch-spec-024-agentic-session-binding
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-024 — Agentic Session Binding

## Intent

Host sessions SHALL be related to SDDK work through explicit semantic bindings without equating conversation/session lifecycle to SDDK Run lifecycle.

## Requirements

### ASB-001 — Session is not Run

`AgenticSessionRef` and `RunRef` SHALL remain distinct identities. Creating/attaching a host session MUST NOT synthesize an SDDK Run unless an explicit workflow operation does so.

### ASB-002 — Binding modes

Bindings SHALL support at least:

```text
PROJECT
WORK_ITEM
RUN
TASK
EPHEMERAL
```

### ASB-003 — Context basis

Each binding SHALL record the ContextBasis used to compile/deliver current context so stale/superseded bases are observable.

### ASB-004 — Transcript ownership

The host remains transcript authority. SDDK SHALL persist only semantic references, binding/receipt data, Contributions, Evidence and resulting engineering knowledge required by SDDK contracts.

### ASB-005 — Reattach/recovery

A restarted SDDK/integration process SHALL be able to re-establish a valid binding from persisted semantic state + host session identity without duplicating/importing the full transcript.

### ASB-006 — Multi-session isolation

Multiple sessions for one project SHALL keep task/run/context bases independently attributable.

## Acceptance

`AW-UAT-020..024`.
