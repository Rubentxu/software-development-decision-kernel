# SDDK Execution Timeline — Cycle-by-Cycle Path to GA

> **Purpose:** human-readable chronological projection of `EXECUTION-SPINE.yaml`.
> **Normative order/status/dependencies:** `EXECUTION-SPINE.yaml`.
> **Context to load per Work Item:** `CYCLE-CONTEXT-MAP.yaml`.
> **Agent entry point:** `LLM-START-HERE.md`.

## 1. Temporal rule

The canonical roadmap is a **single dependency-ordered semantic line**.

By default:

```text
1 semantic Work Item = 1 bounded implementation cycle
```

If a Work Item is too large, it may use more than one concrete execution attempt only under `AGENT-EXECUTION-PROTOCOL.md`. The semantic Work Item keeps the same identity until its exit gate is satisfied or the plan is explicitly split before execution.

Concrete historical labels such as `cycle-72` are assigned at execution time and are never used to determine roadmap order.

The agent moves left-to-right / top-to-bottom only after the current Work Item is terminal with evidence.

## 2. Current bootstrap

At the time this execution line was created:

```text
CURRENT  → GOV-ROADMAP-001
NEXT     → DW-IR-001
FINAL    → GA-002
```

After PR #1 is merged, `GOV-ROADMAP-001` must be closed with merge/reconciliation evidence and the next computation should resolve to `DW-IR-001`.

---

## H0 — Reconcile & Deterministic Foundations

**Goal:** make planning, Workflow IR, architecture boundaries and event semantics trustworthy before building the new persisted runtime.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 10 | `GOV-ROADMAP-001` | Canonical roadmap/backlog/spine/context governance | `governance` |
| 20 | `DW-IR-001` | Typed execution scope | `dynamic-workflow-ir` |
| 30 | `DW-IR-002` | Versioned transition/predicate AST | `dynamic-workflow-ir` |
| 40 | `DW-IR-003` | Revision/hash/provenance invariants | `dynamic-workflow-ir` |
| 50 | `DW-IR-004` | Typed operator I/O/error contracts | `dynamic-workflow-ir` |
| 60 | `DW-IR-005` | IR/compiler determinism proof | `dynamic-workflow-ir` |
| 70 | `HX-AUTHORITY-001` | Human/agent/Secretary authority matrix | `human-authority` |
| 80 | `ARCH-HEX-001` | Close only architecture debt blocking H1–H3 | `architecture` |
| 90 | `EVT-LEDGER-001` | Canonical event/version/replay contract | `event-ledger` |

**H0 exit:** deterministic contracts required by planning persistence and generated runtime are accepted/tested; authority and architecture boundaries are explicit.

---

## H1 — Planning SSOT

**Goal:** replace hand-maintained planning interpretation with repository-reconstructible Planning Ledger state.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 100 | `PLN-LEDGER-001` | Planning Ledger domain/state model | `planning-ledger` |
| 110 | `PLN-LEDGER-002` | Deterministic repository persistence | `planning-ledger` |
| 120 | `PLN-LEDGER-003` | Bind semantic work to cycles/runs and migrate active planning | `planning-ledger` |
| 130 | `PLN-LEDGER-004` | `status/next/blocked/show/graph` projections | `planning-ledger` |

**H1 exit:** a clean checkout computes the same planning graph and same next Work Item without interpreting roadmap prose.

---

## H2 — Generated Workflow MVP

**Goal:** prove the narrow persisted generated-workflow vertical slice before advanced operators or smarter supervision.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 140 | `DW-RUNTIME-001` | `NewWorkflowPlan` → deterministic `ExecutionGraphRevision` | `dynamic-runtime` |
| 150 | `DW-RUNTIME-002` | Persist `WorkflowRun` identity/lifecycle | `dynamic-runtime` |
| 160 | `DW-RUNTIME-003` | Execute Sequence + Conditional | `dynamic-runtime` |
| 170 | `DW-RUNTIME-004` | Bounded Parallel + durable node/run state | `dynamic-runtime` |
| 180 | `DW-RUNTIME-005` | End-to-end replay + resume | `dynamic-runtime` |

**H2 exit:** generated bounded DAG persists, executes, stops, resumes and replays deterministically.

---

## H3 — Decision Plane

