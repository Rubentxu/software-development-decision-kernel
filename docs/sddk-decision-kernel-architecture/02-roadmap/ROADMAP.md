# SDDK Roadmap — Canonical Evolution Line to GA

> **Status:** canonical human planning view
> **Baseline:** `main` / v1.70.0 release baseline as reconciled on 2026-09-03; current workspace has advanced beyond it
> **Machine execution source:** [`EXECUTION-SPINE.yaml`](./EXECUTION-SPINE.yaml)
> **Human cycle timeline:** [`EXECUTION-TIMELINE.md`](./EXECUTION-TIMELINE.md)
> **Context routing:** [`CYCLE-CONTEXT-MAP.yaml`](./CYCLE-CONTEXT-MAP.yaml)
> **Agent procedure:** [`AGENT-EXECUTION-PROTOCOL.md`](./AGENT-EXECUTION-PROTOCOL.md)
> **Decision Memory model:** [`DECISION-MEMORY-GIT-MODEL.md`](./DECISION-MEMORY-GIT-MODEL.md)

## 1. Purpose

This document explains **why** SDDK evolves in a particular order and what each horizon must achieve.

It is deliberately **not** the source from which an agent guesses the next cycle. Exact order, dependencies and current status live in `EXECUTION-SPINE.yaml`.

Planning hierarchy:

1. **GA terminal condition** — what complete means.
2. **Horizons H0-H12** — dependency-ordered capability stages.
3. **Semantic Work Items** — stable capability identities.
4. **Cycles/runs** — concrete execution attempts bound to one semantic Work Item.

Historical `cycle-N` identifiers are execution evidence only, never capability identity.

## 2. Sources of truth

| Concern | Authoritative source |
|---|---|
| Exact next-work selection | `EXECUTION-SPINE.yaml` / Planning Ledger after H1 |
| Human chronology to GA | `EXECUTION-TIMELINE.md` |
| Context to load for a Work Item | `CYCLE-CONTEXT-MAP.yaml` |
| Agent selection/governance algorithm | `AGENT-EXECUTION-PROTOCOL.md` |
| Horizon intent and dependency rationale | this `ROADMAP.md` |
| Capability descriptions | `BACKLOG.md` |
| Historical proposal mapping | `EVOLUTION-CROSSWALK.md` + pack `STATUS.md` files |
| Released truth | `CHANGELOG.md` + git tags/commits + executable behavior/tests |
| Concrete execution truth | cycle/run artifacts, receipts and ledgers |
| Durable project knowledge | accepted vault requirements/ADRs/specs |
| Decision/deliberation history after H4 | Decision Memory projection over canonical evidence; never stronger authority than its sources |

When prose conflicts with released behavior or durable execution evidence, the evidence wins and planning must be reconciled.

## 3. Deterministic continuation rule

```text
if exactly one Work Item is ACTIVE:
    resume it
else:
    scan EXECUTION-SPINE.yaml by ascending order
    choose first non-terminal item whose dependencies are terminal

if selected item is BLOCKED:
    stop; never skip it silently

bind a concrete cycle/run
load the context pack
satisfy exit_gate with durable evidence
mark terminal
recompute next
```

Once H4 CDD/Decision Memory exists, session recovery additionally resolves canonical Decision Memory `HEAD`, computes staleness and renders a bounded `ResumeView`; it **does not** replace runtime/Planning Ledger reconstruction.

## 4. Status vocabulary

- `PROPOSED` — admitted future work, not yet acceptance-ready.
- `READY` — dependencies/acceptance contract permit execution.
- `ACTIVE` — currently executing.
- `BLOCKED` — canonical line stops until resolved/replanned.
- `PARTIAL` — meaningful substrate exists but capability is incomplete.
- `SHIPPED` — acceptance contract satisfied by evidenced implementation.
- `ABSORBED` — intent delivered through a newer capability.
- `SUPERSEDED` — intentionally replaced.

Terminal: `SHIPPED`, `ABSORBED`, `SUPERSEDED`.

## 5. Reconciled baseline

v1.70.0 already contains important substrate:

- state-driven active-cycle context inference;
- graph-aware `cycle next` over declared workflow YAML;
- actionable recovery/conflict hints;
- pause/resume with `CycleStatus::Paused`, leases/fencing and typed receipts;
- first-class facade/project-input and parity work;
- Workflow IR, graph revision, replan and operator substrates;
- artifact-first handoff discipline;
- separation between workflow state, durable vault knowledge and optional Engram memory.

