# SDDK Product Backlog — Canonical Capability View

> **Status:** canonical capability context
> **Baseline:** v1.70.0 / 2026-09-03 reconciliation
> **Exact execution order:** [`EXECUTION-SPINE.yaml`](./EXECUTION-SPINE.yaml)
> **Agent execution rule:** [`AGENT-EXECUTION-PROTOCOL.md`](./AGENT-EXECUTION-PROTOCOL.md)
> **Decision Memory design:** [`DECISION-MEMORY-GIT-MODEL.md`](./DECISION-MEMORY-GIT-MODEL.md)

## 1. Backlog rules

This backlog explains **what each capability means**. It does not choose the next cycle.

- Semantic IDs are stable work identity.
- Concrete `cycle-N`/run IDs are execution bindings only.
- Exact order/dependencies/status are authoritative in `EXECUTION-SPINE.yaml`.
- Released behavior and execution evidence beat stale prose.
- Historical evolution packs are design inputs, never parallel queues.
- One `ACTIVE` semantic Work Item is the default.

Allowed states: `PROPOSED`, `READY`, `ACTIVE`, `BLOCKED`, `PARTIAL`, `SHIPPED`, `ABSORBED`, `SUPERSEDED`.

## 2. Reconciled shipped/absorbed baseline

| Work Item | Status | Disposition |
|---|---|---|
| `SD-CONTEXT-001` | `SHIPPED` | State-driven active-cycle context inference shipped in v1.67.0. |
| `SD-NEXT-001` | `SHIPPED` | `cycle next` derives legal transition from declared workflow YAML; v1.68.0. |
| `SD-RECOVERY-001` | `SHIPPED` | Actionable conflict/recovery hints; v1.69.0. |
| `LF-PAUSE-001` | `SHIPPED` | Pause/resume, `Paused`, leases/fencing/receipts; v1.70.0. |
| `AFI-FACADE-001` | `ABSORBED` | Agent-first facade intent absorbed by facade/project-input/goal/parity work. |
| `AFI-STATEFUL-CLI-001` | `ABSORBED` | Semantic CLI intent substantially absorbed by state-driven context/next/recovery. |
| `MAP-FOUNDATION-001` | `PARTIAL` | Operator substrate exists; durable output/lineage/advanced semantics remain H6. |

---

## 3. H0 — Reconcile & Deterministic Foundations

### `GOV-ROADMAP`

- `GOV-ROADMAP-001` — canonical roadmap, backlog, evolution crosswalk, execution spine, timeline, context map and agent continuation protocol.

### `TEST-MODEL`

Purpose: model **what changed, what can be affected and what verification capability exists** without assuming a language, build system or runner.

- `TEST-MODEL-001` — language-neutral `ActiveChangeSet`, `ProjectTestTopology`, SUT graph and `VerificationCapability` contracts.

Core model supports:

- single-language, multi-module and polyglot repositories;
- components/build units/source/config/schema/generated artifacts;
- runtime and contract boundaries;
- stable semantic tests/suites/capabilities;
- provenance/confidence on inferred topology relations;
- fail-closed unknown impact.

### `TEST-ADAPTER`

Purpose: keep ecosystem mechanics outside kernel planning.

- `TEST-ADAPTER-001` — generic topology/test adapter SPI, capability registry and explicit project mapping fallback;
- `TEST-ADAPTER-002` — prove composability with SDDK's Rust stack, multiple contrasting non-Rust fixtures and a cross-language/polyglot contract edge.

Adapter rule:

```text
kernel chooses semantic evidence
adapter discovers/translates mechanism
runner executes
receipt returns to kernel
```

Cargo, Maven/Gradle, npm/pnpm/yarn, pytest, Go, .NET, CMake/Bazel/etc. are adapter families, never planning branches in the kernel. Unsupported ecosystems can use a stable explicit profile/mapping until a native adapter exists.

### `TEST-SELECT`

- `TEST-SELECT-001` — deterministic change→SUT impact propagation and progressive direct/component/dependency/contract/risk batch selection.

Normal `apply` stops when scoped obligations have fresh evidence. Unknown impact blocks; it does not silently trigger a whole-repository test run.

### `TEST-EVIDENCE`

- `TEST-EVIDENCE-001` — `TestEvidenceReceipt`, freshness identity, graph-driven invalidation/reuse and selector-quality telemetry.

