---
name: sddk-design
description: "Trigger: sddk-design. Delegate creation of an adaptive technical design from SDDK evidence."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.1"
  delegate_only: true
---

## Activation Contract

Route design work to the `sddk-design` executor.

## Hard Rules

- This skill is an adapter; it does not execute the phase inline.
- `prompts/sddk/phases/design.md` is the operational source of truth.

## Decision Gates

| Current role | Action |
|---|---|
| Orchestrator | Delegate to `sddk-design` with the launch plan and resolved paths |
| `sddk-design` executor | Load the phase prompt and continue |

## Execution Steps

1. Delegate once with the phase inputs and selected capabilities.
2. Let the executor apply the phase prompt and ledger contract.

## Output Contract

Return the envelope declared in `prompts/sddk/phases/design.md`.

## References

- `prompts/sddk/phases/design.md`
- `skills/_shared/sddk-phase-common.md`
