# Evolution Crosswalk — Historical Packs to the Canonical Execution Line

> **Purpose:** explain where major historical/research proposals now live, what has shipped, and what must not be implemented twice.
> **Canonical execution order:** `EXECUTION-SPINE.yaml` / `ROADMAP.md` H0 → H12 → `GA-002`.

## 1. Why this document exists

SDDK accumulated valuable evolution dossiers while implementation continued moving. Old documents can therefore describe as future work ideas that were later shipped, absorbed or refined.

This crosswalk turns those documents into design evidence for one execution line.

## 2. Classification rules

- `SHIPPED` — released behavior satisfies the core proposal.
- `ABSORBED` — intent survives through a newer abstraction/path.
- `PARTIAL` — useful substrate exists but acceptance semantics remain.
- `REMAINING` — still valuable and mapped to the canonical line.
- `SUPERSEDED` — do not implement in original form.

Exact status/order is authoritative in `EXECUTION-SPINE.yaml`.

## 3. Dynamic Workflow evolution

Source: `docs/evolutivo-workflows-dinamicos-integracion-roadmap.md`.

| Original theme | Assessment | Canonical destination |
|---|---|---|
| Workflow IR / plan AST | `PARTIAL` | H0 `DW-IR` |
| string `scope` | remaining | `DW-IR-001` |
| transition/predicate AST stability | remaining | `DW-IR-002` |
| graph revision/hash/provenance | `PARTIAL` | `DW-IR-003`, H2 runtime |
| persisted `WorkflowRun` | skeleton/`PARTIAL` | `DW-RUNTIME-002` |
| generated bounded DAG execution | remaining | `DW-RUNTIME-003..005` |
| dynamic/generated next action | `PARTIAL` | H3 `DEC-PLANE-002` |
| replan trigger/types | `PARTIAL` | H5 `RX-SECRETARY` |
| Secretary/policy integration | remaining | H5 `RX-SECRETARY` |
| Map | `PARTIAL` | H6 `DW-OPERATORS-002` |
| Reduce | remaining | H6 `DW-OPERATORS-003` |
| JoinAny/JoinAll | remaining | H6 `DW-OPERATORS-004` |
| operator real outputs | `PARTIAL` | H0 contract + H6 `DW-OPERATORS-001` |
| Workflow Lab | remaining | H6 `LAB-WORKFLOW` |
| Planning Ledger | remaining/raised priority | H1 `PLN-LEDGER` |

**Decision:** this remains the strongest technical dossier for Workflow IR/runtime/reactive design, but its local horizon/cycle numbering is non-canonical.

## 4. Human-Agent Collaboration pack

Source: `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/`.

| Original theme | Assessment | Canonical destination |
|---|---|---|
| authority matrix/reconciliation | remaining/urgent | H0 `HX-AUTHORITY-001` |
| `CurrentRunView` | remaining/refined to shared model | H3 `DEC-PLANE-001` |
| HumanDecision contracts + port | remaining | H5 `HX-DECISION-001` |
| risk-sensitive HITL/adapters | remaining | H5 `HX-DECISION-002` |
| pause/resume substrate | `SHIPPED` v1.70.0 | do not reimplement |
| semantic cold-start resume / rehydration | remaining but generalized | H4 `CDD-CONTINUE` + H5 `HX-RESUME` specialization |
| collaboration observability | remaining | H4-H9 |
| optimization/labs | later | H6+ |

**Decision:** human-specific resume must consume the generic CDD ResumeView/Decision Memory rather than creating another continuity model.

## 5. Complete Evolution pack 2026-08-23

Source: `docs/sddk-complete-evolution-2026-08-23/`.

### 5.1 Agent-First Interface

| Original intent | Assessment | Destination |
|---|---|---|
| semantic facade/project context | largely `ABSORBED` | existing facade/project-input |
| deterministic goal semantics | `ABSORBED/PARTIAL` | goal/facade/runtime contracts |
| semantic CLI avoiding command probing | substantially `ABSORBED` | state-driven CLI + H3 |
| DecisionSnapshot | useful intent, old shape non-canonical | H3 Decision Plane |
| semantic AgentHost tools | remaining | H4 `AGENT-HOST-001` |
| provider/tool telemetry | partial | H4 `AGENT-HOST-002` |
| cross-project/context semantics | remaining | H4 `CTX-COMPILER` |
| process mining | remaining/later | H10 `GCI-LEARNING` |

Do not execute AFI-001..010 literally.

### 5.2 Engineering Assurance

Valid/largely remaining → H7 `EA-ASSURANCE` + `EA-UAT`.

Retain profiles/rules, typed evidence, resolvers, deterministic evaluators, explainable verdicts, UAT integration and policy/provenance outputs. EA consumes common runtime/Decision Plane.

### 5.3 Governed Continuous Improvement

Valid future work → H10 `GCI-LEARNING`, intentionally after stable runtime/replay/lab/Decision Memory semantics.

Retain ExperienceEpisode-like projections, process mining, strategy comparison, bounded experiments and governed promotion/tuning/rollback.

## 6. Continuity, Delegation & Deliberation evolution

Sources:

- `docs/evolutivo-continuidad-sesiones-delegacion-deliberacion.md`
- `docs/sddk-decision-kernel-architecture/02-roadmap/DECISION-MEMORY-GIT-MODEL.md`

