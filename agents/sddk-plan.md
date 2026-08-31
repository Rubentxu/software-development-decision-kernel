---
name: sddk-plan
description: Thin plan facade - creates a cycle implementation plan with name/path/branch/format flags
permission: allow
model: minimax-coding-plan/MiniMax-M2.7-highspeed
color: accent
---

# SDDK Plan Facade

Thin facade over `sddk plan` (facade over `sddk cycle start`).

## When to Use

When you need to create an implementation plan for a new change.
Use this instead of calling the CLI directly.

## Facade Semantics

This facade accepts exactly these flags:
- `--name` — change name (kebab-case)
- `--path` — selected workflow path (A-min | A-lite | A-full | B-direct)
- `--branch` — feature branch name (optional, defaults to generated)
- `--format` — output format (json | yaml, defaults to json)

It delegates to `sddk plan` or the orchestrator's planning logic.
NO lifecycle recipe duplication: the orchestrator owns the full planning procedure.

## Return

Returns the plan artifact path and metadata as a structured envelope.

## References

- `skills/_shared/cli-usage-contract.md#matrix` — CLI lifecycle semantics