**Goal:** one answer to “what can/should happen next?” for declared and generated runs.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 190 | `DEC-PLANE-001` | Unified `CurrentRunView` | `decision-plane` |
| 200 | `DEC-PLANE-002` | Generic persisted-frontier next-action computation | `decision-plane` |
| 210 | `DEC-PLANE-003` | Typed policy, explanation and provenance | `decision-plane` |
| 220 | `DEC-PLANE-004` | CLI/recovery parity over Decision Plane | `decision-plane` |

**H3 exit:** legal actions derive from persisted state + policy, not CLI choreography.

---

## H4 — AgentHost & Context Compiler

**Goal:** let agents operate through stable semantic capabilities with explicit reconstructible context instead of discovering shell commands or depending on chat memory.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 230 | `AGENT-HOST-001` | Semantic AgentHost tool surface | `agent-first` |
| 240 | `AGENT-HOST-002` | Provider health/failure/failover/usage telemetry | `agent-first` |
| 250 | `CTX-COMPILER-001` | Capsules/deltas/staleness/negative knowledge | `agent-context` |
| 260 | `CTX-COMPILER-002` | Cold-start reconstruction from CurrentRunView/recovery state | `agent-context` |

**H4 exit:** a fresh agent can inspect and act semantically with bounded reconstructible context.

---

## H5 — Human & Reactive Control

**Goal:** add HITL and bounded reactive assistance without creating a second authority/orchestration model.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 270 | `HX-DECISION-001` | Human decision types + `HumanDecisionPort` | `human-collaboration` |
| 280 | `HX-DECISION-002` | Risk policy + CLI/AgentHost adapters | `human-collaboration` |
| 290 | `HX-RESUME-001` | `ResumeInfo` + `RehydrationPlan` + semantic cold start | `human-collaboration` |
| 300 | `RX-SECRETARY-001` | Deterministic L0 reactions | `secretary` |
| 310 | `RX-SECRETARY-002` | Bounded L1 Secretary proposals | `secretary` |
| 320 | `RX-SECRETARY-003` | Budgeted/policy-bounded cognitive replan | `secretary` |

**H5 exit:** human and Secretary actions share one policy/authorization/provenance path.

---

## H6 — Runtime Completeness & Workflow Lab

**Goal:** complete advanced operator semantics only after durable generated execution and control-plane decisions are stable.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 330 | `DW-OPERATORS-001` | Typed durable child output/lineage | `advanced-runtime` |
| 340 | `DW-OPERATORS-002` | Durable Map fan-out | `advanced-runtime` |
| 350 | `DW-OPERATORS-003` | Typed Reduce aggregation | `advanced-runtime` |
| 360 | `DW-OPERATORS-004` | JoinAny/JoinAll + runtime guards | `advanced-runtime` |
| 370 | `DW-REPLAY-001` | Advanced graph revision/replay/recovery proof | `advanced-runtime` |
| 380 | `LAB-WORKFLOW-001` | Stable runtime/workflow metrics | `workflow-lab` |
| 390 | `LAB-WORKFLOW-002` | Fork/ablation/strategy comparison/promotion evidence | `workflow-lab` |

**H6 exit:** advanced graphs are deterministic/replayable and alternative strategies can be compared reproducibly.

---

## H7 — Engineering Assurance & UAT

**Goal:** make evidence, assurance and human acceptance first-class consumers of the canonical runtime and Decision Plane.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 400 | `EA-ASSURANCE-001` | Assurance profiles/evidence taxonomy/rules | `engineering-assurance` |
| 410 | `EA-ASSURANCE-002` | Evidence resolvers + deterministic evaluators | `engineering-assurance` |
| 420 | `UAT-BC-001` | UAT lifecycle bounded context/pack | `uat` |
| 430 | `EA-UAT-001` | Gate Decision Plane with assurance/UAT evidence | `engineering-assurance` |

**H7 exit:** workflow progression can be deterministically gated by explainable engineering and user-acceptance evidence.

---

## H8 — Adaptive SDD

**Goal:** prove an adaptive SDD path while keeping A-full as the quality reference until evidence supports promotion.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 440 | `SDD-ADAPTIVE-001` | ChangeContract + SHAPE/adaptive specialist selection | `adaptive-sdd` |
| 450 | `SDD-ADAPTIVE-002` | BUILD WorkGraph/WorkUnit mapping | `adaptive-sdd` |
| 460 | `SDD-ADAPTIVE-003` | CONVERGE + adaptive verification | `adaptive-sdd` |
| 470 | `SDD-ADAPTIVE-004` | INTEGRATE + legacy projection/parity | `adaptive-sdd` |
| 480 | `SDD-ADAPTIVE-005` | Workflow Lab comparison and promotion decision | `adaptive-sdd` |

