# SDDK Cockpit

## Product intent
Cockpit is not merely a dashboard. It is a **moldable operational explorer** over the Event Ledger, graph and projections.

## Local-first generation

```text
Event Ledger
   ↓
Projection Builder
   ↓
control-plane.sqlite / snapshot
   ↓
Static Renderer
   ↓
cockpit.html
```

## Recommended navigation

```text
Overview
Sessions
Workflows
Journal
Graph
Failures
Agents
Models & Providers
Budgets
Context
Evidence
UAT
Supply Chain
Fork / Replay / Diff
```

## Overview cards
- workflows active/completed/failed;
- provider routes healthy/degraded/open;
- failovers over selected period;
- observed token/cost totals;
- attention queue: approvals, blocked workflows, terminal failures;
- longest workflow/node durations.

## Drill-down principle
Every aggregate should navigate to the events/evidence behind it.

Example:

```text
6 failovers
 → list
 → failover #4
 → failed Attempt
 → raw/canonical provider event
 → circuit behavior
 → selected route explanation
 → resumed Attempt
```

## Data embedding
For moderate local datasets embed compressed JSON snapshot. For large history generate a bounded timeframe snapshot or a self-contained directory with chunked local data files. Single-file mode remains the default portable artifact.

## Security
Do not embed secrets, raw authorization headers or full sensitive prompts by default. Use redaction at projection build time.
