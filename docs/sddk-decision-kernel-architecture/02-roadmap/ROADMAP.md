# SDDK Roadmap — Canonical Evolution Line to GA

> **Status:** canonical human planning view
> **Baseline:** `main` / v1.70.0 as reconciled on 2026-09-03
> **Machine execution source:** [`EXECUTION-SPINE.yaml`](./EXECUTION-SPINE.yaml)
> **Agent procedure:** [`AGENT-EXECUTION-PROTOCOL.md`](./AGENT-EXECUTION-PROTOCOL.md)

## 1. Purpose

This document describes **why** SDDK evolves in a particular order and what each horizon must achieve.

It is deliberately not the source from which an agent guesses the next cycle. Exact execution order lives in `EXECUTION-SPINE.yaml`.

The planning hierarchy is:

1. **Product goal / GA terminal condition** — what complete means.
2. **Horizons H0-H12** — dependency-ordered capability stages.
3. **Semantic Work Items** — stable implementation identities.
4. **Cycles/runs** — concrete execution attempts bound to one semantic Work Item.

Historical cycle numbers are evidence only. They are never feature identities.

## 2. Sources of truth

| Concern | Authoritative source |
|---|---|
| Exact next-work selection | `EXECUTION-SPINE.yaml` |
| Agent selection/governance algorithm | `AGENT-EXECUTION-PROTOCOL.md` |
| Horizon intent and exit gates | this `ROADMAP.md` |
| Capability descriptions/status context | `BACKLOG.md` |
| Historical proposal mapping | `EVOLUTION-CROSSWALK.md` + pack `STATUS.md` files |
| Released truth | `CHANGELOG.md` + git tags/commits + executable tests |
| Concrete execution evidence | cycle/run artifacts, receipts and ledgers |
| Future planning SSOT | Planning Ledger introduced in H1; it will project/update this plan rather than replace semantic identities |
| Design rationale | accepted specs and ADRs |

When planning prose conflicts with released behavior or durable execution evidence, execution evidence wins and planning state must be reconciled.

## 3. Deterministic continuation rule

An LLM/agent must not choose work by reading the newest design document.

The canonical rule is:

```text
if exactly one Work Item is ACTIVE:
    resume it
else:
    scan EXECUTION-SPINE.yaml by ascending order
    choose the first non-terminal item whose dependencies are terminal

if selected item is BLOCKED:
    stop; do not skip it

bind one concrete cycle/run to the semantic Work Item
execute until its exit_gate has evidence
mark terminal
recompute next
```

The detailed procedure is normative in `AGENT-EXECUTION-PROTOCOL.md`.

## 4. Status vocabulary

Use only:

- `PROPOSED` — admitted in the future line but not yet dependency-ready.
- `READY` — dependencies satisfied and acceptance contract exists.
- `ACTIVE` — currently executing.
- `BLOCKED` — cannot proceed; canonical line stops unless the plan is governed/revised.
- `PARTIAL` — meaningful substrate exists but capability is incomplete.
- `SHIPPED` — acceptance contract is satisfied by released/evidenced implementation.
- `ABSORBED` — intent was delivered through a newer abstraction/capability.
- `SUPERSEDED` — intentionally replaced and must not drive new implementation.

Terminal states are `SHIPPED`, `ABSORBED`, `SUPERSEDED`.

## 5. Current baseline

v1.70.0 already contains foundations that older evolution packs still describe as future work:

- state-driven active-cycle context inference;
- graph-aware `cycle next` over declared workflow YAML;
- actionable recovery/error contracts;
- cycle pause/resume with `CycleStatus::Paused`, leases/fencing and typed receipts;
- first-class facade/project-input behavior and earlier parity work;
- Workflow IR, graph revision, replan and operator substrates.

Therefore the line does **not** restart AFI, State-Driven CLI, lifecycle flexibility or Human-Agent work from zero.

## 6. Official path to GA

