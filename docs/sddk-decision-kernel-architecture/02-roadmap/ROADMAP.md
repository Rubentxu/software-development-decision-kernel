# SDDK Roadmap — Canonical Evolution Line

> **Status:** canonical planning view
> **Baseline:** `main` / v1.70.0 as reconciled on 2026-09-03
> **Rule:** horizons define build order; epics define capabilities; cycles are execution instances only.

## 1. Purpose

This document is the official roadmap for SDDK. It intentionally separates:

1. **North Star phases** — long-term product direction.
2. **Active horizons H0-H6** — current dependency-ordered implementation sequence.
3. **Capability epics** — stable semantic work identifiers tracked in `BACKLOG.md`.
4. **Cycles/runs** — concrete execution records. Cycle numbers are not roadmap identities.

Historical evolution packs and old cycle plans remain useful as design evidence, but they are not independent competing roadmaps. Their current disposition is recorded in [`EVOLUTION-CROSSWALK.md`](./EVOLUTION-CROSSWALK.md).

## 2. Sources of truth

Planning and execution must not be inferred from one document alone.

| Concern | Authoritative source |
|---|---|
| Long-term direction | this `ROADMAP.md` |
| Active capability status and dependency | `BACKLOG.md` |
| Historical release truth | `CHANGELOG.md` + git tags/commits |
| Concrete execution evidence | cycle artifacts / receipts / ledgers |
| Future machine-readable planning truth | Planning Ledger introduced in H1 |
| Design rationale | specs, ADRs, evolution dossiers |

When sources disagree, released behavior and execution evidence win over stale planning prose. The planning documents must then be reconciled.

## 3. Status vocabulary

Use only these planning states:

- `PROPOSED` — useful idea, not yet admitted to the active dependency chain.
- `READY` — dependencies satisfied and acceptance contract exists.
- `ACTIVE` — currently being executed.
- `BLOCKED` — admitted but dependency or decision prevents execution.
- `PARTIAL` — meaningful substrate is shipped but capability is incomplete.
- `SHIPPED` — acceptance contract satisfied in released code.
- `ABSORBED` — original proposal's intent is implemented through newer abstractions/work.
- `SUPERSEDED` — replaced by a newer design and must not drive implementation.

## 4. Current baseline

The v1.70.0 line already includes important foundations that older evolution packs still describe as future work:

- state-driven active-cycle context inference;
- graph-aware `cycle next` over declared workflow YAML;
- actionable recovery/error contracts;
- cycle pause/resume with `CycleStatus::Paused`, lease/fencing behavior and typed receipts;
- first-class facade/project-input behavior and parity work from earlier releases;
- Workflow IR, graph revision, replan and operator substrates that make dynamic execution feasible.

Therefore the next work does **not** restart the old AFI, lifecycle, state-driven CLI, or Human-Agent plans from zero.

## 5. Official active sequence

```text
H0 Reconcile & Harden
        ↓
H1 Planning SSOT
        ↓
H2 Generated Workflow MVP
        ↓
H3 Decision Plane
        ↓
H4 Human & Reactive Control
        ↓
H5 Runtime Completeness & Workflow Lab
        ↓
H6 Assurance & Governed Learning
```

No later horizon should bypass the exit gate of an earlier horizon unless an ADR records why the dependency is false.

---

## H0 — Reconcile & Harden

**Goal:** make the repository internally truthful before adding another major runtime capability.

### Scope

- Reconcile `ROADMAP.md`, `BACKLOG.md`, release history and current code status.
- Adopt semantic Work Item IDs instead of reusing historical cycle numbers as conceptual identifiers.
- Harden Workflow IR contracts needed by the generated-runtime vertical slice:
  - replace stringly execution scope with a typed scope contract;
  - stabilize transition/predicate AST serialization;
  - define revision/hash/provenance invariants;
  - define operator input/output/error contracts;
  - validate deterministic serialization and replay assumptions.
- Reconcile Human-Agent authority assumptions before implementing new decision paths.
- Keep existing shipped behavior intact.

### Exit gate

- One canonical roadmap and backlog exist.
- Active work uses semantic IDs.
- No known planning item claims `PROPOSED` for behavior already released.
- Workflow IR contracts required by H2 are accepted and testable.

