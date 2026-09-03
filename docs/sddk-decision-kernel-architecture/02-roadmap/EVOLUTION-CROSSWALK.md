# Evolution Crosswalk — From Historical Packs to the Canonical Roadmap

> **Purpose:** explain where every major proposal now lives, what has already shipped, and what must not be implemented twice.
> **Canonical execution order:** `ROADMAP.md` H0 → H6.

## 1. Why this document exists

SDDK accumulated several valuable evolution dossiers while the implementation kept moving. The result was not bad design; it was **planning drift**: old documents continued to describe capabilities as future work after their intent had been partially or fully absorbed by newer code and abstractions.

This crosswalk makes those documents inputs to one roadmap instead of parallel roadmaps.

## 2. Classification rules

- `SHIPPED`: current released behavior already satisfies the core proposal.
- `ABSORBED`: intent survives, but a newer abstraction/implementation path supersedes the original shape.
- `PARTIAL`: useful substrate exists; important acceptance semantics remain.
- `REMAINING`: still valuable and mapped to a canonical epic/horizon.
- `SUPERSEDED`: do not implement in the original form.

## 3. Dynamic Workflow evolution

Source: `docs/evolutivo-workflows-dinamicos-integracion-roadmap.md`.

| Original theme | Current assessment | Canonical destination |
|---|---|---|
| Workflow IR / plan AST | `PARTIAL` | H0 `DW-IR` |
| string `scope` | remaining design debt | `DW-IR-001` |
| transition/predicate AST stability | remaining | `DW-IR-002` |
| graph revision/hash/provenance | `PARTIAL` | `DW-IR-003`, H2 `DW-RUNTIME` |
| persisted `WorkflowRun` | skeleton/`PARTIAL` | H2 `DW-RUNTIME-002` |
| generated bounded DAG execution | remaining | H2 `DW-RUNTIME-003..006` |
| dynamic/generated `cycle next` | `PARTIAL` | H3 `DEC-PLANE-002` |
| replan trigger/types | `PARTIAL` | H4 `RX-SECRETARY` |
| Secretary/policy integration | remaining/blocked | H4 `RX-SECRETARY` |
| Map | `PARTIAL` | H5 `DW-OPERATORS-001` |
| Reduce | remaining | H5 `DW-OPERATORS-002` |
| JoinAny/JoinAll | remaining | H5 `DW-OPERATORS-003` |
| operator real outputs | `PARTIAL` | H0 contract + H5 `DW-OPERATORS-004` |
| Workflow Lab | remaining | H5 `LAB-WORKFLOW` |
| Planning Ledger | remaining and raised priority | H1 `PLN-LEDGER` |

### Decision

The dynamic-workflow dossier is the strongest technical design source for H0-H5, but its old cycle numbers are not canonical identifiers. Its H0-H5 concept has been refined into the repository-wide H0-H6 roadmap.

## 4. Human-Agent Collaboration pack

Source: `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/`.

Baseline of the pack was v1.50.0, so several prerequisites have changed materially since it was written.

| Original theme | Current assessment | Canonical destination |
|---|---|---|
| Authority Matrix / reconciliation | remaining and urgent | H0 `HX-AUTHORITY` |
| `CurrentRunView` | remaining, but should not become a second state model | H3 `DEC-PLANE-001` + alias `HX-CURRENT-RUN` |
| HumanDecision domain contracts | remaining | H4 `HX-DECISION-001` |
| `HumanDecisionPort` | remaining | H4 `HX-DECISION-002` |
| risk-driven HITL policy | remaining | H4 `HX-DECISION-003` |
| CLI/MCP/Agent adapters | remaining, after shared contracts | H4 `HX-DECISION-004` |
| pause/resume substrate | `SHIPPED` in v1.70.0 | do not reimplement |
| semantic cold-start resume | still remaining | H4 `HX-RESUME` |
| rehydration plan | remaining | H4 `HX-RESUME-002` |
| observability/validation | still useful | H4/H5, then EA |
| optimization/labs | defer until stable runtime semantics | H5/H6 |

### Decision

The Human-Agent pack remains valid as a **design dossier for authority, HITL and resumability**, not as an independent roadmap. Its CurrentRun concept must consume the common Decision Plane rather than create a Human-Agent-specific state projection.

## 5. Complete Evolution pack 2026-08-23

Source: `docs/sddk-complete-evolution-2026-08-23/`.

The pack was validated against approximately v1.37.x. It contains three major pillars: Agent-First Interface (AFI), Engineering Assurance (EA) and Governed Continuous Improvement (GCI).

### 5.1 Agent-First Interface

