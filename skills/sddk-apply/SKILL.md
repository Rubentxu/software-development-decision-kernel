---
name: sddk-apply
description: "Trigger: orchestrator launches sddk-apply. Delegate implementation of one approved SDDK task slice."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "1.1"
  delegate_only: true
---

## Activation Contract

Route one approved implementation slice to the `sddk-apply` executor.

## Hard Rules

- This skill is an adapter; it does not execute the phase inline.
- `prompts/sddk/phases/apply.md` is the operational source of truth.

## Decision Gates

| Current role | Action |
|---|---|
| Orchestrator | Delegate to `sddk-apply` with task acceptance, branch, mode, and resolved paths |
| `sddk-apply` executor | Load the phase prompt and continue |

## Execution Steps

1. Delegate once with the complete apply launch context.
2. Let the executor apply the phase prompt and ledger contract.

## Output Contract

Return the completion or blocker envelope declared in
`prompts/sddk/phases/apply.md`.

## References

- `prompts/sddk/phases/apply.md`
- `prompts/sddk/phases/apply-strict-tdd.md`
- `skills/_shared/sddk-phase-common.md`
