---
id: arch-spec-029-workspace-locality
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-029 — Workspace Locality

## Intent

Agentic Workspace integrations SHALL distinguish logical workspace identity from the location where host/tool operations execute.

## Requirements

### WSL-001 — Identity vs location

`WorkspaceIdentity` SHALL identify the logical project/workspace. `WorkspaceLocation` SHALL represent the operational execution locality.

### WSL-002 — Locality vocabulary

The integration SHALL support at least semantically equivalent states to:

```text
Local
Ssh
Container
Remote
```

### WSL-003 — No local-path guessing

SDDK SHALL NOT assume that a host-reported workspace/session can be addressed through the SDDK process's local filesystem.

### WSL-004 — Host capability mediation

Remote/SSH/container operations SHALL be consumed through negotiated public host capabilities/adapters rather than host internals.

### WSL-005 — Evidence basis

Receipts/evidence that depend on workspace files SHALL record enough identity/location/revision basis to explain where the observation was produced without leaking credentials/secrets.

## Acceptance

`AW-UAT-023` and remote/locality variants of J3/J8 integration tests.
