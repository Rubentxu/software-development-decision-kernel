---
name: sddk-archive
description: "Trigger: sddk-archive. Delegate released-cycle closure and durable knowledge sync."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "2.1"
  delegate_only: true
---

## Activation Contract

Route archive after `sddk-release` returns success and runtime state is
`RELEASED/archive`.

## Hard Rules

- Delegate once to `sddk-archive`; never archive inline.
- Preserve cycle state, release receipts, artifact paths/hashes, published SHA,
  tag, path, and vault paths unchanged.
- Treat `prompts/sddk/phases/archive.md` as the sole operational authority.
- Archive closes the cycle; it never performs release Git effects.

## Decision Gates

| Context | Action |
|---|---|
| Caller is the orchestrator | Delegate to `sddk-archive` and stop |
| Caller is `sddk-archive` | Execute the canonical phase prompt |

## Execution Steps

1. Load shared phase context, archive phase prompt, and knowledge-graph skill.
2. Delegator: pass the unchanged launch packet to `sddk-archive`.
3. Executor: execute the phase prompt and return its exact envelope.

## Output Contract

Return the archive report/manifest envelope from the phase prompt. Cycle success
requires runtime `CLOSED`; a release receipt alone is insufficient.

## References

- `../../prompts/sddk/phases/archive.md`
- `../../agents/sddk-archive.md`
- `../_shared/sddk-phase-common.md`
- `../_shared/persistence-contract.md`
- `../knowledge-graph/SKILL.md`