```text
H0  Reconcile & Deterministic Foundations
 ↓
H1  Planning SSOT
 ↓
H2  Generated Workflow MVP
 ↓
H3  Decision Plane
 ↓
H4  AgentHost & Context Compiler
 ↓
H5  Human & Reactive Control
 ↓
H6  Runtime Completeness & Workflow Lab
 ↓
H7  Engineering Assurance & UAT
 ↓
H8  Adaptive SDD
 ↓
H9  Active Graph & Cockpit
 ↓
H10 Governed Continuous Improvement
 ↓
H11 Multi-pack Proof
 ↓
H12 Supply Chain, Production Hardening & GA
```

No later horizon bypasses an earlier horizon gate unless an ADR/explicit planning decision proves the dependency is invalid and updates the execution spine.

---

## H0 — Reconcile & Deterministic Foundations

**Goal:** make planning, Workflow IR, authority boundaries, architecture boundaries and replay/event assumptions truthful before building a new persisted runtime path.

### Canonical semantic sequence

`GOV-ROADMAP-001 → DW-IR-001 → DW-IR-002 → DW-IR-003 → DW-IR-004 → DW-IR-005 → HX-AUTHORITY-001 → ARCH-HEX-001 → EVT-LEDGER-001`

### Exit gate

- one canonical roadmap/backlog/spine exists;
- semantic IDs are used for capability identity;
- typed Workflow IR contracts required by H2 are deterministic and versioned;
- human/agent/Secretary authority assumptions are explicit;
- only architecture-boundary debt that blocks H1-H3 is closed;
- canonical event/replay contracts required by persisted workflow runs are stable and tested.

---

## H1 — Planning SSOT

**Goal:** make planning state machine-readable and deterministic so agents no longer interpret hand-written roadmap prose.

### Canonical semantic sequence

`PLN-LEDGER-001 → PLN-LEDGER-002 → PLN-LEDGER-003 → PLN-LEDGER-004`

### Required capability

Planning Ledger / Decision Ledger models:

- stable semantic Work Item identity;
- dependency graph;
- status transitions;
- acceptance/evidence references;
- supersedes/absorbs relations;
- decision/provenance records;
- cycle/run execution bindings;
- deterministic projections such as `status`, `next`, `blocked`, `show`, `graph`.

### Exit gate

A clean checkout computes the same active/next planning graph without interpreting historical narratives.

---

## H2 — Generated Workflow MVP

**Goal:** prove one narrow persisted generated-workflow vertical slice before reactive intelligence or advanced operators.

### Canonical semantic sequence

`DW-RUNTIME-001 → DW-RUNTIME-002 → DW-RUNTIME-003 → DW-RUNTIME-004 → DW-RUNTIME-005`

### Vertical slice

```text
NewWorkflowPlan / Workflow IR
  → validation + deterministic compilation
  → ExecutionGraphRevision
  → persisted WorkflowRun
  → Sequence / Conditional / bounded Parallel
  → durable node/run state + receipts
  → resume + deterministic replay
```

Map, Reduce, advanced Join, Secretary and cognitive replanning remain outside this MVP.

### Exit gate

A bounded generated DAG can be persisted, executed, stopped, resumed and replayed to equivalent state with stable revision/provenance identity.

---

## H3 — Decision Plane

**Goal:** make “what can/should happen next?” one coherent domain capability for declared and generated workflows.

### Canonical semantic sequence

`DEC-PLANE-001 → DEC-PLANE-002 → DEC-PLANE-003 → DEC-PLANE-004`

### Scope

Unify:

- declared workflow state;
- generated run frontier;
- `cycle next` intent;
- recovery hints;
- policy;
- CurrentRun projection;
- explainable decision context/provenance.

### Exit gate

Available actions derive from persisted state + policy for both declared and generated workflows, and CLI consumes that same semantic contract.

---

## H4 — AgentHost & Context Compiler

**Goal:** make the canonical runtime/Decision Plane usable efficiently by agents without shell-command discovery loops or hidden conversational state.

### Canonical semantic sequence

`AGENT-HOST-001 → AGENT-HOST-002 → CTX-COMPILER-001 → CTX-COMPILER-002`

### Scope

- semantic AgentHost tool surface;
- provider failure classification, health/failover and usage telemetry;
- context capsules/deltas;
- staleness and negative knowledge;
- CurrentRun/recovery context for cold starts.

