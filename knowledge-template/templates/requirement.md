---
type: requirement
title: "<% tp.file.title %>"
slug: "<% tp.file.title %>"
domain:
status: active
created: <% tp.date.now("YYYY-MM-DD") %>
created_in_cycle:
last_modified_cycle:
last_modified_version:
decision_authority:
tested_by:
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: <% tp.date.now("YYYY-MM-DD", 90) %>
---

# <% tp.file.title %>

## Requirement

The system <% tp.frontmatter.rfc2119 %> <do something specific>.

<Description using RFC 2119 keywords: MUST, SHALL, SHOULD, MAY.>

## Scenarios

### Scenario: <Happy path>
- **GIVEN** <precondition>
- **WHEN** <action>
- **THEN** <expected outcome>

### Scenario: <Edge case>
- **GIVEN** <precondition>
- **WHEN** <action>
- **THEN** <expected outcome>

## Traceability

- **Decision authority:** <% tp.frontmatter.decision_authority ?? "none — pure requirement" %>
- **Created in cycle:** <% tp.frontmatter.created_in_cycle ?? "unknown" %>
- **Last modified:** <% tp.frontmatter.last_modified_cycle ?? "never" %> (<% tp.frontmatter.last_modified_version ?? "" %>)
- **Tested by:** `<% tp.frontmatter.tested_by ?? "NOT TESTED" %>` (verified: <% tp.frontmatter.verified_in_cycle ?? "never" %>)
- **Incidences:** <% tp.frontmatter.incidences?.join(", ") ?? "none" %>

## Changelog (bi-temporal)

- <% tp.date.now("YYYY-MM-DDTHH:mm") %> | created | cycle=<% tp.frontmatter.created_in_cycle %> | valid_from=<% tp.date.now("YYYY-MM-DD") %> | valid_to=∞
