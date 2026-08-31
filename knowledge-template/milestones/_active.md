---
type: active_lock
milestone:
acquired:
---

# Active Cycle Lock

> **This file is the serialization lock.** If it exists and contains a milestone link, no new SDDK cycle can start.
>
> **Acquired** by the orchestrator at MCW Step 0.2 when a new cycle begins.
> **Released** by `sddk-release` when the cycle closes (milestone status → completed).

## Current Lock

**Status:** AVAILABLE (no active cycle)

<!-- When a cycle is active, this file contains:
**Status:** LOCKED

**Milestone:** [[M-NNN-slug]]
**Acquired:** YYYY-MM-DD
**Branch:** `<type>/<description>`
**Cycle:** [[CYC-date-slug]]

To release: complete the cycle via `/sddk-release <change>`, or mark the milestone as `blocked`/`abandoned` in its node file.
-->
