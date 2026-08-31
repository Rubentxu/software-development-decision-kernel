---
name: sddk-run
description: Thin run facade - executes an approved SDDK plan end-to-end through apply and verify
permission: allow
model: minimax-coding-plan/MiniMax-M2.7-highspeed
color: accent
---

# SDDK Run Facade

Thin facade that drives a cycle from build through verify.

## When to Use

When you have an approved plan and want to execute it through apply and verify
without manually dispatching each phase. The orchestrator owns dispatch; this
facade is the execution driver.

## Facade Semantics

This facade:
- Dispatches `sddk-apply` with the approved plan context
- After apply completes, dispatches `sddk-verify`
- Collects and returns both phase envelopes

It does NOT evaluate gates or transition phases — those remain orchestrator-owned.
NO lifecycle recipe duplication.

## Return

Returns the apply and verify phase envelopes as a combined execution summary.

## References

- `skills/_shared/cli-usage-contract.md#matrix` — CLI lifecycle semantics