### Primary epics

`GOV-ROADMAP`, `DW-IR`, `HX-AUTHORITY`.

---

## H1 — Planning SSOT

**Goal:** stop using hand-edited roadmap prose as the executable planning state.

### Scope

Introduce the **Planning Ledger / Decision Ledger** as the machine-readable source for planned work and decisions.

Minimum model:

- stable Work Item ID;
- title/capability;
- status;
- dependencies;
- acceptance/evidence references;
- supersedes/absorbs relationships;
- decision/provenance entries;
- optional execution bindings to cycle/run IDs.

Desired read surfaces, after the data model is stable:

- `sddk roadmap status`
- `sddk roadmap next`
- `sddk roadmap blocked`
- `sddk roadmap graph`
- `sddk roadmap show <WORK_ITEM_ID>`

### Exit gate

A repository checkout can deterministically reconstruct the active planning graph without interpreting historical markdown narratives.

### Primary epic

`PLN-LEDGER`.

---

## H2 — Generated Workflow MVP

**Goal:** prove one narrow end-to-end generated workflow path before adding reactive intelligence.

### Required vertical slice

```text
input
  → NewWorkflowPlan / Workflow IR
  → validation + compilation
  → ExecutionGraphRevision
  → persisted WorkflowRun
  → bounded execution
  → durable state/events
  → deterministic replay
```

### Operator scope

Only the minimum useful set is admitted initially:

- `Sequence`;
- bounded `Parallel`;
- `Conditional` / deterministic gates.

`Map`, `Reduce`, advanced joins, Secretary and cognitive replanning are explicitly out of the MVP critical path.

### Exit gate

A generated bounded DAG can be persisted, executed, resumed/replayed and reconstructed with stable revision/provenance semantics.

### Primary epic

`DW-RUNTIME`.

---

## H3 — Decision Plane

**Goal:** make “what should happen next?” one coherent domain capability independent of whether the workflow was declared or generated.

### Scope

Unify the concepts currently spread across:

- declared workflow state;
- generated `WorkflowRun`/execution graph frontier;
- `cycle next`;
- policy;
- current-run projection;
- recovery hints;
- decision context comparable to the old `DecisionSnapshot` intent.

Introduce a stable `CurrentRunView`/decision projection that can be consumed by CLI, AgentHost, Human-Agent collaboration and later Secretary behavior.

### Exit gate

The next legal/available actions for both declared and generated runs are derived from persisted state + policy, not from hard-coded CLI sequences.

### Primary epics

`DEC-PLANE`, `HX-CURRENT-RUN`.

---

## H4 — Human & Reactive Control

**Goal:** add explicit human authority and bounded reactive assistance on top of the same Decision Plane.

### Human authority path

```text
Policy
  → CurrentRunView / decision context
  → ApprovalRequest or HumanDecisionRequest
  → HumanDecisionPort
  → immutable HumanDecision / receipt
  → authorized workflow transition
```

Required capabilities:

- explicit human decision contracts;
- risk-sensitive HITL policy;
- authority/provenance rules;
- semantic cold-start resume and rehydration;
- CLI/AgentHost parity for supported decisions.

### Reactive / Secretary path

Secretary is not a second orchestrator. It proposes bounded declarative actions against the same policy/authority model used for humans.

- deterministic L0 reactions first;
- policy-bounded L1 proposals;
- cognitive replan only where deterministic strategies cannot decide;
- every accepted action produces durable provenance/receipts.

### Exit gate

Human decisions and bounded reactive proposals can safely change workflow progression without introducing a parallel authority model.

### Primary epics

`HX-DECISION`, `HX-RESUME`, `RX-SECRETARY`.

---

## H5 — Runtime Completeness & Workflow Lab

**Goal:** complete advanced execution semantics only after the generated runtime and Decision Plane are stable.

### Scope

- complete `Map` semantics;
- complete `Reduce` semantics;
- `JoinAny` / `JoinAll` and explicit child-output semantics;
- durable child lifecycle and lineage;
- remove placeholder operator outputs;
- cross-tick replay and recovery invariants;
- workflow/runtime observability;
- Workflow Laboratory for replay, comparison, fork/ablation and promotion evidence.

