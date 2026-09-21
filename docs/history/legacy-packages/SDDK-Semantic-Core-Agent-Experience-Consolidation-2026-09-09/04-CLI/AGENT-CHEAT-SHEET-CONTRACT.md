# Agent CLI cheat sheet contract

## Purpose

Provide an LLM/agent enough exact operational knowledge to use SDDK correctly **without normal trial-and-error CLI discovery**.

## Example contextual surface

```text
# SDDK commands for target: verify

Inspect project/run state
  sddk status
  effect: read-only

See legal/advisory next actions
  sddk next
  effect: read-only

Preview verification
  sddk run verify --dry-run
  effect: read-only/planning

Execute verification
  sddk run verify
  effect: governed
  machine output: RunReceiptV1

Explain blocker/decision
  sddk why <ref>
  effect: read-only

Inspect effective context/instructions
  sddk context explain --instructions --commands
  effect: read-only
```

The real surface is generated, versioned and may be smaller/larger according to Task/Profile/Packs.

## Required guidance per command

- exact syntax;
- one sentence purpose;
- when to use / when not to use;
- effect class;
- authority requirement (informative);
- output schema if machine-consumable;
- preconditions;
- tested examples;
- related commands;
- deprecation warning if applicable.

## Rules for agents

- use only syntax present in the supplied contract for routine execution;
- prefer machine output when consuming results programmatically;
- do not infer that command visibility grants authority;
- do not invoke plumbing commands unless task contract makes them relevant;
- help/global discovery is a recovery path when the supplied contract is inconsistent, and such mismatch should be reported as agent-contract drift.
