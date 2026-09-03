# SDDK Execution Timeline — Cycle-by-Cycle Path to GA

> **Purpose:** human-readable chronological projection of `EXECUTION-SPINE.yaml`.
> **Normative order/status/dependencies:** `EXECUTION-SPINE.yaml`.
> **Context to load per Work Item:** `CYCLE-CONTEXT-MAP.yaml`.
> **Agent entry point:** `LLM-START-HERE.md`.

## 1. Temporal rule

The canonical roadmap is a dependency-ordered semantic line. By default:

```text
1 semantic Work Item = 1 bounded implementation cycle
```

A concrete historical label such as `cycle-72` is assigned at execution time and never determines roadmap order. The agent advances only after the current semantic Work Item reaches a terminal state with evidence.

After reconciling the 2026-09-03 scoped-verification reprioritization with current `main` (DW-IR-002 shipped v1.72.0 while the TEST-* block was landing):

```text
SHIPPED  → GOV-ROADMAP-001, DW-IR-001, DW-IR-002, TEST-MODEL-001, TEST-ADAPTER-001, TEST-ADAPTER-002
CURRENT  → TEST-SELECT-001
NEXT     → TEST-EVIDENCE-001
FINAL    → GA-002
```

`GOV-ROADMAP-001` and `DW-IR-001` are already `SHIPPED`. The new H0 verification foundation is inserted **after the work already delivered and before `DW-IR-002`**, so all remaining implementation cycles benefit from cheaper, more precise test feedback without rewriting history.

---

## H0 — Reconcile & Deterministic Foundations

**Goal:** make planning, language-neutral change-scoped verification, Workflow IR, authority, architecture boundaries and event semantics trustworthy before building persisted generated execution.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 10 | `GOV-ROADMAP-001` | Canonical roadmap/backlog/spine/context governance | `governance` |
| 20 | `DW-IR-001` | Typed execution scope — already shipped on `main` | `dynamic-workflow-ir` |
| 22 | `TEST-MODEL-001` | Language-neutral ActiveChangeSet + ProjectTestTopology + SUT graph model | `change-scoped-verification` |
| 23 | `TEST-ADAPTER-001` | Generic topology/test adapter SPI + capability registry + explicit fallback | `change-scoped-verification` |
| 24 | `TEST-ADAPTER-002` | Prove Rust + multiple non-Rust + polyglot adapter composability | `change-scoped-verification` |
| 25 | `TEST-SELECT-001` | Deterministic impact propagation + progressive batch selection | `change-scoped-verification` |
| 26 | `TEST-EVIDENCE-001` | Evidence receipts, freshness, invalidation, reuse + escape telemetry | `change-scoped-verification` |
| 27 | `TEST-APPLY-001` | Integrate scoped testing with apply/TDD/verify/agent contracts | `change-scoped-verification` |
| 30 | `DW-IR-002` | Versioned transition/predicate AST | `dynamic-workflow-ir` |
| 40 | `DW-IR-003` | Revision/hash/provenance invariants | `dynamic-workflow-ir` |
| 50 | `DW-IR-004` | Typed operator I/O/error contracts | `dynamic-workflow-ir` |
| 60 | `DW-IR-005` | IR/compiler determinism proof | `dynamic-workflow-ir` |
| 70 | `HX-AUTHORITY-001` | Human/agent/Secretary authority matrix | `human-authority` |
| 80 | `ARCH-HEX-001` | Close only architecture debt blocking H1–H3 | `architecture` |
| 90 | `EVT-LEDGER-001` | Canonical event/version/replay contract | `event-ledger` |

**H0 exit:** agents use a generic topology/impact/evidence service instead of broad runner guessing; deterministic contracts required by planning persistence/generated runtime are accepted and tested; authority and architecture boundaries are explicit.

---

## H1 — Planning SSOT

**Goal:** replace hand-maintained planning interpretation with repository-reconstructible Planning Ledger state.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 100 | `PLN-LEDGER-001` | Planning Ledger domain/state model | `planning-ledger` |
| 110 | `PLN-LEDGER-002` | Deterministic repository persistence | `planning-ledger` |
| 120 | `PLN-LEDGER-003` | Bind semantic work to cycles/runs and migrate active planning | `planning-ledger` |
| 130 | `PLN-LEDGER-004` | `status/next/blocked/show/graph` projections | `planning-ledger` |

**H1 exit:** a clean checkout computes the same planning graph and next Work Item without interpreting roadmap prose.

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

