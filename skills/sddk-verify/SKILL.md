---
name: sddk-verify
description: "Trigger: sddk-verify, verify change. Delegate evidence-based verification to the phase coordinator."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.3"
  delegate_only: true
---

## Activation Contract

Route post-implementation verification to `sddk-verify`. The phase is read-only
and precedes debt-verify on A-* paths.

## Hard Rules

- Delegate once to `sddk-verify`; never execute verification inline.
- Preserve `verify_role`, `lens_id`, subject identity, artifact paths, path,
  Strict TDD state, and quality commands unchanged.
- Treat `prompts/sddk/phases/verify.md` as the sole operational authority.
- Only the coordinator persists the report and updates the ledger.

## Decision Gates

| Context | Action |
|---|---|
| Caller is the orchestrator | Delegate with `verify_role: coordinator` |
| Caller is the coordinator dispatching a lens | Delegate with `verify_role: lens` and one `lens_id` |
| Caller is already a verify lens | Execute the assigned lens; never recurse |

## Execution Steps

1. Load `skills/_shared/sddk-phase-common.md` and the canonical phase prompt.
   Load `references/multi-stack-validation.md` only for the affected stacks.
2. Delegator: launch `sddk-verify` with the unchanged packet and stop.
3. Executor: follow the canonical prompt and return its exact role-specific
   envelope.

## Output Contract

The coordinator returns the verification report envelope; a lens returns only
the lens envelope. Both schemas and all verdict rules live in the phase prompt.

## References

- `../../prompts/sddk/phases/verify.md`
- `../../agents/sddk-verify.md`
- `../_shared/sddk-phase-common.md`
- `../_shared/persistence-contract.md`
- `references/multi-stack-validation.md`
