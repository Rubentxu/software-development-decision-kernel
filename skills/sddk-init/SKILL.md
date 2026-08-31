---
name: sddk-init
description: "Trigger: sddk init. Delegate zero-intrusion context and testing-capability detection."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.3"
  delegate_only: true
---

## Activation Contract

Route initialization work to the `sddk-init` executor.

## Hard Rules

- This skill is an adapter; it does not execute the phase inline.
- `prompts/sddk/phases/init.md` is the operational source of truth.

## Decision Gates

| Current role | Action |
|---|---|
| Orchestrator | Delegate to `sddk-init` with CLI-resolved identity and paths |
| `sddk-init` executor | Load the phase prompt and continue |

## Execution Steps

1. Delegate once with the init launch context.
2. Let the executor apply the phase prompt and XDG persistence contract.

## Output Contract

Return the envelope declared in `prompts/sddk/phases/init.md`.

## References

- `prompts/sddk/phases/init.md`
- `skills/_shared/persistence-contract.md`
- `skills/_shared/sddk-phase-common.md`
