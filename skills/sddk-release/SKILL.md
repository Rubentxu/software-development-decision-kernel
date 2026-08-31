---
name: sddk-release
description: "Trigger: sddk-release. Delegate local Git publication and receipt capture before archive."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "1.0"
  delegate_only: true
---

## Activation Contract

Route the mandatory publication step before archive. A-* paths arrive with
passing verify and debt evidence; B-direct follows its declared gate.
Release produces `release-receipt`; archive consumes it and emits
`archive-manifest` (chain: `release-receipt` → `archive-manifest`).

## Hard Rules

- Delegate once to `sddk-release`; never execute Git effects inline.
- Preserve candidate SHA, path, evidence paths/hashes, cycle state, and tag plan.
- Treat `prompts/sddk/phases/release.md` as the phase authority and
  `prompts/sddk/git-contract.md` as Git authority.
- Local verify -> push main -> verify head is the mandatory publication route after verify;
  annotated tag is mandatory and peels to verified SHA before archive. Optional post-tag
  external distribution (GitHub Releases, CI/CD assets) is not required for cycle success.
- Do not add forge, CI/CD, or distribution state to release success.

## Decision Gates

| Context | Action |
|---|---|
| Caller is the orchestrator | Delegate to `sddk-release` and stop |
| Caller is `sddk-release` | Execute the canonical phase prompt |

## Execution Steps

1. Load shared phase context, release phase prompt, and Git contract.
2. Delegator: pass the unchanged launch packet to `sddk-release`.
3. Executor: execute the phase prompt and return its exact envelope.

## Output Contract

Return the release report envelope from the phase prompt. Archive artifacts and
cycle-closure claims are not release outputs.

## References

- `prompts/sddk/phases/release.md` — phase authority
- `prompts/sddk/git-contract.md` — Git authority
- `skills/_shared/sddk-phase-common.md` — shared executor protocol
- `skills/sddk-archive/SKILL.md` — successor adapter
