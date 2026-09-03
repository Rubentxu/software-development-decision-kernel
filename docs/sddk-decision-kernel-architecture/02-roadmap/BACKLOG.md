# SDDK Product Backlog — Canonical Capability View

> **Status:** canonical capability backlog
> **Baseline:** v1.70.0 / 2026-09-03 reconciliation
> **Ordering:** follow `ROADMAP.md` horizons H0 → H6.

## 1. Backlog rules

This backlog tracks **capabilities**, not release chronology.

- Use semantic IDs (`DW-RUNTIME-001`, `HX-DECISION-001`, ...).
- Do not use historical `cycle-N` as a capability identity.
- Status values are restricted to `PROPOSED`, `READY`, `ACTIVE`, `BLOCKED`, `PARTIAL`, `SHIPPED`, `ABSORBED`, `SUPERSEDED`.
- Released behavior wins over stale planning text.
- Execution bindings belong in cycle/run artifacts and, after H1, the Planning Ledger.

## 2. Reconciled shipped/absorbed work

These items are retained here specifically to prevent old evolution packs from reopening them as new projects.

| Work Item | Status | Current disposition |
|---|---|---|
| `SD-CONTEXT-001` | `SHIPPED` | State-driven active-cycle context inference shipped in v1.67.0. |
| `SD-NEXT-001` | `SHIPPED` | `cycle next` derives the next legal transition from declared workflow YAML; shipped in v1.68.0. |
| `SD-RECOVERY-001` | `SHIPPED` | Actionable workflow/source/evidence conflict and recovery hints shipped in v1.69.0. |
| `LF-PAUSE-001` | `SHIPPED` | Pause/resume, `CycleStatus::Paused`, leases/fencing and receipts shipped in v1.70.0. |
| `AFI-FACADE-001` | `ABSORBED` | Old Agent-First facade intent absorbed by facade/project-input, goal semantics and parity work already delivered across earlier releases. |
| `AFI-STATEFUL-CLI-001` | `ABSORBED` | Old semantic CLI intent substantially absorbed by state-driven context, `cycle next` and recovery contracts. |
| `MAP-FOUNDATION-001` | `PARTIAL` | Map/runtime/operator substrate exists, but advanced durable output/join semantics are not complete. |

`SHIPPED` and `ABSORBED` entries should only change after evidence shows the classification is wrong.

---

## 3. H0 — Reconcile & Harden

### Epic `GOV-ROADMAP` — Planning/documentation governance

| Work Item | Status | Acceptance |
|---|---|---|
| `GOV-ROADMAP-001` Canonical roadmap/backlog | `ACTIVE` | One official H0-H6 line, one capability backlog, no competing mega-roadmaps. |
| `GOV-ROADMAP-002` Evolution crosswalk | `ACTIVE` | Old packs mapped to `SHIPPED` / `ABSORBED` / `PARTIAL` / remaining epics. |
| `GOV-ROADMAP-003` Semantic identity policy | `ACTIVE` | New work uses semantic IDs; cycle IDs are execution bindings only. |
| `GOV-ROADMAP-004` Repository status reconciliation rule | `READY` | A repeatable rule specifies how planning docs are corrected when release/code evidence disagrees. |

### Epic `DW-IR` — Workflow IR hardening

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `DW-IR-001` Typed execution scope | `READY` | — | Replace stringly scope where execution semantics require a closed typed contract; preserve schema migration compatibility. |
| `DW-IR-002` Transition/predicate AST contract | `READY` | — | Stable versioned serialization and deterministic validation. |
| `DW-IR-003` Graph revision/hash/provenance invariants | `READY` | — | Same normalized plan compiles to stable identity; mutations create explicit new revision lineage. |
| `DW-IR-004` Operator I/O/error contract | `READY` | — | Operators expose explicit input/output/error semantics rather than relying on placeholder values. |
| `DW-IR-005` IR determinism tests | `READY` | `DW-IR-001..004` | Round-trip, hash stability and invalid-plan tests prove the H2 compiler boundary. |

### Epic `HX-AUTHORITY` — Human/agent authority reconciliation

| Work Item | Status | Acceptance |
|---|---|---|
| `HX-AUTHORITY-001` Authority matrix | `READY` | Existing CLI/agent/human/Secretary responsibilities and writable state are explicitly classified. |
| `HX-AUTHORITY-002` No-parallel-authority invariant | `READY` | Future human and Secretary paths are required to converge on the same Decision Plane/policy semantics. |

---

## 4. H1 — Planning SSOT

### Epic `PLN-LEDGER` — Planning / Decision Ledger

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `PLN-LEDGER-001` Planning domain model | `PROPOSED` | `GOV-ROADMAP` | Stable WorkItem, dependency, state, evidence and decision/provenance contracts. |
| `PLN-LEDGER-002` Repository persistence | `PROPOSED` | `PLN-LEDGER-001` | Checkout reconstructs the same planning graph deterministically. |
| `PLN-LEDGER-003` Cycle/run bindings | `PROPOSED` | `PLN-LEDGER-001` | Execution IDs bind to semantic work without becoming conceptual IDs. |
| `PLN-LEDGER-004` Roadmap projections | `PROPOSED` | `PLN-LEDGER-002` | Human-readable status/next/blocked/graph/show projections derive from the ledger. |
| `PLN-LEDGER-005` Planning migration | `PROPOSED` | `PLN-LEDGER-002` | Current active backlog imported with provenance and without rewriting historical cycle evidence. |

