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
