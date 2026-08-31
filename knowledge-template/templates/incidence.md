---
type: incidence
title: "<% tp.file.title %>"
slug: "<% tp.file.title %>"
status: open
severity: medium
discovered: <% tp.date.now("YYYY-MM-DD") %>
discovered_in_cycle:
affects_adrs: []
affects_requirements: []
triggers_adr:
resolved_in_cycle:
resolution:
stale_after: <% tp.date.now("YYYY-MM-DD", 90) %>
---

# <% tp.file.title %>

## Problem

<What happened. Be specific: what was expected, what actually occurred, what's the impact.>

## Evidence

<Concrete evidence: error messages, logs, metrics, test output. Not assumptions.>

## Affects

- **ADRs challenged:** [[]]
- **Requirements impacted:** [[]]

## Proposed Resolution

<How to fix it, or why we're accepting it as a risk.>

## Action

- [ ] <Action 1>
- [ ] <Action 2>

## Resolution (filled when closed)

- **Status:** open → resolved | accepted_risk
- **Resolved in:** [[]]
- **How:** <one paragraph>
- **Triggers new ADR:** [[]] (or "no")