After that release baseline, `DW-IR-001` was implemented on `main` as the typed/versioned/migration-safe `ExecutionScope` contract. The canonical spine therefore treats it as `SHIPPED` and inserts the generic test-impact foundation before `DW-IR-002`, rather than pretending already delivered work has not happened.

Older packs therefore remain evidence/design sources, not executable mega-roadmaps.

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
H4  AgentHost, Context Compiler & Decision Memory
 ↓
H5  Human & Reactive Control
 ↓
H6  Runtime Completeness, Decision Search & Workflow Lab
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

No later horizon bypasses an earlier gate unless an explicit governed plan revision proves the dependency invalid.

---

## H0 — Reconcile & Deterministic Foundations

**Goal:** make planning, **language-agnostic change-scoped verification**, Workflow IR, authority boundaries, architecture boundaries and replay/event assumptions truthful before adding new persistent runtime complexity.

The scoped-verification foundation is intentionally pulled in front of the **remaining** Workflow IR work because every later coding cycle otherwise pays repeated broad-test and test-discovery cost. `DW-IR-001` is preserved as already shipped. The capability must be generic from the start: Rust/Cargo is an adapter used by this repository, not a kernel assumption.

### Sequence

```text
GOV-ROADMAP-001
 → DW-IR-001                       # SHIPPED on current main
 → TEST-MODEL-001
 → TEST-ADAPTER-001
 → TEST-ADAPTER-002
 → TEST-SELECT-001
 → TEST-EVIDENCE-001
 → TEST-APPLY-001
 → DW-IR-002 → DW-IR-003 → DW-IR-004 → DW-IR-005
 → HX-AUTHORITY-001 → ARCH-HEX-001 → EVT-LEDGER-001
```

### H0.A Change-scoped verification foundation

The kernel works with semantic concepts:

```text
ActiveChangeSet
 → ProjectTestTopology
 → SUT Impact Graph
 → VerificationCapabilities
 → TestSelectionPlan
 → TestEvidenceReceipt
```

The model supports single-language, multi-module and polyglot repositories. Build/test ecosystems are adapters and may be composed in one repository. A cross-language change can propagate through API/schema/generated-code/runtime-contract edges without a language-specific planner branch.

Examples of adapter families include Cargo; Maven/Gradle; npm/pnpm/yarn and JS test runners; Python/pytest/tox/nox; Go; .NET; CMake/Meson/Bazel; and future ecosystems through the same SPI. Unsupported ecosystems have a generic explicit project profile/mapping fallback rather than forcing a kernel change.

Lifecycle invariant:

```text
apply  = smallest justified progressive verification evidence
verify = project's declared full verification profile
```

If impact cannot be justified, `apply` fails closed with the missing topology/test/capability relation; it does not hide uncertainty by running every test.

### H0 exit gate

- one canonical roadmap/backlog/spine/context map;
- stable semantic Work Item identities;
- generic change/SUT/test/capability contracts and adapter boundary;
- same impact/planner semantics proven across Rust, contrasting non-Rust fixtures and a polyglot/cross-language boundary;
- explainable progressive selection with durable evidence freshness/invalidation and escape-rate telemetry;
- coding agents consume scoped semantic verification instead of broad runner discovery loops;
- deterministic/versioned Workflow IR contracts;
- one explicit authority matrix for CLI/agent/human/Secretary;
- only architecture debt blocking H1-H3 is closed;
- canonical event/correlation/causation/replay contracts are stable and tested.

---

## H1 — Planning SSOT

**Goal:** make planning state machine-readable and deterministic so agents no longer interpret Markdown as executable state.

### Sequence

`PLN-LEDGER-001 → PLN-LEDGER-002 → PLN-LEDGER-003 → PLN-LEDGER-004`

### Scope

Planning Ledger models:

- semantic Work Item identity;
- dependency graph;
- status transitions;
- acceptance/evidence refs;
- supersedes/absorbs relations;
- planning decisions/provenance;
- cycle/run bindings;
- `status`, `next`, `blocked`, `show`, `graph` projections.

### Exit gate

A clean checkout computes the same planning graph and same next Work Item without interpreting historical prose.

---

## H2 — Generated Workflow MVP

**Goal:** prove one narrow persisted generated-workflow slice before reactive intelligence or advanced operators.

### Sequence

`DW-RUNTIME-001 → DW-RUNTIME-002 → DW-RUNTIME-003 → DW-RUNTIME-004 → DW-RUNTIME-005`

```text
NewWorkflowPlan
 → deterministic validation/compilation
 → ExecutionGraphRevision
 → persisted WorkflowRun
 → Sequence / Conditional / bounded Parallel
 → durable node/run state + receipts
 → resume + deterministic replay
```

