---
id: arch-spec-023-host-capability-negotiation
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-023 — Host Capability Negotiation

## Intent

Agentic Workspace integrations SHALL decide runtime behavior from negotiated semantic capabilities, not from product/version guesses.

## Required capability vocabulary

The generic API SHALL be able to represent at least:

```text
PersistentSessions
ContextInjection
StructuredExecution
SafeInterruption
PermissionMediation
ModelSelection
ReasoningEffort
SessionCompaction
SessionRewind
GlobalEventStream
WorkspaceLocality
```

Equivalent names are acceptable if semantics are preserved.

## Requirements

### HCN-001 — Negotiation is runtime authority

Version/tag/revision metadata is reproducibility evidence. It SHALL NOT substitute for capability negotiation.

### HCN-002 — Unsupported is explicit

A missing capability SHALL be represented as unsupported/unavailable; adapters SHALL NOT silently emulate destructive behavior to pretend support.

### HCN-003 — Host extensions

Host-specific capabilities MAY be namespaced without becoming generic SDDK concepts.

### HCN-004 — Compatibility basis

Receipts SHALL record host product version, exact build/revision where available, protocol/API version and negotiated capabilities.

### HCN-005 — Major protocol mismatch

An incompatible protocol major SHALL produce an explicit incompatible state and SHALL NOT fall back to heuristic operation.

## Acceptance

`AW-UAT-011`, `AW-UAT-014`, `AW-UAT-080`, `AW-UAT-081`, `AW-UAT-090`, `AW-UAT-092`.
