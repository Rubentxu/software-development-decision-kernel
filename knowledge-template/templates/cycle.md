---
type: cycle
title: "<% tp.file.title %>"
slug: "<% tp.file.title %>"
milestone:
status: in_progress
started: <% tp.date.now("YYYY-MM-DD") %>
completed:
path: A-full
branch:
base_commit:
head_commit:
pr:
tag:
verify_verdict:
debt_verdict:
reversibility: medium
artifacts:
  proposal:
  spec:
  design:
  tasks:
  verify_report:
  debt_report:
adrs_touched: []
requirements_touched: []
incidences_found: []
stale_after: <% tp.date.now("YYYY-MM-DD", 365) %>
---

# <% tp.file.title %>

## Artifacts (SDDK phase outputs)

| Artifact | Path / Topic Key |
|----------|-----------------|
| Proposal | `<% tp.frontmatter.artifacts?.proposal %>` |
| Spec (delta) | `<% tp.frontmatter.artifacts?.spec %>` |
| Design | `<% tp.frontmatter.artifacts?.design %>` |
| Tasks | `<% tp.frontmatter.artifacts?.tasks %>` |
| Verify report | `<% tp.frontmatter.artifacts?.verify_report %>` |
| Debt report | `<% tp.frontmatter.artifacts?.debt_report %>` |

## ADRs (decisions made or touched)

<!-- List each ADR with outcome and what it affects -->
- [[]] — proposed | accepted | challenged — affects [[]]

## Requirements (spec changes)

<!-- List each requirement with ADDED/MODIFIED/REMOVED and what changed -->
- [[]] — ADDED | MODIFIED | REMOVED — <one-line summary of change>

## Incidences (problems found during implementation)

<!-- List each incidence discovered. If none, write "none". -->
- [[]] — severity: high | medium | low — <one-line summary>

## Verdicts

- **Verify:** <% tp.frontmatter.verify_verdict ?? "pending" %>
- **Debt-verify:** <% tp.frontmatter.debt_verdict ?? "pending" %>
- **Reversibility:** <% tp.frontmatter.reversibility %>
- **Path:** <% tp.frontmatter.path %>

## Git

- **Branch:** `<% tp.frontmatter.branch %>`
- **Base:** <% tp.frontmatter.base_commit %>
- **Head:** <% tp.frontmatter.head_commit %>
- **PR:** #<% tp.frontmatter.pr %>
- **Tag:** <% tp.frontmatter.tag %>

## Milestone

- [[]]
