---
name: sddk-tasks
description: "Trigger: sddk-tasks. Delegate review-aware decomposition of specs and design into implementation work units."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.1"
  delegate_only: true
---

## Activation Contract

Route task-planning work to the `sddk-tasks` executor.

## Hard Rules

- This skill is an adapter; it does not execute the phase inline.
- `prompts/sddk/phases/tasks.md` is the operational source of truth.

## Decision Gates

| Current role | Action |
|---|---|
| Orchestrator | Delegate to `sddk-tasks` with the launch plan and resolved paths |
| `sddk-tasks` executor | Load the phase prompt and continue |

## Execution Steps

1. Delegate once with the phase inputs and delivery strategy.
2. Let the executor apply the phase prompt and ledger contract.

## Output Contract

Return the envelope declared in `prompts/sddk/phases/tasks.md`.

## References

- `prompts/sddk/phases/tasks.md`
- `skills/_shared/sddk-phase-common.md`
