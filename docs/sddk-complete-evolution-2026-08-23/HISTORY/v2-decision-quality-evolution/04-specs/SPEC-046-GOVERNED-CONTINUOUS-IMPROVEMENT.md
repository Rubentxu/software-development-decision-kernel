# SPEC-046 — Governed Continuous Improvement

**Status:** Proposed

## Purpose

Operationalize ADR-035's learning policy using existing Event Ledger, reactive behaviors, fork/replay/diff and Workflow Laboratory.

## ExperienceEpisode

A derived compact representation of a completed or meaningful execution segment.

```yaml
episode_id: exp-...
source_event_refs: [...]
subject:
  capability: ...
  workflow_template: ...
  workflow_ir_hash: ...
base_revision: ...
configuration_refs:
  skills: [...]
  prompts: [...]
  routing_policy: ...
  context_strategy: ...
outcome:
  accepted: true|false
  verifier: ...
  first_pass: ...
metrics:
  retries: ...
  convergence_rounds: ...
  tokens: ...
  cost: ...
  latency_ms: ...
signals:
  failure_fingerprints: [...]
  human_corrections: [...]
evidence_refs: [...]
```

`ExperienceEpisode` is a projection/index, not source of truth.

## PatternSignal

Produced deterministically when possible:

```text
repeated failure fingerprint
route regression
skill unused/stale
duplicate skill candidates
high handoff entropy
recurring human correction
context repeatedly supplied but unread
workflow over-expansion
verification gap recurrence
```

## ImprovementProposal

```yaml
proposal_id: imp-...
target:
  kind: skill|prompt|agent_manifest|routing_policy|context_strategy|workflow_template|verifier_policy
  ref: ...
motivated_by:
  pattern_refs: [...]
goal:
  metric: ...
constraints:
  preserve: [...]
status: proposed
```

## Candidate

```yaml
candidate_id: cand-...
proposal_id: ...
parent_refs: [...]
target_kind: ...
content_hash: ...
generator:
  strategy: manual|rule|reflection|search
  provider_id: ...
status: proposed|evaluating|rejected|eligible|shadow|promoted|reverted
```

Candidate lineage is a graph projection over `parent_refs`.

## EvaluationContract

Declared before optimization.

```yaml
contract_id: evalc-...
quality:
  required:
    - acceptance_non_regression
    - invariant_coverage_non_regression
    - no_new_blocking_security
datasets:
  train: ...
  development: ...
  holdout:
    ref: ...
    optimizer_access: false
efficiency:
  observe: [tokens, cost, latency, handoffs]
sampling:
  minimum_runs: ...
promotion:
  mode: quality_first
```

## Evaluation lifecycle

```text
candidate
→ isolated fork/worktree
→ development evaluation
→ optional search iterations
→ candidate nomination
→ hidden holdout
→ comparison
→ eligible/rejected
```

Candidate versions are content-addressed and retained according to policy for audit.

## Comparison

Hard gates:

- acceptance/correctness;
- required invariants;
- policy/security;
- evaluation integrity.

Pareto dimensions:

- quality;
- first-pass rate;
- human corrections;
- tokens/cost;
- latency;
- context bytes/read reuse;
- handoffs;
- convergence rounds.

## Promotion

```text
eligible candidate
→ PromotionProposal
→ policy
→ optional human approval
→ activate in shadow
→ bounded rollout
→ monitor
→ promote or revert
→ receipt
```

No candidate generator can grant its own promotion.

## Reactive integration

- **L0:** persist/project.
- **L1:** detect patterns and lifecycle transitions.
- **L2:** generate diagnosis/candidate only for typed bounded signals.

## Skill curation

Initial lifecycle:

```text
active → stale-candidate → archived
```

Signals: last use, success association, patch frequency, semantic overlap, compatibility. No automatic deletion. Consolidation is a candidate evaluated like any other.

## Optional optimization strategies

Not required for v1:

- GEPA-like reflective mutation;
- MCTS/AFlow-like WorkflowIR search;
- candidate populations;
- novelty search;
- lineage/metaproductivity heuristics.

They implement candidate-generation/selection provider interfaces; they do not alter kernel semantics.

## Run-scoped adaptation

A runtime adaptation can create a validated run-local configuration revision. It expires with the run unless converted to an ImprovementProposal and promoted through this protocol.

## Forbidden

- optimizer reads hidden holdout;
- active kernel rewrites itself and continues with new authority;
- evaluator auto-promotes itself;
- replay re-executes effects;
- cost-only objective replaces quality;
- one successful episode directly mutates durable configuration.
