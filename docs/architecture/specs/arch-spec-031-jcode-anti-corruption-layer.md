---
id: arch-spec-031-jcode-anti-corruption-layer
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-031 — JCode Anti-Corruption Layer

## Intent

The first host integration SHALL use JCode's public Rust Harness API/SDK through a separate `sddk-jcode` anti-corruption layer. JCode types and operational vocabulary SHALL terminate at that boundary.

## Current integration basis

As verified on 2026-09-14:

```text
JCode version:       0.84.0
JCode git revision:  752df77d3c13fa7a648eda257dbfcb8d3ea5974d
jcode-sdk:           0.1.0
jcode-sdk published: false
Harness API major:   1
```

## Requirements

### JAC-001 — Exact development pin

Until a suitable registry-published upstream SDK dependency chain exists, development SHALL pin an exact upstream Git revision and commit the executable integration's lockfile.

### JAC-002 — Public SDK only

`sddk-jcode` SHALL consume JCode through `jcode-sdk` / stable Harness API only. Importing JCode internals to gain convenience is prohibited.

### JAC-003 — SDDK SDK only

`sddk-jcode` SHALL consume SDDK through its public Agentic Workspace integration API/SDK, not private SDDK domain/engine/storage modules.

### JAC-004 — Mapping only

The ACL MAY translate sessions, events, capabilities, context injection, structured execution, permissions and safe interruption. It SHALL NOT decide SDDK debt/alignment/verification/authority semantics.

### JAC-005 — Shared runtime behavior

Companion integration SHOULD connect to JCode's shared runtime/session surface rather than silently launch isolated competing homes/runtimes unless the user/task explicitly requests isolation.

### JAC-006 — Publication honesty

While `jcode-sdk` remains `publish=false` and relies on unpublished/path dependencies, `sddk-jcode` SHALL NOT claim crates.io publication compatibility for a normal dependency graph. Git/tag releases are allowed. Preferred long-term path is upstream publication of the required SDK chain.

### JAC-007 — No renamed upstream fork by default

Do not republish/rename `jcode-sdk` as `sddk-jcode-sdk` merely to bypass publication constraints. A fork requires an explicit ADR describing sync/security/API maintenance cost and exit strategy.

### JAC-008 — Independent SemVer

JCode, Harness API, `jcode-sdk`, SDDK Agentic API and `sddk-jcode` versions SHALL evolve independently. Runtime capability negotiation remains authority; compatibility matrices are evidence.

### JAC-009 — MCP secondary

MCP MAY be added only for demonstrated pull-oriented use cases that remain after native SDK integration. It SHALL NOT replace the primary richer SDK/event path by default.

## Acceptance

`AW-UAT-001..006`, `AW-UAT-080..083`, plus external-workspace dependency guards.