Primary quality guard: broad `verify` regressions missed by scoped `apply` become **escape-rate** evidence against the mapping/selection strategy.

### `TEST-APPLY`

- `TEST-APPLY-001` — integrate semantic topology/impact/next-batch behavior with `apply`, strict TDD, `verify` and coding-agent contracts.

Lifecycle invariant:

```text
apply  = progressive change-scoped verification
verify = declared full-project verification profile
```

This is project/language neutral and also applies when one repository contains several ecosystems.

### `DW-IR`

- `DW-IR-001` — typed execution scope;
- `DW-IR-002` — transition/predicate AST contract;
- `DW-IR-003` — graph revision/hash/provenance invariants;
- `DW-IR-004` — typed operator I/O/error contract;
- `DW-IR-005` — IR/compiler determinism proof.

### `HX-AUTHORITY`

- `HX-AUTHORITY-001` — explicit authority matrix/no-parallel-authority invariant for CLI, orchestrator, workers, humans and Secretary.

### `ARCH-HEX`

- `ARCH-HEX-001` — close only architecture boundary debt blocking H1-H3.

### `EVT-LEDGER`

- `EVT-LEDGER-001` — canonical event/version/correlation/causation/replay contracts required by persisted runs and later Decision Memory provenance.

---

## 4. H1 — Planning SSOT

### `PLN-LEDGER`

- `PLN-LEDGER-001` — Planning Ledger domain model/state machine;
- `PLN-LEDGER-002` — deterministic repository persistence;
- `PLN-LEDGER-003` — cycle/run bindings + migration of current planning state;
- `PLN-LEDGER-004` — deterministic `status`, `next`, `blocked`, `show`, `graph` projections.

This absorbs the useful intent of older Planning Lifecycle/Decision Ledger proposals.

---

## 5. H2 — Generated Workflow MVP

### `DW-RUNTIME`

- `DW-RUNTIME-001` — compile `NewWorkflowPlan` to deterministic `ExecutionGraphRevision`;
- `DW-RUNTIME-002` — persist `WorkflowRun` identity/lifecycle;
- `DW-RUNTIME-003` — execute Sequence + Conditional;
- `DW-RUNTIME-004` — bounded Parallel + durable node/run state/receipts;
- `DW-RUNTIME-005` — end-to-end replay/resume UAT.

Full Map/Reduce/Join remain H6.

---

## 6. H3 — Decision Plane

### `DEC-PLANE`

Purpose: one semantic answer to “what actions are legal/available next and why?” for declared and generated runs.

- `DEC-PLANE-001` — unified `CurrentRunView`;
- `DEC-PLANE-002` — generic persisted-frontier next-action computation;
- `DEC-PLANE-003` — typed policy + explainable decision provenance;
- `DEC-PLANE-004` — CLI/recovery parity.

Old `DecisionSnapshot`/CurrentRun concepts are absorbed here rather than duplicated.

---

## 7. H4 — AgentHost, Context Compiler & Decision Memory

### `AGENT-HOST`

- `AGENT-HOST-001` — semantic tool surface over Planning Ledger/runtime/Decision Plane;
- `AGENT-HOST-002` — provider failure classification, health, bounded failover and usage telemetry.

### `CTX-COMPILER`

- `CTX-COMPILER-001` — context capsules, deltas, staleness, negative knowledge and provenance;
- `CTX-COMPILER-002` — CurrentRun/recovery context and cold-start AgentHost continuation.

### `CDD-ROLE`

Purpose: turn role boundaries from prompt convention into contracts the kernel can validate.

- `CDD-ROLE-001` — `AgentRoleContract` with role kind, responsibility, dispatch allowlist, read/write/tool scopes, mutation authority, budgets, schemas, synthesis owner and forbidden actions.

Target invariants:

- leaf agents cannot delegate;
- coordinators dispatch only declared workers;
- one synthesis owner per join;
- no authority cycles;
- workers cannot mutate lifecycle/planning state unless explicitly authorized by role contract.

### `CDD-HANDOFF`

Purpose: make delegation loss-auditable and semantically rich without copying full worker contexts into the orchestrator prompt.

- `CDD-HANDOFF-001` — typed `DelegationRequest`, immutable/versioned `ContextLease`, `AgentContributionEnvelope`;
- `CDD-HANDOFF-002` — `OrchestrationSynthesisReceipt`, dissent preservation, context-loss/information-loss guard.

