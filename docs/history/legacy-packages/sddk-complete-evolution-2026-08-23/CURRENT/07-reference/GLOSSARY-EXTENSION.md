# Glossary Extension

**Engineering Assurance** — bounded context producing evidence-backed engineering assessments.

**Assurance Obligation** — condition that must be demonstrated true for a scope.

**Engineering Finding** — normalized claim linked to evidence and optionally an obligation.

**Engineering Profile** — technology/runtime specialization of generic engineering capabilities.

**ExperienceEpisode** — derived compact projection of execution configuration, outcome, signals and evidence.

**PatternSignal** — typed indication that operational history may justify investigation/improvement.

**ImprovementProposal** — proposal to change a versioned decision-support artifact based on evidence/patterns.

**ExperimentCandidate** — immutable content-addressed variant evaluated in isolation.

**EvaluationContract** — predeclared quality gates, datasets/holdout rules, metrics, budget and promotion requirements.

**CandidateEvaluation** — evidence produced by evaluating a candidate under an EvaluationContract.

**PromotionDecision** — governed decision to reject, shadow, promote or revert a candidate.

**Run-scoped adaptation** — validated change affecting only one WorkflowRun; not durable unless later promoted.

**Candidate lineage** — projection of parent/descendant relationships among experiment candidates.

**Governed Continuous Improvement (GCI)** — SDDK process turning operational experience into experimentally validated, reversible improvements.

GCI is not autonomous self-modification. Engineering Assurance is not Rust-specific. ExperienceEpisode is not a source of truth. Candidate lineage is not a Darwinian kernel requirement.

## DecisionSnapshot
Bounded deterministic view of current state needed to plan/execute a semantic goal.

## Goal
Desired SDDK invariant/state, not a shell macro.

## GoalPlan
Validated dependency plan for satisfying a Goal.

## GoalRun
Durable reconciled execution of a Goal.

## OperationContract
Machine-readable contract describing an operation's requirements, outputs, effects, idempotency and completeness obligations.

## GoalResult
Concise index over a completed GoalRun's state, reports, evidence, receipts and metrics.

## Behavioral Parity Fixture
Contract proving a high-level goal preserves the mandatory behavior/outputs of a migrated low-level sequence.

## Agent Interface Entropy
Operational heuristic describing tool-trajectory dispersion/confusion for a normalized goal. Not a formal information-theory claim.

## Semantic compression
Reducing interface calls/instructions while preserving full workflow obligations and outputs.
