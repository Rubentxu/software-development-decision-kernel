---
id: arch-spec-022-agentic-workspace-boundary
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-022 — Agentic Workspace Boundary

## Intent

SDDK SHALL integrate with coding-agent hosts through a dedicated Agentic Workspace boundary while preserving independent product/runtime ownership.

## Requirements

### AWB-001 — Ownership split

Host owns sessions, transcripts, tools, model/runtime UX and native execution. SDDK owns engineering knowledge, decisions, verification, alignment, governance, evidence and receipts. The integration owns translation/synchronization only.

### AWB-002 — Agent Experience is distinct

Agent Experience contracts (`ContextCapsule`, instructions, skills, return contract) SHALL NOT absorb host/session/runtime concerns. Agentic Workspace SHALL NOT redefine prompt/instruction authority.

### AWB-003 — Independent runtimes

A combined SDDK+host binary SHALL NOT be the architectural target. On-demand local activation is allowed through a public SDDK integration API/SDK.

### AWB-004 — No host internals in SDDK

SDDK domain/engine SHALL NOT import host-internal crates, protocol implementation types or private APIs.

### AWB-005 — Host-specific adapter outside semantic core

JCode-specific mapping SHALL live in a separate `sddk-jcode` integration parcel consuming only public JCode and SDDK integration boundaries.

### AWB-006 — Base independence

Agentic Workspace is not required for `BASE_PRODUCTION_READY`; conversely the host integration MUST remain useful in Base mode without CogniCode/Chronos.

### AWB-007 — Native power preservation

Generic contracts SHALL model SDDK semantics and negotiated capabilities, not a lowest-common-denominator feature set. Host-native extensions MAY remain namespaced.

## Acceptance

`AW-UAT-010`, `AW-UAT-090..092`, plus dependency fitness proving no JCode type/import enters generic SDDK contracts.
