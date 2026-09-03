# Evolution Crosswalk — Historical Packs to the Canonical Execution Line

> **Purpose:** explain where major historical proposals now live, what has shipped, and what must not be implemented twice.
> **Canonical execution order:** `EXECUTION-SPINE.yaml` / `ROADMAP.md` H0 → H12 → `GA-002`.

## 1. Why this document exists

SDDK accumulated several valuable evolution dossiers while implementation continued to move. The resulting problem was planning drift: old documents kept describing capabilities as future work after their intent had been partially or fully absorbed by newer releases.

This crosswalk turns those documents into design evidence for one execution line.

## 2. Classification rules

- `SHIPPED` — released behavior satisfies the core proposal.
- `ABSORBED` — intent survives through a newer abstraction/path.
- `PARTIAL` — useful substrate exists but acceptance semantics remain.
- `REMAINING` — still valuable and mapped to the canonical line.
- `SUPERSEDED` — do not implement in the original form.

Exact status/order is authoritative in `EXECUTION-SPINE.yaml`.

## 3. Dynamic Workflow evolution

Source: `docs/evolutivo-workflows-dinamicos-integracion-roadmap.md`.

| Original theme | Current assessment | Canonical destination |
|---|---|---|
| Workflow IR / plan AST | `PARTIAL` | H0 `DW-IR` |
| string `scope` | remaining design debt | `DW-IR-001` |
| transition/predicate AST stability | remaining | `DW-IR-002` |
| graph revision/hash/provenance | `PARTIAL` | `DW-IR-003`, H2 `DW-RUNTIME` |
| persisted `WorkflowRun` | skeleton/`PARTIAL` | H2 `DW-RUNTIME-002` |
| generated bounded DAG execution | remaining | H2 `DW-RUNTIME-003..005` |
| dynamic/generated next action | `PARTIAL` | H3 `DEC-PLANE-002` |
| replan trigger/types | `PARTIAL` | H5 `RX-SECRETARY` |
| Secretary/policy integration | remaining | H5 `RX-SECRETARY` |
| Map | `PARTIAL` | H6 `DW-OPERATORS-002` |
| Reduce | remaining | H6 `DW-OPERATORS-003` |
| JoinAny/JoinAll | remaining | H6 `DW-OPERATORS-004` |
| operator real outputs | `PARTIAL` | H0 contract + H6 `DW-OPERATORS-001` |
| Workflow Lab | remaining | H6 `LAB-WORKFLOW` |
| Planning Ledger | remaining and raised priority | H1 `PLN-LEDGER` |

### Decision

This dossier remains the strongest technical source for Workflow IR/runtime/reactive design, but its local horizon/cycle numbering is not canonical. Runtime work is admitted only through semantic Work Items in the execution spine.

## 4. Human-Agent Collaboration pack

Source: `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/`.

The pack baseline was v1.50.0; several prerequisites have shipped since then.

| Original theme | Current assessment | Canonical destination |
|---|---|---|
| authority matrix/reconciliation | remaining and urgent | H0 `HX-AUTHORITY-001` |
| `CurrentRunView` | remaining, refined to shared model | H3 `DEC-PLANE-001` |
| HumanDecision contracts + port | remaining | H5 `HX-DECISION-001` |
| risk-sensitive HITL/adapters | remaining | H5 `HX-DECISION-002` |
| pause/resume substrate | `SHIPPED` v1.70.0 | do not reimplement |
| semantic cold-start resume / rehydration | remaining | H5 `HX-RESUME-001` |
| collaboration observability | remaining | H5/H6/H7 |
| optimization/labs | defer until stable runtime semantics | H6+ |

### Decision

The pack is a design dossier for authority, HITL and resumability. `CurrentRunView` must consume the common Decision Plane and must not become a second Human-Agent-specific source of truth.

## 5. Complete Evolution pack 2026-08-23

Source: `docs/sddk-complete-evolution-2026-08-23/`.

Its validated baseline was approximately v1.37.x. Its three pillars now map as follows.

### 5.1 Agent-First Interface (AFI)