Map/Reduce/full Join, Secretary and cognitive replanning remain outside this MVP.

### Exit gate

A bounded generated DAG persists, executes, stops, resumes and replays to equivalent state with stable provenance/revision identity.

---

## H3 — Decision Plane

**Goal:** one coherent domain answer to **what actions are legal/available next and why?** for declared and generated workflows.

### Sequence

`DEC-PLANE-001 → DEC-PLANE-002 → DEC-PLANE-003 → DEC-PLANE-004`

### Scope

Unify:

- declared workflow state;
- generated run frontier;
- `cycle next` intent;
- recovery hints;
- typed policy;
- CurrentRun projection;
- explainable decision/provenance.

### Exit gate

Available actions derive from persisted state + policy and CLI/recovery consume the same semantic contract.

---

## H4 — AgentHost, Context Compiler & Decision Memory

**Goal:** enable a fresh orchestrator/agent to continue work across sessions and agents with stable semantic tools, bounded reconstructible context, machine-enforced role boundaries, rich handoffs and a navigable durable history of decisions.

### Sequence

```text
AGENT-HOST-001
 → AGENT-HOST-002
 → CTX-COMPILER-001
 → CTX-COMPILER-002
 → CDD-ROLE-001
 → CDD-HANDOFF-001
 → CDD-HANDOFF-002
 → CDD-MEMORY-001
 → CDD-MEMORY-002
 → CDD-CONTINUE-001
```

### H4.A AgentHost

- semantic tool surface over planning/runtime/Decision Plane;
- no shell-command discovery loops for normal semantic work;
- provider health, failure classification, bounded failover and usage telemetry.

### H4.B Context Compiler

- context capsules and deltas;
- staleness;
- provenance;
- negative knowledge;
- cold-start context reconstructed from durable state rather than prior chat.

### H4.C CDD — Continuity, Delegation & Deliberation

#### Typed roles

`AgentRoleContract` makes explicit and machine-validatable:

- role kind: orchestrator/coordinator/leaf/evaluator/advisor;
- responsibilities;
- dispatch allowlist;
- mutation authority;
- tool/read/write scopes;
- input/output schemas;
- budgets;
- synthesis ownership;
- forbidden actions.

#### Rich handoff

Delegation moves from a small summary envelope to:

```text
DelegationRequest
 + immutable ContextLease
        ↓
AgentContributionEnvelope
        ↓
OrchestrationSynthesisReceipt
```

Contributions preserve coverage, findings, alternatives, rejected options, pros/cons, assumptions, uncertainty, risks, questions, evidence and artifact refs.

A synthesis receipt records what was consumed/omitted, conflicts, dissent and information-loss checks.

#### Git-like Decision Memory

The memory model is a content-addressed immutable DAG, inspired by Git semantics rather than a flat transcript:

```text
MemoryBlob / MemoryTree / MemoryCommit
                 +
parents + refs + HEAD + tags + reflog
                 ↓
DecisionMemoryProjection
```

Example:

```text
                         what-if/cache-B
                        o M7
                       /   \
canonical   M1---M2---M3---M6---M9 ← HEAD
                    \       /   \
                     M4---M5     M8
                  option-B     secretary/recovery
```

Important invariants:

- objects immutable after content-addressing;
- refs are cheap movable pointers;
- multiple parents are allowed for explicit synthesis/merge;
- a merge has a receipt and preserves conflict/dissent/evidence;
- `what-if`/`rejected` branches are advisory;
- only governed policy/authority may advance canonical `HEAD`;
- raw artifacts/contributions remain reachable;
- private model chain-of-thought is never required/persisted.

The LLM normally receives a bounded tree projection rather than the complete DAG:

```text
HEAD
├─ current goal
├─ current Work Item
├─ runtime frontier
├─ binding decisions
├─ active assumptions
├─ open risks/questions
├─ negative knowledge
├─ pending delegations
└─ continuation candidates
```

A session checkpoint becomes a ref/tag to a memory commit. `diff(previous_session, HEAD)` explains what changed without relying on a narrative summary.

### H4 exit gate

A fresh agent/session can:

1. reconstruct authoritative runtime/planning state;
2. resolve Decision Memory canonical `HEAD`;
3. render a bounded staleness-aware resume tree;
4. traverse/diff historical decision branches to original evidence;
5. recover typed delegated contributions and synthesis receipts;
6. propose continuation candidates with pros/cons, risks, reversibility and prerequisites;
7. do all of the above without previous chat history or Engram being mandatory.

---

## H5 — Human & Reactive Control

**Goal:** add explicit human authority and bounded reactive assistance using the exact same Decision Plane + CDD contracts.

