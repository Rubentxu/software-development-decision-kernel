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

# Slice C — Agent-First Deterministic Interface

| Existing phase | Addition |
|---|---|
| 1 Hexagonal convergence | extract application services from CLI handlers |
| 2 Event Ledger | tool/goal usage events and complete configuration identity |
| 3 Workflow Runtime | Goal/Operation contracts and deterministic planning |
| 4 Dynamic Engine | GoalPlan may compile/use dynamic WorkflowIR |
| 5 AgentHost | semantic tool API, capability suspension/resume |
| 6 Reactive | typed blockers/recovery, no redundant LLM prompts |
| 7 Context Compiler | DecisionSnapshot + tool pruning + cache fingerprints |
| 9 Workflow Laboratory | legacy CLI sequence vs goal-surface parity/efficiency experiments |
| 10 Active Graph | GoalRun/Operation/ToolTrajectory projections |
| 11 Cockpit | goal completeness, reports, tool-use efficiency |
| 14 Hardening | parity, idempotency, holdout/tool-interface promotion policies |

## AFI milestones

### AFI-0 — Observe before simplifying

Instrument current tool/CLI usage.

Exit:

- call counts;
- help/invalid/retry rates;
- repeated-state reads;
- report set per workflow;
- baseline token/latency.

### AFI-1 — DecisionSnapshot

Introduce a single state snapshot without changing execution.

Exit:

- agents can replace repeated routine state reads;
- no report/gate behavior changes.

### AFI-2 — Goal Registry + `goal plan`

Read-only planning only.

Exit:

- planner produces the same required obligation set as selected workflow;
- no side effects.

### AFI-3 — One parity-proven `goal apply`

Choose one bounded workflow segment.

Recommended first candidate:

```text
cycle.verified
```

Run legacy sequence and goal path against golden fixtures.

Hard promotion gate:

```text
same/superset invariants
same/superset mandatory reports
same/superset receipts/evidence
same blocker semantics
```

Only interaction overhead may decrease.

### AFI-4 — Agent semantic tool surface

Expose state/goal/query/evidence tools through AgentHost/stdin adapter.

Do not remove low-level CLI.

### AFI-5 — Work avoidance

Add safe `UP_TO_DATE`/reuse semantics for read/cacheable operations.

### AFI-6 — Report/completeness aggregation

GoalResult indexes all outputs while Cockpit preserves detailed views.

### AFI-7 — Tool trajectory/process mining

Detect high-frequency command sequences and interface confusion.

Only produce GCI proposals.

### AFI-8 — Optional advanced planning

Evaluate schema-hypergraph/search strategies against deterministic planner v1.

Keep only if useful.
