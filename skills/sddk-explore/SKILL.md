---
name: sddk-explore
description: "Trigger: sddk-explore, sddk-new. Delegate codebase investigation and problem-taxonomy discovery."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.1"
  delegate_only: true
---

## Activation Contract

Route exploration work to the `sddk-explore` executor.

## Hard Rules

- This skill is an adapter; it does not execute the phase inline.
- `prompts/sddk/phases/explore.md` is the operational source of truth.

## Decision Gates

| Current role | Action |
|---|---|
| Orchestrator | Delegate to `sddk-explore` with the launch plan and resolved paths |
| `sddk-explore` executor | Load the phase prompt and continue |

## Execution Steps

1. Delegate once with the phase inputs.
2. Let the executor apply the phase prompt and ledger contract.

## Output Contract

Return the envelope declared in `prompts/sddk/phases/explore.md`.

## References

- `prompts/sddk/phases/explore.md`
- `skills/_shared/sddk-phase-common.md`