| Original AFI intent | Current assessment | Canonical destination |
|---|---|---|
| semantic facade/project context | largely `ABSORBED` | existing facade/project-input behavior |
| deterministic goal semantics | `ABSORBED/PARTIAL` | current goal/facade/runtime contracts |
| semantic CLI avoiding command probing | substantially `ABSORBED` | state-driven CLI + H3 Decision Plane |
| DecisionSnapshot | useful intent, original shape not canonical | H3 decision context/provenance |
| semantic AgentHost tool surface | remaining | H4 `AGENT-HOST-001` |
| provider/tool telemetry | partial | H4 `AGENT-HOST-002` |
| cross-project/context semantics | remaining | H4 `CTX-COMPILER` |
| process mining | remaining, intentionally later | H10 `GCI-LEARNING` |

**Decision:** do not execute AFI-001..010 verbatim. Re-admit only missing intent through current semantic Work Items.

### 5.2 Engineering Assurance

Current assessment: valid and largely remaining.

Canonical destination: H7 `EA-ASSURANCE` + `EA-UAT`.

Retained concepts:

- profiles/rules;
- typed evidence;
- evidence resolvers;
- deterministic evaluators;
- risk/capability-specific gates;
- explainable verdicts;
- UAT integration;
- policy/provenance outputs.

EA consumes the common runtime and Decision Plane; it does not create another execution engine.

### 5.3 Governed Continuous Improvement

Current assessment: valid future work, intentionally blocked until stable runtime/replay/lab semantics.

Canonical destination: H10 `GCI-LEARNING`.

Retained concepts:

- ExperienceEpisode-like projections;
- process mining;
- strategy comparison;
- bounded experiments;
- governed promotion/tuning/rollback.

Learning observes canonical execution evidence; it never defines runtime truth.

## 6. State-Driven CLI and Lifecycle Flexibility

| Historical item | Reconciled status |
|---|---|
| active-cycle context inference | `SHIPPED` v1.67.0 |
| workflow-driven `cycle next` | `SHIPPED` v1.68.0 |
| actionable recovery/error contract | `SHIPPED` v1.69.0 |
| cycle pause/resume | `SHIPPED` v1.70.0 |
| Planning Lifecycle / Planning Ledger | remaining; H1 `PLN-LEDGER` |
| inspect/repair/provenance improvements | re-admit only where needed by the canonical spine |

## 7. Secretary material

Existing Secretary specs/ADRs remain useful design inputs, but implementation ordering is now explicit:

1. H0 authority/event/runtime contracts;
2. H2 persisted generated runtime;
3. H3 Decision Plane;
4. H4 AgentHost/context substrate;
5. H5 human authority and bounded Secretary.

Secretary must never become a parallel mutable orchestrator.

## 8. Full canonical mapping by horizon

| Horizon | Main historical material consumed |
|---|---|
| H0 | roadmap drift cleanup; Workflow IR hardening; authority reconciliation; hex/event foundations |
| H1 | Planning Lifecycle / Planning Ledger ideas |
| H2 | WorkflowRun / ExecutionGraphRevision / generated workflow proposals |
| H3 | State-Driven CLI + DecisionSnapshot/CurrentRun intent |
| H4 | AFI AgentHost semantic tools + provider/context ideas |
| H5 | HumanDecision/HITL/resume + Secretary/replan ideas |
| H6 | advanced Map/Reduce/Join/replay + Workflow Lab |
| H7 | Engineering Assurance + UAT |
| H8 | Adaptive SDD proposals |
| H9 | Active Graph / why + Cockpit |
| H10 | GCI/process mining/strategy promotion |
| H11 | generic multi-pack validation |
| H12 | supply chain, production hardening and GA |

## 9. What must not happen

- Do not execute the three historical evolution packs as sequential mega-projects.
- Do not resurrect shipped state-driven CLI or pause/resume work under old IDs.
- Do not implement a second CurrentRun/Decision state model for Human-Agent collaboration.
- Do not let Secretary bypass Decision Plane/human authority policy.
- Do not complete advanced operators before the minimal persisted runtime proves its contracts.
- Do not start process mining before stable runtime/replay/lab evidence exists.
- Do not leave a valid retained capability outside the execution spine as an unscheduled “future epic”.

## 10. Historical document policy

Historical packs are retained for rationale, alternatives, examples, abandoned approaches and detailed acceptance ideas. Their `STATUS.md` companions explain current interpretation.

If historical material conflicts with `EXECUTION-SPINE.yaml`, `ROADMAP.md` or released evidence, it does not drive implementation. The discrepancy must be reconciled through the planning governance rules.