### Sequence

`HX-DECISION-001 → HX-DECISION-002 → HX-RESUME-001 → RX-SECRETARY-001 → RX-SECRETARY-002 → RX-SECRETARY-003`

### Human path

```text
Policy
 → CurrentRunView / ResumeView
 → HumanDecisionRequest
 → HumanDecisionPort
 → immutable HumanDecision + receipt
 → Decision Memory commit
 → authorized workflow transition
```

`HX-RESUME-001` specializes human collaboration over the generic CDD ResumeView; it must not create a second continuity/memory model.

### Secretary path

Secretary is not another orchestrator:

- L0 deterministic reactions first;
- L1 bounded closed-set proposals second;
- cognitive replan only when deterministic paths cannot decide;
- proposal arrives through CDD Contribution/ContinuationCandidate;
- every accepted action traverses policy/authority and leaves durable provenance/Decision Memory;
- Secretary never directly moves canonical Decision Memory `HEAD`.

### Exit gate

Human and reactive decisions safely progress/recover workflows without parallel authority, memory or handoff models.

---

## H6 — Runtime Completeness, Decision Search & Workflow Lab

**Goal:** complete advanced operator semantics, stabilize empirical comparison, then evaluate richer decision-search strategies without contaminating deterministic core behavior.

### Sequence

```text
DW-OPERATORS-001 → DW-OPERATORS-002 → DW-OPERATORS-003 → DW-OPERATORS-004
 → DW-REPLAY-001
 → LAB-WORKFLOW-001 → LAB-WORKFLOW-002
 → LAB-DECISION-001 → LAB-DECISION-002
```

### Runtime completion

- durable child outputs/lineage;
- Map;
- Reduce;
- JoinAny/JoinAll;
- graph/depth/concurrency/budget guards;
- advanced replay/partial recovery.

### Workflow Lab

- stable quality/cost/latency/retry/handoff/failure metrics;
- fork/ablation;
- shadow/promotion evidence.

### Decision Lab

Core deterministic baseline:

1. legal frontier from Decision Plane;
2. policy filtering;
3. typed `ContinuationCandidate` scoring;
4. explicit risk/reversibility/evidence/uncertainty/cost dimensions;
5. Pareto frontier;
6. human escalation on material ambiguity/risk.

Then bounded experimental lookahead:

- Decision Memory branch/fork;
- beam/best-first baseline;
- depth/node/token/time budgets;
- explicit pruning receipts;
- counterfactual replay/evaluation;
- no mutation of canonical `HEAD`.

Finally evaluate Tree-of-Thoughts/Graph-of-Thoughts/MCTS/LATS-like strategies **as experiments only**. Promotion requires evidence of quality uplift plus bounded cost, stability, traceability, rollback and preserved policy/HITL.

### Exit gate

Advanced graphs are durable/replayable and alternative workflow/decision strategies can be compared reproducibly without weakening deterministic governance.

---

## H7 — Engineering Assurance & UAT

**Goal:** turn quality and user acceptance into typed reproducible evidence on the canonical runtime.

### Sequence

`EA-ASSURANCE-001 → EA-ASSURANCE-002 → UAT-BC-001 → EA-UAT-001`

Scope:

- assurance profiles/rules;
- evidence taxonomy/resolvers;
- deterministic evaluators;
- UAT scenario/human-check/defect/retest/signoff lifecycle;
- Decision Plane policy backed by assurance/UAT evidence.

**Exit:** workflow progression can be gated by deterministic assurance and explicit human UAT evidence without a second engine.

---

## H8 — Adaptive SDD

**Goal:** deliver adaptive SDD only after generic runtime, Decision Plane, CDD, Lab and assurance foundations exist.

### Sequence

`SDD-ADAPTIVE-001 → SDD-ADAPTIVE-002 → SDD-ADAPTIVE-003 → SDD-ADAPTIVE-004 → SDD-ADAPTIVE-005`

Scope:

- ChangeContract + SHAPE;
- adaptive specialist selection;
- BUILD WorkGraph/WorkUnit mapping;
- CONVERGE/adaptive verification;
- INTEGRATE/legacy projections;
- empirical comparison against A-full.

**Promotion rule:** never promote merely because it is cheaper; require non-inferior quality/invariant evidence. H8 may learn richer selection strategies, but it builds on the deterministic H0 SUT/topology/evidence foundation instead of creating another test-selection model.

---

## H9 — Active Graph & Cockpit

**Goal:** make operational, causal and deliberative state inspectable.

### Sequence