### Exit gate

A fresh agent can inspect and continue work through semantic tools with bounded, provenance-aware context and provider-resilient execution.

---

## H5 — Human & Reactive Control

**Goal:** add explicit human authority and bounded reactive assistance on the same Decision Plane.

### Canonical semantic sequence

`HX-DECISION-001 → HX-DECISION-002 → HX-RESUME-001 → RX-SECRETARY-001 → RX-SECRETARY-002 → RX-SECRETARY-003`

### Human authority path

```text
Policy
  → CurrentRunView / decision context
  → HumanDecisionRequest / ApprovalRequest
  → HumanDecisionPort
  → immutable decision + receipt
  → authorized workflow transition
```

### Reactive path

Secretary is not a second orchestrator:

- L0 deterministic reactions first;
- L1 bounded closed-set proposals second;
- cognitive replan only after deterministic strategies cannot decide;
- all accepted actions traverse the same policy/authority path and produce durable provenance.

### Exit gate

Human and reactive decisions can safely progress/recover workflows without a parallel authority model and cold-start resume does not depend on chat memory.

---

## H6 — Runtime Completeness & Workflow Lab

**Goal:** complete advanced operator semantics and create the empirical promotion environment after the MVP runtime and control planes are stable.

### Canonical semantic sequence

`DW-OPERATORS-001 → DW-OPERATORS-002 → DW-OPERATORS-003 → DW-OPERATORS-004 → DW-REPLAY-001 → LAB-WORKFLOW-001 → LAB-WORKFLOW-002`

### Scope

- durable child outputs and lineage;
- complete Map;
- complete Reduce;
- JoinAny/JoinAll;
- graph/node/depth/concurrency/budget guards;
- advanced replay and partial recovery;
- stable runtime metrics;
- fork/ablation, strategy comparison and promotion/shadow evidence.

### Exit gate

Advanced dynamic graphs have durable deterministic semantics and alternative workflow strategies can be compared reproducibly.

---

## H7 — Engineering Assurance & UAT

**Goal:** turn quality/acceptance expectations into typed, reproducible capabilities on the canonical runtime.

### Canonical semantic sequence

`EA-ASSURANCE-001 → EA-ASSURANCE-002 → UAT-BC-001 → EA-UAT-001`

### Scope

- assurance profiles/rules;
- evidence taxonomy and resolvers;
- deterministic evaluators;
- UAT scenario/human-check/defect/retest/signoff lifecycle;
- Decision Plane gates backed by assurance/UAT evidence.

### Exit gate

Workflow progression can be gated by deterministic assurance and explicit human UAT evidence without a separate execution engine.

---

## H8 — Adaptive SDD

**Goal:** deliver the adaptive SDD path only after the generic dynamic runtime, Decision Plane, lab and assurance foundations exist.

### Canonical semantic sequence

`SDD-ADAPTIVE-001 → SDD-ADAPTIVE-002 → SDD-ADAPTIVE-003 → SDD-ADAPTIVE-004 → SDD-ADAPTIVE-005`

### Scope

- ChangeContract + SHAPE;
- adaptive specialist selection;
- BUILD WorkGraph/WorkUnit mapping;
- CONVERGE + adaptive verification;
- INTEGRATE + legacy projections;
- empirical comparison against A-full.

### Promotion rule

Adaptive is not promoted merely because it is cheaper. Require non-inferior quality/invariant coverage and bounded rollout evidence from Workflow Lab.

### Exit gate

Representative simple and high-risk changes complete with required invariants/evidence, and promotion decision is evidence-backed.

---

## H9 — Active Graph & Cockpit

**Goal:** make SDDK's causal state inspectable and operationally understandable.

### Canonical semantic sequence

`GRAPH-WHY-001 → GRAPH-WHY-002 → COCKPIT-001 → COCKPIT-002`

### Scope

- typed graph projections for requirements, evidence, decisions, runs, artifacts, debt and lineage;
- causal `why` / `debt why` queries;
- overview/journal/timeline/execution graph;
- provider health, usage, assurance and experiment views.

### Exit gate

