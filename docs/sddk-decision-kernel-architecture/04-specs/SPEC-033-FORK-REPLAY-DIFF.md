# SPEC-033 — Fork, Replay & Diff

**Status:** Proposed

## Replay
Rebuild state/projections from the original event stream without new external effects.

## Fork
Create a new logical branch at an event/version boundary:

```yaml
fork_id: fork-B
parent_correlation: wf-123
at_event: evt-890
changes:
  model_policy: local-first
```

## Experiments
Compare:
- model/provider;
- prompt/agent definition;
- context strategy;
- workflow strategy;
- policy;
- verifier.

## Diff dimensions
- final artifacts/content hash;
- tests/verification;
- architecture graph change;
- events/decisions;
- tokens/cost/time;
- human intervention;
- evidence completeness.

## Side effects
Experiments run in isolated worktrees/sandboxes by default. External irreversible effects require explicit policy and should generally be disabled for A/B.