`GRAPH-WHY-001 → GRAPH-WHY-002 → COCKPIT-001 → COCKPIT-002`

Scope:

- typed graph projections over requirements/evidence/decisions/Decision Memory/delegations/runs/artifacts/debt/lineage;
- `why`, `debt why`, `decision why`;
- journal/timeline/execution graph;
- Decision Memory branch tree;
- provider/usage/assurance/handoff/experiment views.

H9 visualizes/query-projects the H4 Decision Memory. It must not create another graph database of authority.

**Exit:** a user/agent can inspect both state and causal/deliberative explanation without manually reconstructing files/logs.

---

## H10 — Governed Continuous Improvement

**Goal:** learn from stable execution and decision evidence without letting learned behavior bypass governance.

### Sequence

`GCI-LEARNING-001 → GCI-LEARNING-002 → GCI-LEARNING-003`

Scope:

- ExperienceEpisode projections;
- process mining over events + decision/delegation outcomes;
- rejected-branch/revisit-trigger analysis;
- strategy quality/cost/risk comparison;
- bounded experiments;
- evidence-backed promotion/tuning;
- rollback and policy ratchets.

**Exit:** SDDK can improve strategies from real evidence while deterministic policy, human authority and rollback remain mandatory.

---

## H11 — Multi-pack Proof

**Goal:** prove the kernel/runtime/CDD model is generic rather than accidentally SDD-specific.

### Sequence

`MULTIPACK-001 → MULTIPACK-002 → MULTIPACK-003 → MULTIPACK-004`

- lock generic pack contracts;
- run UAT on the same runtime/Decision Plane;
- run Incident on the same runtime/Decision Plane;
- prove no pack-specific kernel special cases.

---

## H12 — Supply Chain, Production Hardening & GA

**Goal:** turn the validated architecture into a production-grade stable release.

### Sequence

`SUPPLYCHAIN-001 → SUPPLYCHAIN-002 → PROD-HARDEN-001 → PROD-HARDEN-002 → GA-001 → GA-002`

Scope:

- SBOM/provenance/artifact lifecycle;
- signed gates/controlled overrides;
- performance/retention/migration hardening;
- reliability/security/upgrade/rollback/operator docs;
- release-readiness matrix across SDD/UAT/Incident;
- stable compatibility contract.

### GA terminal condition

`GA-002` is terminal only when representative canonical scenarios, security/recovery/provenance, supported upgrade/rollback and compatibility gates all have evidence and no unresolved P0/P1 debt blocks release.

After `GA-002`, this plan ends; post-GA work starts from a new versioned plan.

---

## 7. Relationship to previous architectural directions

| Previous direction | Canonical horizon |
|---|---|
| baseline / architecture ratchet / hexagonal convergence | H0 |
| language-agnostic SUT impact / scoped apply verification | H0 |
| canonical event ledger | H0 |
| workflow runtime core / generated execution | H2 + H6 |
| DecisionSnapshot/current-run intent | H3 |
| AgentHost/provider resilience | H4 |
| Context Compiler | H4 |
| session continuity / rich delegation / Decision Memory | H4 |
| human collaboration / Secretary | H5 |
| dynamic/counterfactual decision search | H6 |
| Workflow Laboratory | H6 |
| Engineering Assurance / UAT | H7 |
| adaptive SDD | H8 |
| Active Graph / Cockpit | H9 |
| GCI/process mining | H10 |
| multi-pack proof | H11 |
| supply chain / production / GA | H12 |

Nothing material should remain as an unscheduled “retained epic”.

## 8. Evolution dossiers

These are design/research dossiers, not parallel roadmaps:

- `docs/evolutivo-workflows-dinamicos-integracion-roadmap.md`
- `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/`
- `docs/sddk-complete-evolution-2026-08-23/`
- `docs/evolutivo-continuidad-sesiones-delegacion-deliberacion.md`

Their ideas become executable only when mapped to semantic Work Items in `EXECUTION-SPINE.yaml`.

## 9. Current next work

1. `GOV-ROADMAP-001`, `DW-IR-001` and `DW-IR-002` are `SHIPPED` (PR #1 at `2596e46`; ExecutionScope v1.71.0; transition/predicate AST v1.72.0).
2. `TEST-APPLY-001` is the deterministic CURRENT Work Item (evidence freshness/invalidation shipped v1.77.0); with it the TEST-* block closes and `DW-IR-003` resumes.
3. H0 then proceeds through adapter proof, selector, evidence/invalidation and apply/verify integration.
4. The complete route, including CDD/Decision Memory, remains placed through `GA-002` in `EXECUTION-SPINE.yaml`.

That is the official line an LLM must follow.