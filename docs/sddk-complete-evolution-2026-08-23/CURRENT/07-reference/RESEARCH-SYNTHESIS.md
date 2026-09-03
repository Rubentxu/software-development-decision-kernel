# Research Synthesis — Patterns Adopted, Deferred and Rejected

**Purpose:** explain why external autoresearch/self-improvement ideas do or do not belong in SDDK.

## Hermes / Nous

Sources:
- https://hermes-agent.nousresearch.com/docs/guides/delegation-patterns
- https://hermes-agent.nousresearch.com/docs/user-guide/features/curator
- https://github.com/NousResearch/hermes-agent-self-evolution

Useful: isolated subagent context, explicit delegation contracts, independent result verification, skill lifecycle/curation, offline candidate evolution.

Adopt: skill use/staleness signals, consolidation proposals, bounded isolated workers using existing WorkflowIR.

Do not adopt: direct "learn once → edit active skill" semantics or session-local background behavior as durable workflow authority.

## GEPA

Source: https://arxiv.org/abs/2507.19457

Pattern:

```text
rich traces → reflection → targeted candidate mutations → Pareto preservation
```

Adopt later as one candidate-generation provider. Reflection is not authoritative.

## AFlow

Source: https://arxiv.org/abs/2410.10762

Pattern: workflow optimization as search over topology/control flow using execution feedback.

SDDK adaptation: search over validated WorkflowTemplate/WorkflowIR candidates, never arbitrary generated scheduler code. Defer MCTS until simple candidate evaluation works.

## Darwin Gödel Machine

Source: https://arxiv.org/abs/2505.22954

Useful: preserve multiple candidate lineages, empirical validation, sandbox/human oversight.

Adopt: parent refs + lineage projection; optional population strategy.

Reject: self-modification of active kernel/runtime as product mechanism.

## Imbue Catalyst

Sources:
- https://imbue.com/blog/2026-07-20-imbue-catalyst-nanochat
- https://imbue.com/blog/2026-07-20-imbue-catalyst-theory-discovery

Useful lessons: linear search tunnels into early assumptions; diversity can help; excessive information sharing can collapse diversity.

Adopt abstractly: preserve alternative candidate branches and allow selective experiment context.

Reject for core roadmap: scientific Theory/Hypothesis ontology and autonomous theory-discovery product features.

## Google AI Co-Scientist

Source: https://research.google/blog/accelerating-scientific-breakthroughs-with-an-ai-co-scientist/

Useful: specialist generation/reflection/ranking/evolution and resource-aware supervisor. In SDDK these remain workflow compositions/capabilities, not kernel agent types.

## Anthropic multi-agent research

Source: https://www.anthropic.com/engineering/multi-agent-research-system

Lessons: orchestrator/worker is effective for breadth-first independent subtasks; coordination cost grows; workers need explicit goal/scope/output; independent exploration reduces path dependency.

Adopt: bounded parallelism and explicit WorkUnit contracts. Reject unbounded fan-out.

## HarnessOpt-Bench

Source: https://arxiv.org/abs/2608.06301

Strong alignment:
- optimize harness, not just model weights;
- hidden held-out evaluation;
- trusted evaluation boundary;
- preserve candidate versions;
- fixed evaluation budget.

Adopt strongly through EvaluationContract, holdout isolation, candidate preservation and audit trail.

## Community signal from Hermes self-evolution

Active project work includes GEPA integration correctness, semantic-preservation constraints, holdout integrity, statistical significance, persistence of optimized instructions and cost/audit trails.

Lesson:

> Self-improvement infrastructure needs stronger deterministic contracts than a normal prompt-edit loop.

## Final classification

### Adopt now
- evidence-backed assurance;
- skill lifecycle signals;
- ExperienceEpisode projection;
- configuration identity;
- manual candidate experiments;
- holdout isolation;
- quality-first comparison;
- governed promotion/revert.

### Adopt after baseline
- trace-reflection candidate generation;
- run-scoped adaptations;
- candidate lineages;
- Pareto frontier.

### Research only
- GEPA-like automated evolution;
- AFlow/MCTS WorkflowIR search;
- population/novelty/metaproductivity allocation.

### Reject from current product roadmap
- generic scientific autoresearch platform;
- theory discovery ontology;
- autonomous weight training;
- unrestricted self-rewriting kernel;
- infinite recursive agent trees.
