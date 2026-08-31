# ADR-016 — Expected Outcomes Are Events; Misuse Is Error

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Denials and failed checks are normal facts but exceptions can make them look like runtime faults.

## Decision

Represent expected operational outcomes as events. Keep typed errors for invariant violations/programming misuse.

## Consequences

### Positive

- Better audit semantics.
- Reactive handling of failure/denial.

### Trade-offs / risks

- Requires consistent taxonomy.

## Implementation notes

Document mapping per bounded context and add tests that denials do not abort unrelated processing.

## Revisit trigger

Revisit ambiguous cases through domain-specific ADRs.