### Exit gate

Advanced dynamic graphs have deterministic durable semantics, and alternative workflow strategies can be compared using stable measurements.

### Primary epics

`DW-OPERATORS`, `DW-REPLAY`, `LAB-WORKFLOW`.

---

## H6 — Assurance & Governed Learning

**Goal:** apply higher-order assurance and learning to a stable execution/event model rather than creating another execution engine.

### Engineering Assurance

Refine and implement the useful parts of the 2026-08-23 Engineering Assurance proposal as cross-cutting capabilities:

- assurance profiles/rules;
- evidence types and resolvers;
- deterministic evaluators;
- capability-specific gates;
- UAT and verification integration;
- policy/provenance outputs suitable for the Decision Plane.

Contracts may be specified earlier, but full runtime integration should target the H2-H5 architecture.

### Governed Continuous Improvement

Build learning only after event and replay semantics are stable:

- `ExperienceEpisode`-style projections;
- process mining over real workflow/run events;
- strategy comparison;
- bounded experiments;
- promotion/tuning with explicit evidence and rollback.

### Exit gate

SDDK can measure, compare and safely evolve its workflows without allowing learned behavior to bypass deterministic policy, evidence or human authority.

### Primary epics

`EA-ASSURANCE`, `GCI-LEARNING`.

---

## 6. North Star phases

The existing product direction remains valid, but these phases are **not** the immediate execution queue. H0-H6 above determine build order.

| Phase | North Star capability |
|---|---|
| 0 | Baseline & architecture ratchet |
| 1 | Hexagonal convergence |
| 2 | Canonical Event Ledger |
| 3 | Workflow Runtime core |
| 4 | Dynamic workflow engine |
| 5 | AgentHost + provider resilience |
| 6 | Reactive behaviors + Supervisor/Secretary |
| 7 | Context Compiler |
| 8 | Adaptive SDD |
| 9 | Workflow Laboratory |
| 10 | Active Graph + causal/`why` views |
| 11 | Static Cockpit/control-plane projections |
| 12 | UAT bounded context / pack |
| 13 | Multi-pack proof on common runtime |
| 14 | Supply-chain, policy and production hardening |

Horizons may span several North Star phases because they represent dependency order, not product taxonomy.

## 7. Cycle identity policy

Historical cycle-number collisions have made roadmap interpretation unsafe. From this point:

- capability work is identified by semantic Work Item IDs such as `DW-RUNTIME-001`;
- a cycle/run ID records **execution**, not conceptual identity;
- one Work Item may require multiple cycles;
- one cycle may close multiple small Work Items when their acceptance contracts permit it;
- documentation must refer to the Work Item ID first and cycle/run second;
- historical `cycle-N` references remain historical and are not renumbered.

## 8. Promotion rules

1. Do not make Supervisor/Secretary smarter before dynamic graph execution is durable and policy-readable.
2. Do not implement advanced operators before the minimal generated WorkflowRun vertical slice is stable.
3. Do not run process mining/learning over event semantics that are still being redesigned.
4. Do not duplicate authority: human decisions, agents and Secretary proposals must converge on the same Decision Plane and policy model.
5. Do not remove the existing stable workflow path merely because an adaptive/generated path is cheaper. Promotion requires non-inferior invariant/evidence coverage and bounded comparison evidence.
6. Prefer `ABSORBED` or `SUPERSEDED` over reimplementing an old proposal whose intent already exists under newer abstractions.

## 9. Historical material

Detailed historical cycle narratives remain recoverable from git history, release notes, cycle artifacts and evolution dossiers. They are intentionally not duplicated in this canonical roadmap because doing so previously mixed release history with forward planning.

See:

- [`BACKLOG.md`](./BACKLOG.md)
- [`EVOLUTION-CROSSWALK.md`](./EVOLUTION-CROSSWALK.md)
- [`../../evolutivo-workflows-dinamicos-integracion-roadmap.md`](../../evolutivo-workflows-dinamicos-integracion-roadmap.md)
- [`../../SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/STATUS.md`](../../SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/STATUS.md)
- [`../../sddk-complete-evolution-2026-08-23/STATUS.md`](../../sddk-complete-evolution-2026-08-23/STATUS.md)
