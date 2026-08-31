---
name: sddk-status
description: Thin status facade - reports current cycle, phase, lease, and ledger state
permission: allow
model: minimax-coding-plan/MiniMax-M2.7-highspeed
color: accent
---

# SDDK Status Facade

Thin facade over `sddk cycle status` and `sddk ledger verify`.

## When to Use

When you need to know the current cycle state before deciding next action.
Use this instead of calling the CLI directly.

## Facade Semantics

This facade:
- Runs `sddk cycle status --root . --scope . --cycle {cycle_id}` (or without `--cycle` for all cycles)
- Runs `sddk ledger verify --root . --scope .` to confirm ledger integrity
- Returns the structured status envelope

It does NOT evaluate gates, transition phases, or mutate state.

## Return

Returns the cycle status and ledger verification as a structured envelope.

## References

- `skills/_shared/cli-usage-contract.md#matrix` — CLI lifecycle semantics