**Goal:** one semantic answer to “what can/should happen next?” for declared and generated runs.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 190 | `DEC-PLANE-001` | Unified `CurrentRunView` | `decision-plane` |
| 200 | `DEC-PLANE-002` | Generic persisted-frontier next-action computation | `decision-plane` |
| 210 | `DEC-PLANE-003` | Typed policy, explanation and provenance | `decision-plane` |
| 220 | `DEC-PLANE-004` | CLI/recovery parity over Decision Plane | `decision-plane` |

**H3 exit:** legal actions derive from persisted state + policy, not CLI choreography.

---

## H4 — AgentHost, Context Compiler & Decision Memory

**Goal:** let fresh agents operate semantically with reconstructible context, explicit role boundaries, rich handoffs and a durable navigable history of why decisions were made.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 230 | `AGENT-HOST-001` | Semantic AgentHost tool surface | `agent-first` |
| 240 | `AGENT-HOST-002` | Provider health/failure/failover/usage telemetry | `agent-first` |
| 250 | `CTX-COMPILER-001` | Capsules/deltas/staleness/negative knowledge | `agent-context` |
| 260 | `CTX-COMPILER-002` | Cold-start reconstruction from CurrentRunView/recovery state | `agent-context` |
| 261 | `CDD-ROLE-001` | Typed agent roles + delegation/authority topology validation | `cdd-role` |
| 263 | `CDD-HANDOFF-001` | DelegationRequest + ContextLease + ContributionEnvelope | `cdd-handoff` |
| 265 | `CDD-HANDOFF-002` | SynthesisReceipt + dissent/information-loss guard | `cdd-handoff` |
| 267 | `CDD-MEMORY-001` | Git-like content-addressed Decision Memory DAG + refs/HEAD/reflog | `cdd-memory` |
| 268 | `CDD-MEMORY-002` | log/tree/show/diff/merge-base/branch + session-delta projections | `cdd-memory` |
| 269 | `CDD-CONTINUE-001` | ResumeView + rich ContinuationCandidate frontier | `cdd-continuity` |

**H4 exit:** a fresh LLM resolves authoritative state plus Decision Memory `HEAD`, can traverse/diff relevant decision branches and receives/provides typed, loss-auditable agent handoffs without prior chat memory.

---

## H5 — Human & Reactive Control

**Goal:** add HITL and bounded reactive assistance on top of the same Decision Plane, CDD handoff contracts and Decision Memory.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 270 | `HX-DECISION-001` | Human decision types + `HumanDecisionPort` | `human-collaboration` |
| 280 | `HX-DECISION-002` | Risk policy + CLI/AgentHost adapters | `human-collaboration` |
| 290 | `HX-RESUME-001` | `ResumeInfo` + `RehydrationPlan` over CDD ResumeView | `human-collaboration` |
| 300 | `RX-SECRETARY-001` | Deterministic L0 reactions | `secretary` |
| 310 | `RX-SECRETARY-002` | Bounded L1 Secretary proposals through CDD contribution path | `secretary` |
| 320 | `RX-SECRETARY-003` | Budgeted/policy-bounded cognitive replan | `secretary` |

**H5 exit:** human and Secretary actions share one policy/authorization/provenance path and leave Decision Memory evidence rather than side-channel state.

---

## H6 — Runtime Completeness, Decision Search & Workflow Lab

**Goal:** complete advanced operator semantics, stabilize empirical comparison and only then experiment with bounded alternative-decision search.

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 330 | `DW-OPERATORS-001` | Typed durable child output/lineage | `advanced-runtime` |
| 340 | `DW-OPERATORS-002` | Durable Map fan-out | `advanced-runtime` |
| 350 | `DW-OPERATORS-003` | Typed Reduce aggregation | `advanced-runtime` |
| 360 | `DW-OPERATORS-004` | JoinAny/JoinAll + runtime guards | `advanced-runtime` |
| 370 | `DW-REPLAY-001` | Advanced graph revision/replay/recovery proof | `advanced-runtime` |
| 380 | `LAB-WORKFLOW-001` | Stable runtime/workflow metrics | `workflow-lab` |
| 390 | `LAB-WORKFLOW-002` | Fork/ablation/strategy comparison/promotion evidence | `workflow-lab` |
| 392 | `LAB-DECISION-001` | Decision-Memory branch/fork lookahead + Pareto/beam/best-first baseline | `decision-lab` |
| 394 | `LAB-DECISION-002` | Evaluate ToT/GoT/MCTS/LATS-like strategies before any promotion | `decision-lab` |

