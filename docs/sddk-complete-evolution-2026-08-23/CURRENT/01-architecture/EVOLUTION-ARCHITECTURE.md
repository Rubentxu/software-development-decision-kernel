# Evolution Architecture — Assurance + Controlled Learning

## Context

This evolution composes existing SDDK primitives:

```text
Event Ledger
WorkflowTemplate / WorkflowIR
Capability Registry
Execution Router
Context Compiler
Governed Effects
Fork / Replay / Diff
Workflow Laboratory
Active Graph
Packs
```

## Target architecture

```text
                         USER / PACK GOAL
                               │
                               ▼
                    Supervisor / Workflow
                               │
                    semantic capabilities
                               │
         ┌─────────────────────┴─────────────────────┐
         │                                           │
         ▼                                           ▼
 ENGINEERING ASSURANCE                    NORMAL SDDK EXECUTION
 domain pack                                         │
         │                                           │
 findings / obligations / evidence                   │
         └─────────────────────┬─────────────────────┘
                               ▼
                         Event Ledger
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                ▼
        Active Graph      Projections      Experience View
              │                                 │
              │                           pattern signals
              │                                 ▼
              │                         Improvement Proposal
              │                                 │
              │                                 ▼
              │                         Workflow Laboratory
              │                    fork / replay / diff / holdout
              │                                 │
              │                          candidate evidence
              │                                 ▼
              │                        Promotion Decision
              │                                 │
              └─────────────────────────────────┤
                                                ▼
                                   governed config/artifact update
                                   shadow → bounded rollout
```

## Engineering Assurance boundary

Engineering Assurance owns only:

```text
EngineeringAssessment
AssuranceObligation
EngineeringFinding
AssuranceEvidenceLink
AssuranceVerdict
EngineeringProfile
```

It does not own WorkflowRun scheduling, capability grants, Git writes, model routing, Event Ledger authority or SDD completion semantics.

## Governed Continuous Improvement boundary

Continuous Improvement is not a new execution authority.

```text
Observe
→ Derive Experience
→ Detect Pattern
→ Propose Improvement
→ Materialize Candidate
→ Evaluate Candidate
→ Compare
→ Recommend Promotion
→ Governed Promotion
→ Shadow/Bounded Rollout
→ Monitor
→ Keep/Revert
```

## Experience model

Do not copy raw conversations into another memory store. Derive compact typed episodes:

```yaml
experience_episode:
  id: exp-...
  source_events: [...]
  scope:
    capability: architecture.review
    workflow: sdd-adaptive
  outcome:
    verdict: FAIL
    accepted: false
  signals:
    failure_fingerprints: [...]
    retries: 3
    human_corrections: 1
  route:
    provider: ...
    model: ...
  configuration_refs:
    skill_version: ...
    prompt_hash: ...
    workflow_ir_hash: ...
    context_policy_hash: ...
  evidence_refs: [...]
```

The episode is a projection/index. Event Ledger and artifacts remain authoritative.

## Improvement targets

Versioned targets are explicitly allowlisted:

```text
SkillVersion
PromptVersion
AgentManifestVersion
RoutingPolicyVersion
ContextStrategyVersion
WorkflowTemplateVersion
VerifierPolicyVersion
```

Do not make "anything in the repository" automatically evolvable. Kernel source code is excluded from autonomous promotion.

## Candidate model

```yaml
candidate:
  candidate_id: cand-...
  target_kind: skill
  target_ref: systems-reasoning@1.0
  parent_refs: [...]
  mutation_reason:
    pattern_refs: [...]
  change_hash: ...
  generator:
    capability: improvement.propose
    provider: ...
  status: proposed|evaluating|rejected|eligible|shadow|promoted|reverted
```

A lineage is a projection of `parent_refs`; no Darwinian kernel primitive is required.

## Candidate generation strategies

Start simple:

```text
manual proposal
rule-based proposal
LLM reflection over selected traces
```

Only later add GEPA-like reflective evolution, population search, MCTS over WorkflowIR, novelty/lineage selection. Strategies are providers behind a capability.

## Evaluation contract

Declared before candidate search:

```yaml
evaluation_contract:
  quality_gates:
    - acceptance
    - invariant_coverage
    - regression_free
  datasets:
    train: ...
    development: ...
    holdout: ...
  efficiency_metrics:
    - tokens
    - cost
    - latency
  promotion:
    require_non_inferior_quality: true
    minimum_samples: ...
```

The optimizer must not read hidden holdout content.

## Multi-objective comparison

Hard constraints first:

```text
correctness
policy
security
required invariants
evidence integrity
```

Then Pareto dimensions:

```text
quality
first-pass
human corrections
tokens
cost
latency
handoffs
context reuse
convergence
```

Avoid a single synthetic scalar by default.

## Reactivity

Use existing L0/L1/L2.

### L0 — observe
Record events and projections.

### L1 — deterministic
Detect repeated failures, stale skills, metric regressions, completed experiments and bounded thresholds.

### L2 — cognitive
Only from typed signals: diagnose a pattern, propose candidate, synthesize experiment, explain trade-offs.

No arbitrary event text is blindly injected into prompts.

## Run-scoped adaptation

A workflow may use a validated run-local revision. A successful ephemeral adaptation becomes an ImprovementProposal, not an automatic global config mutation.

## Graph extensions

Projection nodes:

```text
EngineeringAssessment
AssuranceObligation
EngineeringFinding
ExperienceEpisode
ImprovementProposal
ExperimentCandidate
EvaluationContract
CandidateEvaluation
PromotionDecision
ConfigurationVersion
```

All are rebuildable.

## Architecture invariant

> No learning mechanism may bypass the same governance that applies to every other effect in SDDK.

# Addendum — Agent-first interaction layer

The final architecture adds a semantic interaction façade between cognitive agents and application services:

```text
Cognitive layer
      ↓
Goal / State / Query contracts
      ↓
Application services / Workflow Runtime
      ↓
low-level deterministic operations
```

This does not create a new source of truth.

Goal state, reports and tool trajectories are all reconstructed from existing authoritative stores/events/artifacts.

## Shared decision-quality loop

Engineering Assurance and Goal execution both feed Governed Continuous Improvement:

```text
GoalRun
  ↓
detailed evidence/reports
  ↓
Event Ledger
  ↓
ExperienceEpisode
  ↓
tool/workflow pattern signals
  ↓
candidate interface/workflow improvement
  ↓
Workflow Laboratory
```
