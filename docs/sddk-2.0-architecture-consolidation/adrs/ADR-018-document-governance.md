# ADR-018 — Separate Stable Contracts, Current Plans, History and Ephemeral Handoffs

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Agent-facing files have accumulated normative rules and historical/session material.

## Decision

Keep AGENTS.md concise/stable; separate architecture, development, ADR/contracts, changelog, roadmap, future ideas and history. Keep ephemeral handoff in XDG runtime state.

## Consequences

### Positive

- Less agent context noise.
- Reduced documentation drift.

### Trade-offs / risks

- Requires migration and link maintenance.

## Implementation notes

Add a docs index and deterministic inventory checks. Archive, do not erase, historical audits.

## Revisit trigger

Revisit document names if repository conventions evolve; role separation remains.
