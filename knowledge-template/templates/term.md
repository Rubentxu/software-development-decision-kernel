---
type: term
title: "<% tp.file.title %>"
slug: "<% tp.file.title %>"
domain:
status: active
used_in_adrs: []
used_in_requirements: []
---

# <% tp.file.title %>

## Definition

<Clear, precise definition of the term as used in this project.>

## Canonical usage

<How this term SHOULD be used. Example sentence.>

## Avoid (anti-terms)

<Terms that should NOT be used for this concept, and why.>

- ❌ <Wrong term> — <why it's wrong>

## Appears in

- ADRs: <% tp.frontmatter.used_in_adrs?.join(", ") ?? "none yet" %>
- Requirements: <% tp.frontmatter.used_in_requirements?.join(", ") ?? "none yet" %>