A user or agent can inspect both operational state and causal explanation without manually reconstructing it from logs/files.

---

## H10 — Governed Continuous Improvement

**Goal:** learn from stable execution semantics without allowing learned behavior to bypass governance.

### Canonical semantic sequence

`GCI-LEARNING-001 → GCI-LEARNING-002 → GCI-LEARNING-003`

### Scope

- ExperienceEpisode-style projections;
- process mining over canonical events;
- strategy quality/cost/risk comparison;
- bounded experiments;
- evidence-backed promotion/tuning;
- rollback and policy ratchets.

### Exit gate

SDDK can improve strategies from real evidence while deterministic policy, human authority and rollback remain mandatory boundaries.

---

## H11 — Multi-pack Proof

**Goal:** prove the kernel/runtime is genuinely generic rather than accidentally SDD-specific.

### Canonical semantic sequence

`MULTIPACK-001 → MULTIPACK-002 → MULTIPACK-003 → MULTIPACK-004`

### Scope

- lock generic pack contracts;
- run UAT on the same runtime/Decision Plane;
- run Incident on the same runtime/Decision Plane;
- architecture and end-to-end proof that pack-specific kernel branches are unnecessary.

### Exit gate

SDD, UAT and Incident operate through the same generic kernel/runtime contracts with no domain-specific kernel special cases.

---

## H12 — Supply Chain, Production Hardening & GA

**Goal:** turn the validated architecture into a production-grade stable release.

### Canonical semantic sequence

`SUPPLYCHAIN-001 → SUPPLYCHAIN-002 → PROD-HARDEN-001 → PROD-HARDEN-002 → GA-001 → GA-002`

### Scope

- SBOM/provenance and artifact lifecycle;
- signed gates and controlled overrides;
- performance, retention and migration hardening;
- reliability/security/upgrade/rollback/operator documentation;
- release-readiness matrix across SDD/UAT/Incident;
- stable compatibility contract.

### GA terminal condition

`GA-002` is terminal only when:

- representative canonical scenarios pass;
- no unresolved P0/P1 debt blocks release;
- security/recovery/provenance gates pass;
- supported upgrade/rollback paths have evidence;
- the GA release/tag exists;
- compatibility policy is documented.

After `GA-002`, this execution plan ends. Post-GA evolution starts from a new versioned plan rather than silently appending work to this one.

---

## 7. Relationship to the old North Star 0-14

The previous 0-14 direction remains valuable as architectural/product history, but the execution line above now places all of its remaining capabilities on an explicit path:

| Previous direction | Canonical horizon |
|---|---|
| baseline / architecture ratchet / hexagonal convergence | H0 |
| canonical event ledger | H0 |
| workflow runtime core + dynamic workflow engine | H2 + H6 |
| AgentHost/provider resilience | H4 |
| reactive Supervisor/Secretary | H5 |
| Context Compiler | H4 |
| adaptive SDD | H8 |
| Workflow Laboratory | H6 |
| Active Graph / `why` | H9 |
| Static Cockpit | H9 |
| UAT bounded context | H7 |
| multi-pack proof | H11 |
| supply chain / hardening | H12 |
| production/GA stabilization | H12 |

Nothing material is left as an unscheduled “retained epic”.

## 8. Historical evolution packs

The following are design dossiers, not parallel roadmaps:

- `docs/evolutivo-workflows-dinamicos-integracion-roadmap.md`
- `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/`
- `docs/sddk-complete-evolution-2026-08-23/`

Their current disposition is recorded in `EVOLUTION-CROSSWALK.md` and companion `STATUS.md` files.

Useful ideas may be re-admitted only by mapping them to a semantic Work Item on `EXECUTION-SPINE.yaml` or by a governed plan revision.

## 9. Current next work

At this branch state:

1. `GOV-ROADMAP-001` is `ACTIVE` because PR #1 is establishing this governance baseline.
2. After it becomes terminal, `DW-IR-001` is the deterministic next item.
3. Every subsequent item is already ordered to `GA-002` in `EXECUTION-SPINE.yaml`.

That is the official line an LLM should follow cycle by cycle.
