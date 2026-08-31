---
type: adr
title: "<% tp.file.title %>"
slug: "<% tp.file.title %>"
status: proposed
created: <% tp.date.now("YYYY-MM-DD") %>
decided:
superseded_by:
created_in_cycle:
affects_requirements: []
affects_domains: []
related_adrs: []
challenged_by:
stale_after: <% tp.date.now("YYYY-MM-DD", 365) %>
---

# <% tp.file.title %>

## Context and Problem Statement

<Describe the forces at play: technical, business, or political context. The question that needs to be answered. Any constraints.>

## Decision Drivers

1. <Driver 1>
2. <Driver 2>
3. <Driver 3>

## Considered Options

### Option 1: <Name>
<Description. Pros. Cons.>

### Option 2: <Name>
<Description. Pros. Cons.>

## Decision Outcome

**Chosen option:** "<Option N>", because <justify against drivers>.

### Consequences

**Positive:**
- <Benefit 1>

**Negative:**
- <Cost 1>

**Risks:**
- <Risk 1 and mitigation>

## Specs Impacted by This Decision

| Domain | Requirement | Impact |
|--------|-------------|--------|
| [[auth]] | [[REQ-Session-Expiration]] | Defines HOW sessions expire |
| [[auth]] | [[REQ-Token-Refresh]] | Refresh depends on this ADR |

## Implementation Log (append-only — updated by `sddk-release`)

> One entry per cycle that touches this ADR. Do not edit existing entries.

<!-- Example entry:
### 2026-08-03 — CYC-007 (PR #42, v1.2.0)
- **outcome:** accepted | challenged
- **incidences:** [[INC-002-replica-lag]] (or "none")
- **scope_changes:** "read replicas deferred to next cycle" (or "none")
- **health:** sound | needs revision — <why>
-->

## Changelog (bi-temporal)

- <% tp.date.now("YYYY-MM-DDTHH:mm") %> | created | status=proposed | valid_from=<% tp.date.now("YYYY-MM-DD") %> | valid_to=∞