`AgentContributionEnvelope` must represent at least:

- objective + context revision;
- coverage satisfied/missing;
- findings;
- proposals/alternatives/rejections;
- pros/cons;
- assumptions/uncertainty;
- risks/open questions;
- evidence refs/artifact refs;
- context delta;
- recommendation/confidence/metrics.

The raw artifact remains authoritative evidence; the envelope is an index/projection.

### `CDD-MEMORY`

Purpose: represent SDDK's deliberative history as a traversable immutable DAG with Git-like semantics.

- `CDD-MEMORY-001` — content-addressed `DecisionMemoryBlob`, `DecisionMemoryTree`, `DecisionMemoryCommit`, parent links, refs, `HEAD`, tags and append-only reflog;
- `CDD-MEMORY-002` — semantic `log/tree/show/diff/merge-base/branch/ancestors/why/reflog`, SessionCheckpoint/SessionDelta and decision/delegation branch projections.

Key model:

```text
objects immutable
+ parentage
+ refs/branches
+ canonical HEAD
+ tags/reflog
+ explicit merge receipt
+ lossless links to evidence/contributions
```

Suggested refs include:

```text
refs/heads/canonical
refs/heads/session/<id>
refs/heads/decision/<decision>/<option>
refs/heads/what-if/<experiment>
refs/heads/rejected/<decision>/<option>
refs/tags/cycle/<cycle>
refs/tags/release/<version>
```

A `what-if` or rejected branch is advisory and never gains workflow/planning authority implicitly.

Decision Memory is a reconstructible projection/index over canonical state/events/artifacts/knowledge; it is not another source of truth.

### `CDD-CONTINUE`

- `CDD-CONTINUE-001` — `ResumeView` + rich `ContinuationCandidate` frontier integrated with AgentHost/orchestrator cold start.

A continuation candidate includes:

- action/kind/prerequisites;
- pros/cons;
- risks;
- reversibility;
- confidence/uncertainty;
- evidence refs;
- expected value/cost;
- what it blocks/unlocks;
- human-authority requirement.

Cold start should support semantic equivalents of:

```text
memory log --graph
memory show <ref>
memory diff <session-A>..<HEAD>
memory why <decision>
resume explain [--at <ref|timestamp>]
session diff <A> <B>
```

---

## 8. H5 — Human & Reactive Control

### `HX-DECISION`

- `HX-DECISION-001` — immutable HumanDecision request/decision/receipt contracts + `HumanDecisionPort`;
- `HX-DECISION-002` — risk-sensitive approval policy + CLI/AgentHost adapters.

### `HX-RESUME`

- `HX-RESUME-001` — `ResumeInfo`/`RehydrationPlan` as human-collaboration specialization over generic CDD `ResumeView`.

v1.70 pause/resume is substrate; CDD/HX semantic resumability is richer.

### `RX-SECRETARY`

- `RX-SECRETARY-001` — deterministic L0 reactive rules;
- `RX-SECRETARY-002` — bounded L1 closed-set proposals through the same CDD contribution + policy/authority path;
- `RX-SECRETARY-003` — bounded cognitive replan after deterministic options are exhausted.

Secretary is never a second orchestrator, memory owner or authority path.

---

## 9. H6 — Runtime Completeness, Decision Search & Workflow Lab

### `DW-OPERATORS`

- `DW-OPERATORS-001` — durable child output/lineage + remove placeholder result semantics;
- `DW-OPERATORS-002` — durable Map;
- `DW-OPERATORS-003` — deterministic Reduce;
- `DW-OPERATORS-004` — JoinAny/JoinAll + graph/runtime guards.

### `DW-REPLAY`

- `DW-REPLAY-001` — advanced graph revision lineage, cross-tick replay and partial recovery.

### `LAB-WORKFLOW`

- `LAB-WORKFLOW-001` — stable quality/cost/latency/retry/handoff/failure metrics;
- `LAB-WORKFLOW-002` — fork/ablation/strategy comparison and promotion/shadow evidence.

### `LAB-DECISION`

Purpose: investigate dynamic decision-tree/graph search without contaminating deterministic core policy.

