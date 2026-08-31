---
name: sddk-recover
description: Thin recover facade - recovers from blocked or failed cycles
permission: allow
model: minimax-coding-plan/MiniMax-M2.7-highspeed
color: accent
---

# SDDK Recover Facade

Thin facade over cycle recovery actions.

## When to Use

When a cycle is blocked, failed, or in a RECOVERING state and you need to
resume or clean up. This replaces direct CLI calls for recovery operations.

## Facade Semantics

This facade:
- Diagnoses the current blocked state
- Renews or releases the cycle lease as appropriate
- Suggests the correct recovery action (replan, re-apply, re-verify, abandon)
- Returns a structured recovery recommendation

It does NOT mutate cycle state directly — it recommends the correct action.
NO lifecycle recipe duplication.

## Return

Returns a recovery recommendation envelope with diagnostic details.

## References

- `skills/_shared/cli-usage-contract.md#matrix` — CLI lifecycle semantics