This research was produced after auditing current `sddk-cycle-resume`, phase handoffs, verify coordinator patterns, orchestrator wrappers, Auto-Grill, Dynamic Workflow and Secretary material, and comparing external long-running/multi-agent patterns.

| Research theme | Current substrate | Canonical destination |
|---|---|---|
| authoritative cold-start runtime reconstruction | `PARTIAL/SHIPPED substrate` via cycle resume/ledger/artifacts | preserve; H4 enriches rather than replaces |
| episodic session summary / Engram | useful advisory substrate | remains optional, never authority |
| role boundaries | prompt-level mostly | `CDD-ROLE-001` |
| rich delegation context | launch-plan/artifact substrate | `CDD-HANDOFF-001` |
| worker contribution preservation | verify has advanced local pattern | generalize in `CDD-HANDOFF-001` |
| synthesis/dissent/information-loss audit | missing generally | `CDD-HANDOFF-002` |
| Git-like decision memory DAG | missing | `CDD-MEMORY-001` |
| tree/log/diff/merge-base/session projections | missing | `CDD-MEMORY-002` |
| rich resume/continuation candidates | fragmented | `CDD-CONTINUE-001` |
| decision branch/fork/lookahead | workflow/lab substrate only | `LAB-DECISION-001` |
| ToT/GoT/MCTS/LATS-like strategies | research only | `LAB-DECISION-002` experiments |
| Decision Memory visualization | model created H4 | H9 Active Graph/Cockpit projection |
| decision/delegation process mining | future | H10 GCI |

### Core decisions

1. **Decision Memory is not another source of truth.** It is a reconstructible content-addressed projection/index over canonical events, planning/runtime state, accepted knowledge, artifacts and agent contributions.
2. The internal representation is a DAG, while the LLM/UI may receive a bounded tree projection.
3. Git-like semantics are intentional: immutable objects, parents, refs, `HEAD`, tags, reflog, ancestry, diff, merge-base/fork-point and explicit merge receipts.
4. `what-if`/rejected branches have no runtime authority.
5. Rich agent contribution + synthesis contracts precede Human/Secretary implementation so those paths do not invent incompatible handoffs.
6. Experimental search strategies remain in Workflow Lab; deterministic legal frontier/policy remains baseline.
7. Private model chain-of-thought is neither required nor persisted.

## 7. State-Driven CLI and Lifecycle Flexibility

| Historical item | Reconciled status |
|---|---|
| active-cycle context inference | `SHIPPED` v1.67.0 |
| workflow-driven `cycle next` | `SHIPPED` v1.68.0 |
| actionable recovery/error contract | `SHIPPED` v1.69.0 |
| cycle pause/resume | `SHIPPED` v1.70.0 |
| Planning Lifecycle / Planning Ledger | remaining; H1 |
| inspect/repair/provenance improvements | re-admit only where needed |

## 8. Secretary material

Canonical implementation ordering:

1. H0 authority/event contracts;
2. H2 persisted generated runtime;
3. H3 Decision Plane;
4. H4 AgentHost/Context + CDD roles/handoffs/Decision Memory;
5. H5 HumanDecision + Secretary.

Secretary must return through CDD contribution/candidate contracts, traverse normal policy/authority and never move canonical Decision Memory `HEAD` directly.

## 9. Canonical mapping by horizon

| Horizon | Main material consumed |
|---|---|
| H0 | roadmap reconciliation; Workflow IR hardening; authority; hex/event foundations |
| H1 | Planning Lifecycle / Planning Ledger |
| H2 | WorkflowRun / ExecutionGraphRevision / generated runtime |
| H3 | State-Driven CLI + DecisionSnapshot/CurrentRun intent |
| H4 | AFI AgentHost + Context Compiler + CDD continuity/delegation/Decision Memory |
| H5 | HumanDecision/HITL/resume specialization + Secretary |
| H6 | Map/Reduce/Join/replay + Workflow Lab + Decision Lab |
| H7 | Engineering Assurance + UAT |
| H8 | Adaptive SDD |
| H9 | Active Graph / why + Cockpit + Decision Memory visualization |
| H10 | GCI/process mining including decision/delegation outcomes |
| H11 | generic multi-pack validation |
| H12 | supply chain, production hardening, GA |

## 10. What must not happen

- Do not execute historical packs as sequential mega-projects.
- Do not resurrect shipped state-driven CLI/pause-resume under old IDs.
- Do not create a Human-Agent-specific duplicate CurrentRun/Decision/Memory model.
- Do not let Secretary bypass Decision Plane/CDD/human authority policy.
- Do not reduce subagent work to an untraceable prose summary when raw artifact/contribution exists.
- Do not treat Engram/session summary as canonical state.
- Do not make a `what-if` memory branch authoritative merely because it scores higher.
- Do not introduce MCTS/ToT/GoT/LATS-like behavior as core default before Lab evidence.
- Do not complete advanced operators before minimal persisted runtime proof.
- Do not start GCI/process mining before stable evidence semantics.
- Do not leave valid retained capability outside the execution spine.

## 11. Historical/research document policy

Dossiers are retained for rationale, alternatives, examples, rejected approaches and acceptance ideas. Where available, read `STATUS.md` first.

If historical/research prose conflicts with the execution spine, accepted design or released evidence, it does not drive implementation. Reconcile the discrepancy through planning governance.
