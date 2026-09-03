# Implementation Backlog — Ordered

## Epic EA — Engineering Assurance

- **EA-001 Pack contract:** manifest, capability IDs, schemas, fixture corpus.
- **EA-002 Deterministic adjudicator:** PASS/PW/FAIL/INCONCLUSIVE, stale evidence, scoped waiver.
- **EA-003 Profile resolver:** deterministic manifests, polyglot scope, generic fallback.
- **EA-004 Rust profile:** reuse `rust-patterns`; small delta skill; compiler/clippy/test; optional Miri/fuzz/Kani.
- **EA-005 SDD bridge:** normalize verify/debt evidence; map ChangeContract obligations; deduplicate analyzers.
- **EA-006 Active Graph projection:** assessment/finding/obligation/evidence.
- **EA-007 Cockpit views:** Assurance Report + Invariant Map.

## Epic GCI — Governed Continuous Improvement

- **GCI-001 Configuration identity:** record exact versions/hashes for skill, prompt, agent manifest, routing, context, workflow, verifier.
- **GCI-002 ExperienceEpisode projection.**
- **GCI-003 Pattern signals:** repeated failure, route regression, skill staleness, recurring correction, handoff entropy, workflow over-expansion.
- **GCI-004 Candidate artifact model:** content-addressed versions + parent refs.
- **GCI-005 EvaluationContract:** quality gates, dev/holdout split, metrics, budget.
- **GCI-006 Laboratory candidate runner:** fork/worktree isolation, apply/run/diff/evidence.
- **GCI-007 Multi-objective comparator:** hard gates + Pareto view.
- **GCI-008 Promotion lifecycle:** eligibility, policy, optional approval, shadow, rollout, revert receipt.
- **GCI-009 Skill curator:** usage, staleness, duplicate signal, archive candidate, consolidation proposal.
- **GCI-010 Reflection candidate generator:** one selected-trace provider.
- **GCI-011 Optional optimizer SPI:** future GEPA/MCTS/population providers outside kernel.

## Explicitly not in backlog

Scientific Theory/Hypothesis ontology, generic paper-writing researcher, weight training, self-rewriting kernel, unbounded recursive delegation, automatic codebase mutation from experience.

## Epic AFI — Agent-First Deterministic Interface

### AFI-001 Current interface telemetry
- normalize tool/CLI call records;
- baseline help/invalid/retry/repeated-read metrics;
- identify top trajectories by goal.

### AFI-002 Application-service extraction
Move deterministic use cases out of Clap handlers where needed.

CLI becomes adapter.

### AFI-003 DecisionSnapshot
One bounded state/query surface.

### AFI-004 GoalDefinition / GoalRegistry
Include state, verification, report, receipt and knowledge obligations.

### AFI-005 OperationContract registry
Declare requires/produces/effects/idempotency/cache/report obligations.

### AFI-006 Deterministic GoalPlanner v1
Dependency/obligation resolution without LLM.

### AFI-007 GoalRun reconciler
Resume/retry/postcondition semantics.

### AFI-008 GoalResult completeness
Aggregate refs without losing detailed artifacts.

### AFI-009 Behavioral parity harness
Legacy sequence vs high-level goal.

### AFI-010 Semantic Agent Tool API
`state`, `goal.plan`, `goal.apply`, `query`, `evidence.submit`.

### AFI-011 Tool Schema Compiler
Canonical contracts → JSON schema/tool docs/CLI help.

### AFI-012 Persistent stdio adapter spike
Evaluate whether hot process materially reduces overhead.

### AFI-013 Work-avoidance cache
Only for operations with safe declared semantics.

### AFI-014 Tool-use process mining
Frequent sequence/confusion analysis.

### AFI-015 GCI interface optimization
Interface candidates evaluated via Workflow Laboratory.

## AFI Definition of Done

A migration is not successful merely because it reduces CLI calls.

It must show:

```text
100% mandatory obligation closure
100% required report/receipt parity or explicit compatible successor
no safety/gate regression
lower interaction overhead
```