This epic **absorbs Lifecycle Flexibility Primitive 2 / Planning Lifecycle** from the older roadmap.

---

## 5. H2 — Generated Workflow MVP

### Epic `DW-RUNTIME` — Persisted generated workflow vertical slice

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `DW-RUNTIME-001` Compile plan to graph revision | `PROPOSED` | `DW-IR` | Valid `NewWorkflowPlan` compiles into a deterministic `ExecutionGraphRevision`. |
| `DW-RUNTIME-002` Persist `WorkflowRun` | `PROPOSED` | `DW-RUNTIME-001` | Run identity, graph revision and provenance survive process restart. |
| `DW-RUNTIME-003` Execute bounded DAG | `PROPOSED` | `DW-RUNTIME-002` | Sequence + bounded Parallel + Conditional execute without hard-coded workflow command sequences. |
| `DW-RUNTIME-004` Durable node/run state | `PROPOSED` | `DW-RUNTIME-003` | Progress, terminal state, failures and receipts are persisted. |
| `DW-RUNTIME-005` Replay/reconstruction | `PROPOSED` | `DW-RUNTIME-004` | Replay reconstructs equivalent run/graph state deterministically. |
| `DW-RUNTIME-006` Resume vertical slice | `PROPOSED` | `DW-RUNTIME-004` | A stopped process resumes the same persisted run safely. |

**Explicitly deferred:** full Map/Reduce/Join, Secretary and cognitive replanning.

---

## 6. H3 — Decision Plane

### Epic `DEC-PLANE` — Unified progression/policy surface

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `DEC-PLANE-001` CurrentRunView | `PROPOSED` | `DW-RUNTIME` | One projection exposes declared/generated frontier, current state, blockers and available actions. |
| `DEC-PLANE-002` Generic next-action contract | `PROPOSED` | `DEC-PLANE-001` | `cycle next`/equivalent consumes persisted frontier + policy for both declared and generated workflows. |
| `DEC-PLANE-003` Policy evaluation boundary | `PROPOSED` | `DEC-PLANE-001` | Available/forbidden/requires-approval actions are typed and explainable. |
| `DEC-PLANE-004` Decision context/provenance | `PROPOSED` | `DEC-PLANE-003` | Decision inputs are durable/explainable and satisfy the useful intent of the old `DecisionSnapshot` proposal. |
| `DEC-PLANE-005` CLI/AgentHost parity | `PROPOSED` | `DEC-PLANE-002..004` | Both surfaces expose the same semantic actions/results. |

### Epic `HX-CURRENT-RUN`

`HX-CURRENT-RUN` is not a second implementation. It is the Human-Agent consumer/projection requirements for `DEC-PLANE-001` and is tracked as an **aliasing capability** to avoid duplicate CurrentRun models.

---

## 7. H4 — Human & Reactive Control

### Epic `HX-DECISION` — Human decision / HITL

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `HX-DECISION-001` HumanDecision domain contracts | `PROPOSED` | `HX-AUTHORITY`, `DEC-PLANE` | Immutable request/decision/receipt types with provenance. |
| `HX-DECISION-002` HumanDecisionPort | `PROPOSED` | `HX-DECISION-001` | Application layer requests decisions without depending on CLI/UI adapters. |
| `HX-DECISION-003` Risk-sensitive approval policy | `PROPOSED` | `DEC-PLANE-003`, `HX-DECISION-001` | Policy decides autonomous/approval/deny behavior deterministically where possible. |
| `HX-DECISION-004` CLI/AgentHost adapters | `PROPOSED` | `HX-DECISION-002..003` | Adapters preserve identical authority and receipt semantics. |

### Epic `HX-RESUME` — Semantic resumability

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `HX-RESUME-001` ResumeInfo / rehydration model | `PROPOSED` | `DEC-PLANE-001` | Cold start explains current state, pending decision/work and safe continuation. |
| `HX-RESUME-002` RehydrationPlan | `PROPOSED` | `HX-RESUME-001` | Required context is reconstructed explicitly instead of relying on agent memory. |
| `HX-RESUME-003` Resume parity | `PROPOSED` | `HX-RESUME-002` | CLI and agent surfaces resume the same run with equivalent semantics. |

Note: v1.70.0 pause/resume is valuable substrate but does **not** by itself complete semantic cold-start Human-Agent resumability.

