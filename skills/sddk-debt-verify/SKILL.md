---
name: sddk-debt-verify
description: "Trigger: sddk-debt-verify, debt verify. Delegate the path-derived post-verify debt gate and return evidence-bound reports."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: SDDK Team
  version: "1.1"
  delegate_only: true
---

## Activation Contract

Route the mandatory post-verify debt gate on A-* paths. B-direct does not load
this skill. The canonical phase prompt derives depth and workers from the path.

## Hard Rules

- Delegate to `sddk-debt-verify`; only that coordinator may launch debt workers.
- Treat `prompts/sddk/phases/debt-verify.md` as the sole operational authority.
- Preserve the launch packet unchanged; do not choose depth, workers, verdict,
  or remediation in this adapter.
- Keep runtime handoff `specification_only`; do not claim CLI enforcement.

## Decision Gates

| Signal | Rule |
|---|---|
| Caller is not `sddk-debt-verify` | Delegate once and stop |
| Caller is the coordinator | Load the phase prompt and execute it |
| Path is B-direct | Return to the caller; the workflow disables this gate |

## Execution Steps

1. Load shared phase context and the canonical phase contract.
2. If acting as delegator, launch `sddk-debt-verify` with the unchanged packet
   and stop.
3. If acting as coordinator, execute the canonical phase prompt and return its
   exact envelope.

## Output Contract

Required artifacts:

- `{cycle-artifacts-dir}/debt-report.json` - machine authority.
- `{cycle-artifacts-dir}/debt-report.md` - human projection.

The envelope includes `contract_version`, subject binding, cluster coverage,
finding counts, verdict, remediation target, runtime handoff status, risks, and
context quality. Exact fields live in the canonical phase contract.

## References

- `../../prompts/sddk/phases/debt-verify.md`
- `../../agents/sddk-debt-verify.md`
- `../_shared/sddk-phase-common.md`
- `../_shared/persistence-contract.md`
- `../../prompts/sddk/orchestrator.md`
- `../../prompts/sddk/mcw.md`
