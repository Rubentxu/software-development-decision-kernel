# SPEC-040 — Workflow Laboratory

**Status:** Proposed

## Purpose
Evaluate workflow/harness structure empirically and provide the feedback loop for simplifying SDDK without sacrificing quality.

## Experiment model

```text
WorkflowExperiment
  goal/base_revision/evaluation_contract
  ├─ Run A: A-full
  ├─ Run B: sdd-adaptive
  └─ optional forks/ablations
```

Runs must be isolated via worktrees/forks when they mutate code.

## Experiment types
- baseline vs adaptive;
- ablation: remove/merge one phase/capability;
- verifier-depth comparison;
- model/provider route comparison;
- prompt/policy comparison;
- static vs dynamic decomposition.

## Metrics
Quality first:
- acceptance/invariant coverage;
- tests/regressions;
- architecture/security/UAT findings;
- evidence completeness;
- human corrections.

Efficiency second:
- lead time;
- tokens/cost;
- agent calls;
- handoffs;
- context read/reuse;
- WorkUnits and convergence rounds.

## Cockpit integration
Static views:
- experiment summary;
- side-by-side execution graph;
- timeline;
- cost/token waterfall;
- gap/remediation comparison;
- causal explanation of why one strategy expanded more.

## Promotion rule
Adaptive/default policy changes require explicit thresholds and bounded rollout. A cheaper workflow cannot replace baseline if quality confidence is insufficient.

## Test-tooling boundary

Test-tooling ownership for Workflow Laboratory assets follows ADR-0069: Python owns external golden/evaluation/analytical assets (scripts/, experiment harness, scenario generators). See [ADR-0069-test-tooling-ownership.md](../../adr/ADR-0069-test-tooling-ownership.md) and [TEST-TOOLING-EVIDENCE-AUDIT.md](../09-implementation/TEST-TOOLING-EVIDENCE-AUDIT.md).