**H8 exit:** adaptive SDD is promoted only with non-inferior invariant/quality evidence.

---

## H9 — Active Graph & Cockpit

**Goal:** expose causal, operational and experimental state as projections of canonical evidence, never as alternate runtime truth.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 490 | `GRAPH-WHY-001` | Typed active-graph projections | `active-graph` |
| 500 | `GRAPH-WHY-002` | `why` / `debt why` causal queries | `active-graph` |
| 510 | `COCKPIT-001` | Overview/journal/timeline/execution graph views | `cockpit` |
| 520 | `COCKPIT-002` | Provider/usage/assurance/experiment views | `cockpit` |

**H9 exit:** users/agents can inspect current and causal state without manually reconstructing files/logs.

---

## H10 — Governed Continuous Improvement

**Goal:** learn from stable canonical execution evidence without allowing learning to bypass policy or authority.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 530 | `GCI-LEARNING-001` | ExperienceEpisode projection + process mining | `gci` |
| 540 | `GCI-LEARNING-002` | Bounded strategy experiments | `gci` |
| 550 | `GCI-LEARNING-003` | Evidence-backed promotion/tuning/rollback | `gci` |

**H10 exit:** workflow improvement is measurable, reversible and governed.

---

## H11 — Multi-pack Proof

**Goal:** prove SDDK is a generic kernel/runtime rather than an SDD-specific engine.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 560 | `MULTIPACK-001` | Lock generic pack contracts | `multipack` |
| 570 | `MULTIPACK-002` | UAT pack on canonical runtime | `multipack` |
| 580 | `MULTIPACK-003` | Incident pack on canonical runtime | `multipack` |
| 590 | `MULTIPACK-004` | Prove no pack-specific kernel special cases | `multipack` |

**H11 exit:** SDD, UAT and Incident share one generic runtime/Decision Plane contract.

---

## H12 — Supply Chain, Production Hardening & GA

**Goal:** make the proven architecture releasable, operable, secure and supportable as a stable product.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 600 | `SUPPLYCHAIN-001` | SBOM/provenance/artifact lifecycle | `supply-chain` |
| 610 | `SUPPLYCHAIN-002` | Signed gates/policy ratchets/controlled overrides | `supply-chain` |
| 620 | `PROD-HARDEN-001` | Performance/retention/migration/reliability | `production` |
| 630 | `PROD-HARDEN-002` | Security/upgrade/rollback/operator docs | `production` |
| 640 | `GA-001` | Full release-readiness matrix | `ga` |
| 650 | `GA-002` | Publish GA + freeze first stable compatibility contract | `ga` |

**Terminal condition:** `GA-002` is terminal with release and compatibility evidence.

---

## 3. Timeline invariants

The following are non-negotiable unless the canonical plan is explicitly revised:

1. Do not jump over the first non-terminal dependency-satisfied Work Item.
2. Do not execute a later horizon because its code looks easier or more interesting.
3. Do not resurrect an old evolution pack as a parallel timeline.
4. Do not reuse historical cycle numbers as semantic identity.
5. Do not pull advanced Map/Reduce/Join into H2.
6. Do not introduce Human/Secretary authority before the Decision Plane.
7. Do not introduce learning/process mining before stable runtime/replay/lab evidence.
8. Do not promote adaptive SDD without Workflow Lab evidence.
9. Do not claim generic-kernel maturity before multi-pack proof.
10. Do not claim GA before supply-chain, recovery, security and release-readiness gates pass.

## 4. How the agent uses this document

For the selected Work Item:

1. locate it here;
2. identify its predecessor, successor, horizon and context-pack key;
3. load that key from `CYCLE-CONTEXT-MAP.yaml`;
4. read the selected Work Item's exact dependency/objective/exit gate from `EXECUTION-SPINE.yaml`;
5. execute under `AGENT-EXECUTION-PROTOCOL.md`;
6. never use this Markdown table to override a newer evidenced status in the canonical Planning Ledger/spine.