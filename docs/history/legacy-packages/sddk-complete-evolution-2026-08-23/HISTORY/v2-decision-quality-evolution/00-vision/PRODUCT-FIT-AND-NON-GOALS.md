# Product Fit & Anti-Frankenstein Rules

## Purpose

Prevent SDDK from becoming a collection of unrelated agent features.

Every proposed capability passes the **SDDK Feature Admission Test** before entering an ADR or roadmap milestone.

## Feature Admission Test

Score each dimension `0`, `1` or `2`.

| Dimension | 0 | 1 | 2 |
|---|---|---|---|
| Product identity | incidental AI feature | useful to software workflows | directly strengthens decision governance |
| Existing JTBD | none | adjacent | directly improves one or more |
| Reuse of primitives | requires new subsystem | partial reuse | composes existing runtime |
| Authority clarity | unclear | mostly bounded | explicit authority + receipt |
| Domain placement | leaks into kernel | arguable | clean kernel/pack/skill/profile boundary |
| Evidence of value | intuition | plausible metric | testable experiment |
| Reversibility | difficult | possible | native fork/rollback |
| Complexity budget | broad | moderate | smallest useful slice |

### Admission rule

- `< 10`: reject.
- `10–12`: research/spike only.
- `13–16`: roadmap candidate.

No score overrides a product-invariant violation.

## Application to discussed ideas

### Engineering Assurance — ADMIT

Directly improves technical decisions and verification; reusable by existing packs; maps naturally to capability/evidence/event contracts; technology knowledge remains profiles.

### Generic `sddk-pack-research` — REJECT FOR CORE ROADMAP

Interesting, but not required by SDDK's primary jobs. Scientific hypothesis/theory ontologies would create a second product. External packs can implement them later using generic primitives.

### Generic `sddk-pack-evolution` — DEFER / DO NOT CREATE YET

The useful mechanism already belongs to:

```text
ADR-035 Evaluation Feedback
+ SPEC-024 Agent/Workflow Evaluation
+ SPEC-033 Fork/Replay/Diff
+ SPEC-040 Workflow Laboratory
```

Create a new pack only if a stable bounded context later emerges that cannot live cleanly in the laboratory/evaluation layer.

### Experience Graph — ADMIT AS PROJECTION

Useful for pattern detection, causal analysis and experiment reuse. It is not an authority or second database.

### Population/lineage search — CONDITIONAL

Useful as an optimization strategy after a simple baseline exists. Do not bake Darwinian/MCTS concepts into kernel types.

### GEPA-style trace reflection — CONDITIONAL

Useful for generating candidate mutations from rich traces. Reflection proposes candidates; evaluation decides.

### Hermes-style skill curator — ADMIT IN MINIMAL FORM

Track use, drift, duplication and staleness. Initial action is recommendation/archive candidate, not autonomous destructive deletion.

### Self-modifying active agent/kernel — REJECT

Violates authority, replay and rollback principles. Use run-scoped candidates or isolated forks.

### Scientific competing theory strands — REJECT AS PRODUCT MODEL

The diversity lesson is useful for experiments, but `Theory/Hypothesis` are not core software-decision entities. Use generic experiment candidate lineages instead.

## Rule of thumb

> Import **patterns**, not foreign product ontologies.