### Epic `RX-SECRETARY` — Bounded reactive control

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `RX-SECRETARY-001` Deterministic reaction rules | `PROPOSED` | `DEC-PLANE` | L0 reactions use typed signals and policy with no LLM required. |
| `RX-SECRETARY-002` Bounded proposal contract | `BLOCKED` | `DEC-PLANE`, authority gate | Secretary proposes closed-set actions; it does not mutate workflow state directly. |
| `RX-SECRETARY-003` Proposal authorization/receipts | `BLOCKED` | `HX-DECISION`, `RX-SECRETARY-002` | Accepted actions pass the same policy/authority path and are durably receipted. |
| `RX-SECRETARY-004` Cognitive replan boundary | `BLOCKED` | `RX-SECRETARY-001..003` | LLM replan is bounded, explainable and used only after deterministic options are exhausted. |

Existing Secretary specs/ADRs remain design inputs; they do not override this dependency order.

---

## 8. H5 — Runtime Completeness & Workflow Lab

### Epic `DW-OPERATORS`

| Work Item | Status | Depends on | Acceptance |
|---|---|---|---|
| `DW-OPERATORS-001` Map durable semantics | `PARTIAL` | `DW-RUNTIME` | Dynamic fan-out has persisted child lineage, outputs and replay semantics. |
| `DW-OPERATORS-002` Reduce semantics | `PROPOSED` | `DW-OPERATORS-001` | Typed deterministic aggregation and failure policy. |
| `DW-OPERATORS-003` JoinAny/JoinAll semantics | `PROPOSED` | `DW-RUNTIME` | Explicit completion/cancellation/output semantics. |
| `DW-OPERATORS-004` Child output model | `PARTIAL` | `DW-RUNTIME` | Remove placeholder `Value::Null` result semantics from real execution paths. |
| `DW-OPERATORS-005` Runtime guards | `PROPOSED` | `DW-OPERATORS-001..004` | graph/node/depth/concurrency/budget bounds enforced durably. |

### Epic `DW-REPLAY`

- durable graph revision lineage;
- child lifecycle reconstruction;
- cross-tick replay invariants;
- recovery after partial expansion/execution;
- deterministic conflict detection.

### Epic `LAB-WORKFLOW`

| Work Item | Status | Depends on |
|---|---|---|
| `LAB-WORKFLOW-001` Stable workflow metrics | `PROPOSED` | `DW-REPLAY` |
| `LAB-WORKFLOW-002` Fork/ablation runner | `PROPOSED` | `LAB-WORKFLOW-001` |
| `LAB-WORKFLOW-003` Strategy comparison | `PROPOSED` | `LAB-WORKFLOW-002` |
| `LAB-WORKFLOW-004` Promotion/shadow policy | `PROPOSED` | `LAB-WORKFLOW-003` |

---

## 9. H6 — Assurance & Governed Learning

### Epic `EA-ASSURANCE` — Engineering Assurance

Current disposition: **valid capability family, not a separate execution engine**.

Planned sub-capabilities:

- assurance profiles;
- evidence taxonomy/types;
- evidence resolvers;
- deterministic evaluators;
- capability/risk-specific rules;
- UAT integration;
- policy/decision outputs consumable by the Decision Plane;
- provenance and explainability.

Detailed EA tasks from `sddk-complete-evolution-2026-08-23` must be re-admitted individually against this architecture rather than executed verbatim from the old pack.

### Epic `GCI-LEARNING` — Governed Continuous Improvement

Current disposition: **valid future capability; blocked until stable event/replay semantics**.

Planned sub-capabilities:

- ExperienceEpisode-like projections;
- process mining from canonical workflow/run events;
- strategy quality/cost comparison;
- bounded experiments;
- evidence-backed strategy promotion;
- rollback and policy ratchets.

---

## 10. Other retained product epics

The following remain in the North Star and should be scheduled by dependency/value without creating a parallel roadmap:

- `ARCH-HEX` — hexagonal convergence / focused ports;
- `EVT-LEDGER` — canonical events and journal/replay;
- `AGENT-HOST` — AgentHost and provider resilience;
- `CTX-COMPILER` — context capsules/deltas/staleness;
- `SDD-ADAPTIVE` — adaptive SDD experimental path;
- `GRAPH-WHY` — Active Graph and causal `why` queries;
- `COCKPIT` — static/control-plane projections;
- `UAT-BC` — UAT bounded context/pack;
- `MULTIPACK` — proof that SDD/UAT/Incident share the same kernel/runtime;
- `SUPPLYCHAIN` — SBOM/provenance/artifact lifecycle/policy hardening;
- `TEST-BOUNDARY` — remaining test-tooling boundary cleanup.

Admission into an active horizon requires a dependency argument and acceptance contract.

## 11. Immediate next queue

Unless new evidence invalidates dependencies, the next semantic queue is:

1. finish `GOV-ROADMAP-001..004`;
2. implement/accept `DW-IR-001..005`;
3. implement `HX-AUTHORITY-001..002` as contract reconciliation, not a new runtime;
4. start `PLN-LEDGER-001`;
5. proceed to `DW-RUNTIME-001` only when H0 exit criteria are satisfied.

Do **not** start Map/Reduce/Join completion, Secretary Stage 1+, process mining or GCI ahead of this queue.
