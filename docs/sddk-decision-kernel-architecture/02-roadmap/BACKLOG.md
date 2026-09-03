# SDDK Product Backlog — Canonical Capability View

> **Status:** canonical capability context
> **Baseline:** v1.70.0 / 2026-09-03 reconciliation
> **Exact execution order:** [`EXECUTION-SPINE.yaml`](./EXECUTION-SPINE.yaml)
> **Agent execution rule:** [`AGENT-EXECUTION-PROTOCOL.md`](./AGENT-EXECUTION-PROTOCOL.md)

## 1. Backlog rules

This backlog explains capabilities. It does **not** choose the next cycle.

- Semantic IDs are the stable identity of work.
- Concrete `cycle-N` / run IDs are execution bindings only.
- Exact order, dependencies and current status are authoritative in `EXECUTION-SPINE.yaml`.
- Released behavior and durable execution evidence beat stale prose.
- Old evolution packs are design inputs, never parallel execution queues.
- The canonical line is serial by default: one `ACTIVE` semantic Work Item unless a governed planning decision explicitly allows safe concurrency.

Allowed states: `PROPOSED`, `READY`, `ACTIVE`, `BLOCKED`, `PARTIAL`, `SHIPPED`, `ABSORBED`, `SUPERSEDED`.

## 2. Reconciled shipped/absorbed baseline

| Work Item | Status | Disposition |
|---|---|---|
| `SD-CONTEXT-001` | `SHIPPED` | State-driven active-cycle context inference shipped in v1.67.0. |
| `SD-NEXT-001` | `SHIPPED` | `cycle next` derives the next legal transition from declared workflow YAML; shipped in v1.68.0. |
| `SD-RECOVERY-001` | `SHIPPED` | Actionable workflow/source/evidence conflict and recovery hints shipped in v1.69.0. |
| `LF-PAUSE-001` | `SHIPPED` | Pause/resume, `CycleStatus::Paused`, leases/fencing and receipts shipped in v1.70.0. |
| `AFI-FACADE-001` | `ABSORBED` | Agent-first facade intent absorbed by existing facade/project-input, goal semantics and parity work. |
| `AFI-STATEFUL-CLI-001` | `ABSORBED` | Semantic CLI intent substantially absorbed by state-driven context, `cycle next` and recovery contracts. |
| `MAP-FOUNDATION-001` | `PARTIAL` | Map/operator substrate exists; durable output/lineage/advanced join semantics remain in H6. |

These classifications exist to prevent old packs from reopening already-delivered capabilities.

---

## 3. H0 — Reconcile & Deterministic Foundations

### `GOV-ROADMAP`

Purpose: remove roadmap ambiguity and establish one machine-readable execution line.

Primary Work Item:

- `GOV-ROADMAP-001` — canonical roadmap, backlog, evolution crosswalk, execution spine and agent continuation protocol.

### `DW-IR`

Purpose: make the generated-workflow compiler boundary deterministic before H2.

Work Items:

- `DW-IR-001` — typed execution scope;
- `DW-IR-002` — transition/predicate AST contract;
- `DW-IR-003` — graph revision/hash/provenance invariants;
- `DW-IR-004` — typed operator I/O/error contract;
- `DW-IR-005` — IR determinism/compiler-boundary tests.

### `HX-AUTHORITY`

- `HX-AUTHORITY-001` — authority matrix plus no-parallel-authority invariant for CLI, agent, human and Secretary paths.

### `ARCH-HEX`

- `ARCH-HEX-001` — close only hexagonal/boundary debt that blocks the H1-H3 canonical path.

### `EVT-LEDGER`

- `EVT-LEDGER-001` — canonical event/versioning/correlation/causation/replay contracts required by persisted workflow runs.

---

## 4. H1 — Planning SSOT

### `PLN-LEDGER`

Purpose: remove hand-written markdown as executable planning state.

Work Items:

