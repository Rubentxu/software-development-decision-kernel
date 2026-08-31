---
name: sddk-spec
description: "Trigger: sddk-spec. Delegate behavior-specification work from an approved SDDK proposal."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.1"
  delegate_only: true
---

## Activation Contract

Route specification work to the `sddk-spec` executor.

## Hard Rules

- This skill is an adapter; it does not execute the phase inline.
- `prompts/sddk/phases/spec.md` is the operational source of truth.

## Decision Gates

| Current role | Action |
|---|---|
| Orchestrator | Delegate to `sddk-spec` with the launch plan and resolved paths |
| `sddk-spec` executor | Load the phase prompt and continue |

## Execution Steps

1. Delegate once with the phase inputs.
2. Let the executor apply the phase prompt, knowledge writes, and ledger contract.

## Output Contract

Return the envelope declared in `prompts/sddk/phases/spec.md`.

## References

- `prompts/sddk/phases/spec.md`
- `skills/_shared/sddk-phase-common.md`
