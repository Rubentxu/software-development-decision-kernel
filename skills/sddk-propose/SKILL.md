---
name: sddk-propose
description: "Trigger: sddk-new, sddk-propose. Delegate creation of an adaptive SDDK change proposal."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.1"
  delegate_only: true
---

## Activation Contract

Route proposal work to the `sddk-propose` executor.

## Hard Rules

- This skill is an adapter; it does not execute the phase inline.
- `prompts/sddk/phases/propose.md` is the operational source of truth.

## Decision Gates

| Current role | Action |
|---|---|
| Orchestrator | Delegate to `sddk-propose` with the launch plan and resolved paths |
| `sddk-propose` executor | Load the phase prompt and continue |

## Execution Steps

1. Delegate once with the phase inputs.
2. Let the executor apply the phase prompt and artifact contract.

## Output Contract

Return the envelope declared in `prompts/sddk/phases/propose.md`.

## References

- `prompts/sddk/phases/propose.md`
- `skills/_shared/sddk-phase-common.md`