- `PLN-LEDGER-001` — Planning Ledger domain model/state machine;
- `PLN-LEDGER-002` — deterministic repository persistence;
- `PLN-LEDGER-003` — cycle/run bindings and migration of current planning state;
- `PLN-LEDGER-004` — deterministic `status`, `next`, `blocked`, `show`, `graph` projections.

This absorbs the useful intent of the older lifecycle/planning-ledger proposal.

---

## 5. H2 — Generated Workflow MVP

### `DW-RUNTIME`

Purpose: one narrow durable generated-workflow vertical slice.

Work Items:

- `DW-RUNTIME-001` — compile `NewWorkflowPlan` to deterministic `ExecutionGraphRevision`;
- `DW-RUNTIME-002` — persist `WorkflowRun` identity/lifecycle;
- `DW-RUNTIME-003` — execute Sequence + Conditional;
- `DW-RUNTIME-004` — bounded Parallel + durable node/run state/receipts;
- `DW-RUNTIME-005` — end-to-end replay/resume UAT.

Explicitly deferred to H6: full Map, Reduce, JoinAny/JoinAll and advanced replay.

---

## 6. H3 — Decision Plane

### `DEC-PLANE`

Purpose: one answer to “what actions are legal/available next?” for declared and generated workflows.

Work Items:

- `DEC-PLANE-001` — `CurrentRunView`;
- `DEC-PLANE-002` — generic next-action computation;
- `DEC-PLANE-003` — typed policy evaluation + explainable decision provenance;
- `DEC-PLANE-004` — CLI/recovery parity on the Decision Plane.

The useful intent of old `DecisionSnapshot` / Human-Agent current-run proposals is absorbed here rather than implemented as duplicate models.

---

## 7. H4 — AgentHost & Context Compiler

### `AGENT-HOST`

- `AGENT-HOST-001` — semantic agent tool surface over planning/runtime/Decision Plane capabilities;
- `AGENT-HOST-002` — provider failure classification, health, bounded failover and usage telemetry.

### `CTX-COMPILER`

- `CTX-COMPILER-001` — context capsules, deltas, staleness, negative knowledge and provenance;
- `CTX-COMPILER-002` — CurrentRun/recovery context and cold-start AgentHost continuation.

This horizon is where the old AFI semantic-tool intent that was not already absorbed becomes concrete.

---

## 8. H5 — Human & Reactive Control

### `HX-DECISION`

- `HX-DECISION-001` — immutable HumanDecision request/decision/receipt contracts + `HumanDecisionPort`;
- `HX-DECISION-002` — risk-sensitive approval policy + CLI/AgentHost adapters.

### `HX-RESUME`

- `HX-RESUME-001` — `ResumeInfo`, `RehydrationPlan`, semantic cold-start resume.

v1.70.0 pause/resume is substrate, not completion of semantic Human-Agent resumability.

### `RX-SECRETARY`

- `RX-SECRETARY-001` — deterministic L0 reactive rules;
- `RX-SECRETARY-002` — bounded L1 closed-set proposals through shared authority/policy;
- `RX-SECRETARY-003` — bounded cognitive replan only after deterministic options are exhausted.

Secretary is never a second orchestrator and never mutates authoritative workflow state outside the Decision Plane/policy path.

---

## 9. H6 — Runtime Completeness & Workflow Lab

### `DW-OPERATORS`

- `DW-OPERATORS-001` — durable child output/lineage model and removal of placeholder result semantics;
- `DW-OPERATORS-002` — complete durable Map;
- `DW-OPERATORS-003` — deterministic Reduce;
- `DW-OPERATORS-004` — JoinAny/JoinAll plus graph/runtime guards.

### `DW-REPLAY`

- `DW-REPLAY-001` — advanced graph revision lineage, cross-tick replay and partial-recovery invariants.

### `LAB-WORKFLOW`

- `LAB-WORKFLOW-001` — stable workflow/runtime quality/cost/latency/retry/handoff/failure metrics;
- `LAB-WORKFLOW-002` — fork/ablation, strategy comparison and promotion/shadow evidence.