**H6 exit:** advanced graphs are deterministic/replayable; counterfactual decision branches are bounded and traceable; experimental search cannot bypass canonical policy/HITL or mutate canonical `HEAD`.

---

## H7 — Engineering Assurance & UAT

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 400 | `EA-ASSURANCE-001` | Assurance profiles/evidence taxonomy/rules | `engineering-assurance` |
| 410 | `EA-ASSURANCE-002` | Evidence resolvers + deterministic evaluators | `engineering-assurance` |
| 420 | `UAT-BC-001` | UAT lifecycle bounded context/pack | `uat` |
| 430 | `EA-UAT-001` | Gate Decision Plane with assurance/UAT evidence | `engineering-assurance` |

**H7 exit:** workflow progression can be deterministically gated by explainable engineering and user-acceptance evidence.

---

## H8 — Adaptive SDD

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

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 490 | `GRAPH-WHY-001` | Typed active-graph projection including Decision Memory/delegation relations | `active-graph` |
| 500 | `GRAPH-WHY-002` | `why` / `debt why` / `decision why` causal queries | `active-graph` |
| 510 | `COCKPIT-001` | Overview/journal/timeline/execution + Decision Memory tree views | `cockpit` |
| 520 | `COCKPIT-002` | Provider/usage/assurance/handoff/experiment views | `cockpit` |

**H9 exit:** users/agents can inspect current state, causal history and deliberative branches without manually reconstructing files/logs.

---

## H10 — Governed Continuous Improvement

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 530 | `GCI-LEARNING-001` | ExperienceEpisode/process mining over events + decision/delegation outcomes | `gci` |
| 540 | `GCI-LEARNING-002` | Bounded strategy experiments | `gci` |
| 550 | `GCI-LEARNING-003` | Evidence-backed promotion/tuning/rollback | `gci` |

**H10 exit:** workflow and decision-strategy improvement is measurable, reversible and governed.

---

## H11 — Multi-pack Proof

| Order | Semantic cycle | Purpose | Context pack |
|---:|---|---|---|
| 560 | `MULTIPACK-001` | Lock generic pack contracts | `multipack` |
| 570 | `MULTIPACK-002` | UAT pack on canonical runtime | `multipack` |
| 580 | `MULTIPACK-003` | Incident pack on canonical runtime | `multipack` |
| 590 | `MULTIPACK-004` | Prove no pack-specific kernel special cases | `multipack` |

**H11 exit:** SDD, UAT and Incident share one generic runtime/Decision Plane contract.

---

## H12 — Supply Chain, Production Hardening & GA

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

## 2. Timeline invariants

1. Do not jump over the first non-terminal dependency-satisfied Work Item.
2. Do not execute a later horizon because its code looks easier or more interesting.
3. Do not resurrect an old evolution pack as a parallel timeline.
4. Do not reuse historical cycle numbers as semantic identity.
5. Test-impact planning must remain language/build/test-runner neutral; ecosystem support is an adapter/profile concern.
6. Normal `apply` does not compensate for unknown impact by running the entire repository; unknown mapping fails closed and `verify` owns broad integration evidence.
7. Do not pull advanced Map/Reduce/Join into H2.
8. Do not introduce Human/Secretary authority before Decision Plane + CDD role/handoff/memory foundations.
9. A Decision Memory `what-if`/rejected branch is advisory until governed promotion; it never gains runtime authority by being newer or more detailed.
10. Do not introduce ToT/GoT/MCTS/LATS-like search into core runtime; evaluate it in H6 and retain deterministic baseline/fallback.
11. Do not introduce learning/process mining before stable runtime/replay/lab evidence.
12. Do not promote adaptive SDD without Workflow Lab evidence.
13. Do not claim generic-kernel maturity before multi-pack proof.
14. Do not claim GA before supply-chain, recovery, security and release-readiness gates pass.

## 3. How the agent uses this document

For the selected Work Item:

1. locate it here;
2. identify predecessor, successor, horizon and context-pack key;
3. load that key from `CYCLE-CONTEXT-MAP.yaml`;
4. read exact dependencies/objective/exit gate from `EXECUTION-SPINE.yaml`;
5. execute under `AGENT-EXECUTION-PROTOCOL.md`;
6. when H4 CDD is shipped, resolve the current Decision Memory/ResumeView as additional reconstructed context;
7. never use this Markdown projection to override newer evidenced Planning Ledger/spine/runtime truth.