- `LAB-DECISION-001` — Decision Memory branch/fork lookahead, deterministic Pareto + bounded beam/best-first baseline, explicit pruning receipts and counterfactual branches;
- `LAB-DECISION-002` — reproducible experiments for Tree-of-Thoughts-, Graph-of-Thoughts-, MCTS- and LATS-like strategies.

Promotion requires:

- quality/success uplift;
- bounded cost and latency;
- stable failure behavior;
- traceability to branch/evidence;
- rollback;
- no bypass of policy/HITL;
- deterministic baseline/fallback remains available.

---

## 10. H7 — Engineering Assurance & UAT

### `EA-ASSURANCE`

- `EA-ASSURANCE-001` — assurance profiles, evidence taxonomy/rules;
- `EA-ASSURANCE-002` — evidence resolvers/deterministic evaluators.

### `UAT-BC`

- `UAT-BC-001` — scenario/human-check/defect/retest/signoff lifecycle.

### `EA-UAT`

- `EA-UAT-001` — Decision Plane gates backed by assurance/UAT evidence.

---

## 11. H8 — Adaptive SDD

### `SDD-ADAPTIVE`

- `SDD-ADAPTIVE-001` — ChangeContract + SHAPE/adaptive specialist selection;
- `SDD-ADAPTIVE-002` — BUILD WorkGraph/WorkUnit mapping;
- `SDD-ADAPTIVE-003` — CONVERGE + adaptive verification;
- `SDD-ADAPTIVE-004` — INTEGRATE + legacy projections/parity;
- `SDD-ADAPTIVE-005` — Workflow Lab comparison/promotion against A-full.

Adaptive is promoted only with non-inferior quality/invariant evidence.

---

## 12. H9 — Active Graph & Cockpit

### `GRAPH-WHY`

- `GRAPH-WHY-001` — typed causal projection over requirements/evidence/decisions/Decision Memory/delegations/runs/artifacts/debt/lineage;
- `GRAPH-WHY-002` — `why`, `debt why`, `decision why` paths.

### `COCKPIT`

- `COCKPIT-001` — overview/journal/timeline/execution graph + Decision Memory tree;
- `COCKPIT-002` — provider/usage/assurance/handoff/experiment views.

H9 visualizes/query-projects H4 CDD; it does not create a second memory graph of authority.

---

## 13. H10 — Governed Continuous Improvement

### `GCI-LEARNING`

- `GCI-LEARNING-001` — ExperienceEpisode/process mining over canonical events plus decision/delegation outcomes, rejected branches and revisit triggers;
- `GCI-LEARNING-002` — bounded strategy experiments;
- `GCI-LEARNING-003` — evidence-backed promotion/tuning/rollback/policy ratchets.

---

## 14. H11 — Multi-pack Proof

### `MULTIPACK`

- `MULTIPACK-001` — lock generic pack contracts;
- `MULTIPACK-002` — UAT pack on canonical runtime;
- `MULTIPACK-003` — Incident pack on canonical runtime;
- `MULTIPACK-004` — prove no pack-specific kernel special cases.

---

## 15. H12 — Supply Chain, Production Hardening & GA

### `SUPPLYCHAIN`

- `SUPPLYCHAIN-001` — SBOM, provenance, governed artifact lifecycle;
- `SUPPLYCHAIN-002` — signed gates, policy ratchets, controlled overrides.

### `PROD-HARDEN`

- `PROD-HARDEN-001` — performance, retention, migration, reliability;
- `PROD-HARDEN-002` — security, upgrade/rollback, recovery/operator documentation.

### `GA`

- `GA-001` — full release-readiness matrix;
- `GA-002` — GA release + first stable compatibility contract.

`GA-002` is terminal.

---

## 16. How to know the next cycle

Do not read this document top-to-bottom and guess. Use `EXECUTION-SPINE.yaml` and `AGENT-EXECUTION-PROTOCOL.md`.

```text
SHIPPED: GOV-ROADMAP-001
CURRENT: TEST-MODEL-001
NEXT after TEST-MODEL-001 evidence: TEST-ADAPTER-001
LATER H0: TEST-ADAPTER-002 → TEST-SELECT-001 → TEST-EVIDENCE-001 → TEST-APPLY-001 → DW-IR-001
FINAL: GA-002
```

The spine contains every admitted Work Item between those points, including language-agnostic scoped verification, CDD/Decision Memory and Decision Lab, with explicit order, dependencies, objective and exit gate.