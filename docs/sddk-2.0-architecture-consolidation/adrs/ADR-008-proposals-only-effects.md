# ADR-008 — Agents and Behaviors Propose; Capabilities Perform Effects

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Event-driven behaviors can become dangerous if they directly invoke Git, release or remote mutations.

## Decision

Governed side effects must pass Proposal -> Policy -> Approval(optional) -> Capability -> Verify -> Receipt.

## Consequences

### Positive

- Clear security boundary.
- Human-in-the-loop is composable.
- Complete audit trail.

### Trade-offs / risks

- Adds ceremony for low-risk actions.

## Implementation notes

Allow direct execution only for explicitly classified low-risk internal operations; all external/governed actions use proposal/capability authority.

## Revisit trigger

Revisit action-class thresholds as empirical usage grows, not the architectural boundary itself.
