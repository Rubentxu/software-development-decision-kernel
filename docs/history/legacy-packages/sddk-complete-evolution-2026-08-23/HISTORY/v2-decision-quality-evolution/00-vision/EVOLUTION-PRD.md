# Evolution PRD — Decision Quality as the next SDDK capability

**Status:** Proposed extension  
**Parent product:** SDDK — Software Development Decision Kernel

## Problem

SDDK already coordinates software-engineering workflows and is evolving toward invariant-driven workflows, canonical event history, dynamic WorkflowIR, capability routing, independent verification, fork/replay/diff, Workflow Laboratory, Active Graph and a pack microkernel.

Two gaps remain.

### Gap A — engineering reasoning is not yet a reusable first-class capability

Architecture, systems reasoning, performance analysis and verification are spread across agents, skills, SDD verification and debt analysis. The same reasoning should be reusable by SDD, UAT, Incident and Security while remaining outside the kernel.

### Gap B — SDDK records execution but only partially closes the learning loop

ADR-035 and SPEC-024 already define evaluation feedback and controlled promotion, but the architecture lacks a precise operational model for:

```text
execution history
→ reusable experience
→ improvement proposal
→ candidate experiment
→ evidence
→ promotion/rejection
```

Without this, learning risks becoming either manual intuition or uncontrolled agent self-editing.

## Product outcome

SDDK evolves from:

```text
govern decisions + record outcomes
```

to:

```text
govern decisions
+ verify them with reusable assurance
+ learn from outcomes
+ improve decision mechanisms through controlled experiments
```

The authority model remains unchanged:

```text
Git              = code authority
Event Ledger     = operational authority
Kernel runtime   = scheduling/governance authority
Packs            = bounded-context semantics
Agents           = proposal/reasoning providers
Tools            = evidence/effect providers
Graph/Cockpit    = rebuildable projections
```

## New jobs to be done

### JTBD-14 — Request engineering assurance semantically

Review architecture/change for the risks that actually apply without hard-coding a model, language or tool.

### JTBD-15 — Explain an engineering finding

Trace finding → invariant/obligation → evidence → source revision → decision.

### JTBD-16 — Learn from recurring execution failures

Detect repeated patterns and propose a measurable improvement instead of rediscovering the same problem.

### JTBD-17 — Compare alternative decision mechanisms safely

Compare skill, prompt, route, context or workflow variants on the same base without modifying the active system.

### JTBD-18 — Promote an improvement with evidence

A candidate becomes active only if quality is non-inferior, policy allows it and rollback remains possible.

## Product invariants

- **PI-1 Product-fit before novelty.** No feature enters because it is fashionable in agent research.
- **PI-2 No autonomous authority escalation.** Learning proposes changes; it never grants itself promotion/effect authority.
- **PI-3 Quality before efficiency.** Tokens/cost cannot override correctness, security, acceptance or required evidence.
- **PI-4 Experience is derived.** Experience/lineage/fitness are projections from canonical events/artifacts.
- **PI-5 Technology knowledge stays outside kernel.** Rust/Go/JVM/etc. are profiles/providers.
- **PI-6 Improvement is experimental and reversible.** Every durable improvement has baseline, evaluation contract, provenance and rollback.
- **PI-7 Unknown stays unknown.** Missing/stale required evidence means `INCONCLUSIVE`.

## In scope

- Engineering Assurance pack.
- Generic systems reasoning skill.
- Technology profile protocol.
- Evidence-backed findings/obligations.
- Deterministic assurance verdict.
- Experience projection from Event Ledger.
- Improvement proposals and versioned experiment candidates for skills, prompts, agent/provider manifests, routing policies, context strategies, workflow templates/strategies and verifier policies.
- Fork/replay/diff experiments.
- hidden/held-out evaluation where appropriate.
- multi-objective/Pareto comparison.
- shadow/bounded rollout and promotion/revert receipts.
- skill lifecycle/curation signals.
- graph/cockpit explanation.

## Explicitly out of scope

- generic scientific hypothesis/theory management;
- autonomous paper generation as a product goal;
- model-weight training/evolution;
- unrestricted self-edit of active SDDK code;
- recursive unbounded agent spawning;
- autonomous modification of kernel based on its own judge score;
- one universal fitness scalar;
- converting every event into an LLM reflection;
- adding an `autoresearch` pack solely because other agents have one.

## Success metrics

### Assurance

- evidence completeness;
- blocking findings with non-prose evidence;
- false-positive/waiver rate;
- accepted defect escape rate;
- assurance evidence reuse across packs;
- language profiles added with zero kernel changes.

### Improvement

- first-pass success delta versus baseline;
- invariant/evidence coverage delta;
- human correction delta;
- retry/convergence delta;
- tokens/cost/latency after quality constraints;
- candidate rejection and rollback rates;
- stale skill/config reduction;
- promoted changes with valid holdout/shadow evidence;
- duplicate experiment avoidance via history/graph reuse.

**Number of AI features is not a success metric.**