---

## 10. H7 — Engineering Assurance & UAT

### `EA-ASSURANCE`

- `EA-ASSURANCE-001` — assurance profiles, evidence taxonomy and rule contracts;
- `EA-ASSURANCE-002` — evidence resolvers and deterministic evaluators.

### `UAT-BC`

- `UAT-BC-001` — UAT scenario, human check, defect, retest and signoff lifecycle on the canonical runtime.

### `EA-UAT`

- `EA-UAT-001` — integrate assurance/UAT verdicts with Decision Plane policy/provenance.

Engineering Assurance from the 2026-08-23 pack remains valid here, but not as a separate execution engine.

---

## 11. H8 — Adaptive SDD

### `SDD-ADAPTIVE`

- `SDD-ADAPTIVE-001` — ChangeContract + SHAPE/adaptive specialist selection;
- `SDD-ADAPTIVE-002` — BUILD WorkGraph/WorkUnit mapping;
- `SDD-ADAPTIVE-003` — CONVERGE + adaptive verification;
- `SDD-ADAPTIVE-004` — INTEGRATE + legacy projections/parity;
- `SDD-ADAPTIVE-005` — Workflow Lab comparison and promotion decision against A-full.

Adaptive is promoted only with non-inferior quality/invariant evidence, not simply lower cost.

---

## 12. H9 — Active Graph & Cockpit

### `GRAPH-WHY`

- `GRAPH-WHY-001` — typed causal graph projection over requirements/evidence/decisions/runs/artifacts/debt/lineage;
- `GRAPH-WHY-002` — minimal-facade causal `why` / `debt why` queries.

### `COCKPIT`

- `COCKPIT-001` — overview, journal, timeline and execution graph;
- `COCKPIT-002` — provider health, usage, assurance and experiment views.

---

## 13. H10 — Governed Continuous Improvement

### `GCI-LEARNING`

- `GCI-LEARNING-001` — ExperienceEpisode-style projections + process mining;
- `GCI-LEARNING-002` — bounded strategy experiments/comparison;
- `GCI-LEARNING-003` — evidence-backed promotion/tuning/rollback/policy ratchets.

GCI waits for stable runtime/event/replay/Lab semantics and cannot bypass deterministic policy or human authority.

---

## 14. H11 — Multi-pack Proof

### `MULTIPACK`

- `MULTIPACK-001` — generic pack contract lock/proof;
- `MULTIPACK-002` — UAT pack on canonical runtime;
- `MULTIPACK-003` — Incident pack on canonical runtime;
- `MULTIPACK-004` — end-to-end proof of no pack-specific kernel special cases.

---

## 15. H12 — Supply Chain, Production Hardening & GA

### `SUPPLYCHAIN`

- `SUPPLYCHAIN-001` — SBOM, provenance and governed artifact lifecycle;
- `SUPPLYCHAIN-002` — signed gates, policy ratchets and controlled overrides.

### `PROD-HARDEN`

- `PROD-HARDEN-001` — performance, retention, migration and reliability hardening;
- `PROD-HARDEN-002` — security review, upgrade/rollback and operator/recovery documentation.

### `GA`

- `GA-001` — full release-readiness matrix across SDD/UAT/Incident plus security/recovery/provenance gates;
- `GA-002` — GA release and first stable compatibility contract.

`GA-002` is the terminal semantic Work Item of this plan.

---

## 16. How to know the next cycle

Do not read this document top-to-bottom and guess.

Use `EXECUTION-SPINE.yaml` and `AGENT-EXECUTION-PROTOCOL.md`.

Current bootstrap state:

```text
ACTIVE: GOV-ROADMAP-001
NEXT after evidence/merge: DW-IR-001
FINAL: GA-002
```

The spine already contains every admitted Work Item between those points, with explicit ordering, dependencies, objective and exit gate.
