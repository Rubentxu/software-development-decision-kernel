---
type: milestone
title: "<% tp.file.title %>"
slug: "<% tp.file.title %>"
status: planned
domain:
priority: 1
created: <% tp.date.now("YYYY-MM-DD") %>
started:
completed:
target_version:
branch:
pr:
tag:
cycle:
linked_adrs: []
linked_specs: []
depends_on: []
stale_after: <% tp.date.now("YYYY-MM-DD", 90) %>
---

# <% tp.file.title %>

## Goal

<What this milestone achieves. One paragraph.>

## Scope

**In:**
- <Item 1>
- <Item 2>

**Out:**
- <Explicitly deferred>

## Tracking

- **Branch:** (created at Step 1.8)
- **PR:** (created at Step 3)
- **Tag:** (pushed at Step 3)
- **Cycle manifest:** [[CYC-<date>-<slug>]]

## Decisions (ADRs)

- [[ADR-NNN]] — <one-line summary>
- [[ADR-NNN]] — <one-line summary>

## Requirements (specs touched)

- [[REQ-Slug]] — ADDED | MODIFIED | REMOVED
- [[REQ-Slug]] — ADDED | MODIFIED | REMOVED

## Dependencies

- Depends on: [[M-NNN]] (or "none")
- Blocks: [[M-NNN]] (or "none")

## Changelog (bi-temporal)

- <% tp.date.now("YYYY-MM-DDTHH:mm") %> | created | status=planned | valid_from=<% tp.date.now("YYYY-MM-DD") %> | valid_to=∞