| Original AFI intent | Current assessment | Canonical destination |
|---|---|---|
| semantic facade/project context | largely `ABSORBED` | existing facade/project-input behavior |
| deterministic goal semantics | `ABSORBED/PARTIAL` | current goal/facade/runtime contracts |
| semantic CLI avoiding low-level command probing | substantially `ABSORBED` | state-driven context + `cycle next` + future `DEC-PLANE` |
| DecisionSnapshot | useful intent, original shape not canonical | H3 `DEC-PLANE-004` |
| semantic AgentHost tool surface | remaining | North Star `AGENT-HOST` + H3 parity |
| cross-project semantic tooling | remaining | later AgentHost/capability work |
| tool/process telemetry | substrate exists, broader intent remains | H5/H6 |
| process mining over agent tools | remaining, but too early | H6 `GCI-LEARNING` |

**Decision:** do not execute AFI-001..010 verbatim. Treat AFI as `ABSORBED` except for the semantic AgentHost and learning pieces explicitly remapped above.

### 5.2 Engineering Assurance

Current assessment: **valid and largely remaining**, but should be implemented as a cross-cutting capability over the common runtime, evidence and Decision Plane.

Canonical epic: H6 `EA-ASSURANCE`.

Useful retained concepts:

- profiles and rules;
- typed evidence;
- resolvers;
- deterministic evaluators;
- capability/risk-specific assurance;
- explainable verdicts;
- UAT integration;
- policy/provenance outputs.

Contracts can be designed earlier, but EA must not fork workflow execution into its own engine.

### 5.3 Governed Continuous Improvement

Current assessment: **valid future work, intentionally blocked** until event, replay and workflow semantics are stable.

Canonical epic: H6 `GCI-LEARNING`.

Retained concepts:

- ExperienceEpisode-like projections;
- process mining;
- strategy tuning/comparison;
- bounded experiments;
- governed promotion and rollback.

**Decision:** GCI consumes stable runtime evidence; it does not define runtime truth.

## 6. State-Driven CLI and Lifecycle Flexibility

Several entries in historical roadmap/backlog prose are stale because the implementation moved faster than the planning docs.

| Historical item | Reconciled status |
|---|---|
| active-cycle context inference | `SHIPPED` v1.67.0 |
| graph/declaration-driven `cycle next` | `SHIPPED` v1.68.0 |
| actionable recovery/error contract | `SHIPPED` v1.69.0 |
| cycle pause/resume | `SHIPPED` v1.70.0 |
| Planning Lifecycle / Planning Ledger | remaining; promoted to H1 `PLN-LEDGER` |
| lifecycle inspect/repair/provenance improvements | retain only when they support H1-H5 contracts |

Old backlog entries that still say `PROPOSED` for shipped items are superseded by the reconciled `BACKLOG.md`.

## 7. Secretary material

Existing Secretary spec/ADR work remains useful, but implementation must obey the new dependency order:

1. H0 authority reconciliation;
2. H2 durable generated runtime;
3. H3 Decision Plane and policy surface;
4. H4 bounded Secretary proposals sharing the same authority semantics.

Secretary must never become a parallel mutable orchestrator.

## 8. Canonical mapping by horizon

### H0

Consumes:
- dynamic workflow IR hardening;
- Human-Agent authority reconciliation;
- roadmap/backlog drift cleanup.

### H1

Consumes:
- Lifecycle Flexibility Planning Ledger proposal;
- governance/provenance concepts from older packs.

### H2

Consumes:
- dynamic WorkflowRun / ExecutionGraphRevision / generated workflow proposals.

### H3

Consumes:
- shipped state-driven CLI behavior;
- old AFI DecisionSnapshot intent;
- Human-Agent CurrentRunView intent;
- policy/recovery projections.

### H4

Consumes:
- HumanDecision/HITL/resume pack;
- Secretary/reaction proposals;
- replan substrate.

### H5

Consumes:
- advanced dynamic operators;
- durable replay/lineage;
- Workflow Lab.

### H6

Consumes:
- Engineering Assurance;
- Governed Continuous Improvement;
- process mining/strategy promotion.

## 9. What must not happen

- Do not schedule the three evolution packs as three sequential mega-projects.
- Do not resurrect shipped state-driven CLI or pause/resume work under old IDs.
- Do not implement a second CurrentRun/Decision state model for Human-Agent collaboration.
- Do not let Secretary bypass the Decision Plane or Human authority policy.
- Do not complete advanced operators before the minimal persisted generated runtime proves its contracts.
- Do not build process-mining feedback loops before event/replay semantics stabilize.

## 10. Historical document policy

Historical packs are retained unchanged except for small `STATUS.md` companions. They remain valuable for rationale, alternatives, examples, abandoned approaches and detailed acceptance ideas.

If a historical document conflicts with `ROADMAP.md` or `BACKLOG.md`, this crosswalk determines its current interpretation until the Planning Ledger becomes the machine-readable planning SSOT.
