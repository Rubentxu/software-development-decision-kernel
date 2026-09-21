# Roadmap Continuation — Decision Quality Evolution

**Status:** Proposed amendment to the current roadmap.

## Principle

Do not append a giant "AI self-improvement phase". Integrate two vertical slices into the roadmap already in motion.

## Slice A — Engineering Assurance

| Existing phase | Addition |
|---|---|
| 1 Hexagonal convergence | dogfood architecture obligations/fitness |
| 2 Event Ledger | `engineering.*` events |
| 3 Workflow Runtime | assurance workflows compile to existing IR |
| 4 Dynamic Engine | adaptive specialist dimensions |
| 5 AgentHost | route agent/tool/human reviewers |
| 6 Supervisor | propose extra review based on risk |
| 7 Context Compiler | minimal assurance capsules/staleness |
| 8 SDD Adaptive | SHAPE/CONVERGE assurance integration |
| 9 Workflow Laboratory | evaluate review depth/route quality |
| 10 Active Graph | finding/obligation/evidence causal graph |
| 11 Cockpit | assurance/invariant views |
| 12 UAT | technical evidence linkage |
| 13 Multi-pack | SDD/UAT/Incident reuse same capabilities |
| 14 Hardening | signed/high-risk evidence/provenance |

## Slice B — Governed Continuous Improvement

| Existing phase | Addition |
|---|---|
| 2 Event Ledger | complete metrics/config refs needed for learning |
| 4 Dynamic Engine | run-scoped validated adaptations only |
| 6 Reactive | deterministic pattern signals + bounded cognitive proposals |
| 7 Context Compiler | ExperienceEpisode/negative-knowledge capsules |
| 9 Workflow Laboratory | candidate lifecycle, holdout, Pareto, promotion evidence |
| 10 Active Graph | experience/candidate/lineage/evaluation projections |
| 11 Cockpit | experiment/candidate/promotion lenses |
| 13 Multi-pack | improvements evaluated across representative packs |
| 14 Hardening | promotion policy, shadow rollout, rollback, audit |

## Milestones

### M-EA0 — Contracts without new runtime

ADR/SPEC acceptance, compact skills, schemas/fixtures, no new kernel domain type.

### M-EA1 — Deterministic assurance slice

Pack manifest; finding/obligation/evidence model; deterministic verdict; replay fixtures.

### M-EA2 — Rust profile dogfood

Use SDDK itself. Targets include focused ports vs legacy Ledger, Event Envelope/Registry invariants, WorkflowIR determinism, projection replay, lease/liveness paths and architecture rules.

### M-EA3 — SDD integration

Current verify/debt evidence bridge; ChangeContract obligation mapping; no duplicate analyzers.

### M-GCI0 — Experience observability baseline

Record configuration refs, derive ExperienceEpisode, failure/human-correction fingerprints and baseline metrics. No autonomous proposals yet.

### M-GCI1 — Manual candidate laboratory

Support manually supplied skill/prompt/routing/context/workflow candidates using fork/replay/diff.

### M-GCI2 — Governed promotion

Holdout boundary, PromotionProposal, shadow, bounded rollout, revert and receipts.

### M-GCI3 — Assisted candidate generation

One conservative strategy:

```text
typed pattern signal → LLM reflection → candidate proposal
```

No population/MCTS yet.

### M-GCI4 — Optional search strategies

Only after GCI1–3 demonstrate value. Evaluate GEPA-like reflection, workflow search/MCTS and population/lineage selection separately.

### M-EA4/GCI5 — Graph + Cockpit

Expose assurance causal map, recurring-failure clusters, candidate lineages, experiment comparisons and promotion/revert history.

### M-EA5 — Multi-language / multi-pack proof

At least Rust + one non-Rust profile, and SDD + Incident or UAT consumer.

## Dependency rule

Do not start automated candidate search until Event data quality + experiment isolation + evaluation contracts + rollback are proven.

## Promotion to product default

Neither deep review nor learned workflow strategy becomes default merely because it is interesting or cheaper. Require measured non-inferior quality.
