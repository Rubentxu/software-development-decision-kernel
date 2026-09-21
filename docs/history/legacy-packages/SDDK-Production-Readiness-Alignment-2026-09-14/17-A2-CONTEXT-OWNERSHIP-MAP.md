# A2.0 — Context Ownership Map

**Audit basis:** certified C7 baseline `0c2ca56` / workspace `1.169.19`.
**Purpose:** classify every relevant module by bounded context and state class
*before* any move. A module is moved only when its owner is unambiguous.

State classes (exactly one): `FACT`, `OBJECT`, `PROJECTION`, `EPHEMERAL`.
Migration modes: `MOVE` (physical), `FACADE` (temporary reexport), `KEEP`
(already-owned), `CONSOLIDATE` (duplicate removed), `RENAME` (behavior-preserving).

Rules: MOVE → VERIFY → CONSOLIDATE → DELETE. No mass-move + redesign in one
commit. No decorative empty contexts. Reorganise inside the current crates
(no new crate per bounded context).

---

## 1. Target contexts → current module ownership

### shared
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-domain/src/identity` | project/workspace identity + vault path | one canonical identity | OBJECT | KEEP |
| `sddk-domain/src/evidence` | universal Evidence model | one evidence authority | OBJECT | KEEP |
| `sddk-domain/src/event_envelope` | `EventEnvelopeV1` | shared fact envelope | FACT | KEEP |
| `sddk-domain/src/models` | typed IDs/records | shared domain ids | OBJECT | KEEP |
| `sddk-domain/src/format`, `macros`, `error`, `schema`, `delivery_kind` | helpers/errors | shared utilities | EPHEMERAL | KEEP |
| `sddk-engine/src/revision_substrate` | `Oid`/`Revision<T>`/`Ref`/`RefStore` | universal revision/CAS | OBJECT | KEEP |
| `sddk-engine/src/canonical_event_log` | `CanonicalEventLog`, `CasRef` | single append authority + ref | FACT | KEEP |
| `sddk-engine/src/cas_object_store`, `evidence_ref` | CAS object store + ref | shared content-addressing | OBJECT/EPHEMERAL | KEEP |
| `sddk-storage/src/cas` | `FilesystemCas` | shared bytes CAS | OBJECT | KEEP |

### planning
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-domain/src/goal` | Goal | planning intent | OBJECT | KEEP |
| `sddk-domain/src/planning/**` | WorkItem, manifests, evidence relation | planning model | OBJECT/FACT | KEEP |
| `sddk-domain/src/plan_revision` | plan revisions/lineage | planning revision semantics | OBJECT | KEEP |
| `sddk-domain/src/spine` | roadmap spine projection | planning projection | PROJECTION | KEEP |
| `sddk-domain/src/proposal` | change proposal | planning | OBJECT | KEEP |
| `sddk-domain/src/validator`, `compiler` | plan validation/compile | planning correctness | EPHEMERAL | KEEP |

### execution
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-domain/src/workflow*` (`workflow`, `workflow_ir`, `workflow_run`) | WorkflowDefinition/IR/Run | execution model | OBJECT/FACT | KEEP |
| `sddk-domain/src/execution_scope`, `execution_graph_compiler` | scope + graph compile | execution | EPHEMERAL | KEEP |
| `sddk-engine/src/workflow_runtime`, `execution_controller` | Run runtime | execution | FACT/EPHEMERAL | KEEP |
| `sddk-engine/src/target_task/**`, `task_executor`, `tasks` | Target/Task runtime + DAG | execution | EPHEMERAL | KEEP |
| `sddk-engine/src/run_view` | run state view (derived) | execution projection | PROJECTION | KEEP |
| `sddk-engine/src/cycle_*` (`cycle_summary`, `cycle_pause`, `cycle_replan`, `cycle_supersede`, `cycle_narrative`) | cycle runtime ops | execution lifecycle | FACT | KEEP |
| `sddk-engine/src/build_work_graph`, `join_guard`, `retry`, `circuit_breaker` | execution infrastructure | execution | EPHEMERAL | KEEP |

### decision
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-engine/src/decision_memory` | typed durable decision history | decision authority | OBJECT | KEEP |
| `sddk-engine/src/human_decision`, `decision_plane_gate` | human/decision plane | decision | FACT | KEEP |
| `sddk-engine/src/decision_lab_baseline`, `decision_lab_experimental`, `strategy_experiments`, `strategy_comparison`, `lab_promotion` | decision labs | decision experiments | EPHEMERAL/PROJECTION | KEEP |
| `sddk-domain/src/plan_revision` decision semantics | decision refs | (shared with planning; classification recorded) | OBJECT | KEEP |

### knowledge
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-engine/src/semantic_graph`, `semantic_node`, `semantic_kind` | SemanticGraph projection | rebuildable graph | PROJECTION/OBJECT | KEEP |
| `sddk-engine/src/active_graph`, `active_graph_view`, `active_graph_drift`, `active_graph_digest` | ActiveGraph derived view | projection only | PROJECTION | KEEP |
| `sddk-engine/src/why_queries` | WHY projection | knowledge/graph query | PROJECTION | KEEP |
| `sddk-engine/src/vault_boundary` | vault boundary | knowledge | EPHEMERAL | KEEP |
| `sddk-storage/src/graph_store`, `projection_store` | graph/checkpoint stores | knowledge persistence | PROJECTION | KEEP |
| `sddk-vault/**` (`index`, `search`, `parser`, `validate`, `export`, `repair`, `graph`) | Vault source material | source, not authority | PROJECTION | KEEP |

### alignment
| path | concept | reason | state | mode |
|---|---|---|---|---|
| _(none yet)_ | — | A4 implements Software Alignment | — | — |

No decorative module is created. The context is reserved; its first real owner
arrives in A4 (or a later refactor) without inventing semantics here.

### verification
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-engine/src/converge_verification`, `engineering_assurance*`, `change_contract`, `integrate_parity`, `replay_proof`, `up_to_date` | verification orchestration | change-scoped verification | EPHEMERAL/PROJECTION | KEEP |
| `sddk-engine/src/gate_evaluator`, `signed_gates`, `gate_signing`, `release_readiness`, `production_hardening` | gates/release checks | verification | FACT/EPHEMERAL | KEEP |
| `sddk-engine/src/uat_lifecycle`, `uat_pack` | UAT lifecycle | verification | EPHEMERAL | KEEP |
| `sddk-cli/src/verify_cmd`, `audit_cmd`, `change` | CLI verify/audit surfaces | verification orchestration | EPHEMERAL | KEEP |

Note: these are the **current** verifiers. They are NOT declared to be the future
`Verify`/`DebVerify` of A4; ownership is recorded, the distinction stays explicit.

### governance
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-engine/src/authority` | authority facts/context | governance | FACT | KEEP |
| `sddk-engine/src/authority_engine` (+ `runner`, `bridge`) | AuthorityEngine decision path | governance | OBJECT | KEEP |
| `sddk-engine/src/risk_approval_policy` | policy snapshots | governance | FACT | KEEP |
| `sddk-engine/src/gate_error` | gate errors | governance | EPHEMERAL | KEEP |
| `sddk-cli/src/admission` | enforcement choke point | governance | EPHEMERAL | KEEP |
| `sddk-gateway/src/permissions`, `policy` | gateway permission policy | governance | EPHEMERAL | KEEP |

### agent_experience
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-engine/src/context_capsule`, `context_compiler`, `cold_start` | ContextCapsule + compiler | agent context | EPHEMERAL | KEEP |
| `sddk-engine/src/agent_role_contract`, `agent_contribution_envelope`, `orchestration_synthesis` | agent protocol/synthesis | agent experience | OBJECT | KEEP |
| `sddk-engine/src/agent_host` | agent execution host (identity/lease/fencing/retry) | agent execution, NOT Agentic Workspace host | OBJECT | KEEP (see §4) |
| `sddk-engine/src/completion_provider_router`, `telemetry` | completion/model provider routing + usage | agent experience (completion providers) | EPHEMERAL | RENAME (A2-S2, done) |
| `sddk-cli/src/command_spec`, `command_surface`, `cheat_sheet`, `examples_walker`, `agent_surface_golden`, `agent_profile`, `arg_schema`, `surface_integration`, `instruction_compiler`, `execution_receipt`, `skill_definition`, `skill_registry_bridge` | typed command/agent surface, skills, instruction, receipts | agent experience | PROJECTION/EPHEMERAL | KEEP |
| `sddk-engine/src/receipt_writers`, `typed_child_output`, `continuation_candidate` | agent receipts/child output | agent experience | PROJECTION | KEEP |

### extension
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-domain/src/pack`, `sddk-engine/src/pack_registry`, `generic_pack_contracts`, `pack_agnosticity`, `incident_pack` | pack SDK + registry | extension | OBJECT | KEEP |
| `sddk-gateway/**` (`capability`, `runner`, `oracles`, `forge`, `git`, `filesystem`, `test_runner`, `playwright`, `computer_use`, `artifact_store`, `release`, `uat_policy`, `semantic`) | concrete adapters/extension infra | extension/gateway | EPHEMERAL | KEEP |
| `sddk-cli/src/knowledge_ingest` | vault ingestion adapter | extension/knowledge edge | EPHEMERAL | KEEP |

### observability / analytics (cross-cutting, not a core context)
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-engine/src/cockpit_views`, `cockpit_observability`, `experience_episodes`, `workflow_metrics`, `cycle_narrative` | views/analytics | observability projections | PROJECTION | KEEP |
| `sddk-cli/src/telemetry`, `metrics` | control-plane telemetry | observability | PROJECTION | KEEP |

### compatibility (bounded, read-only)
| path | concept | reason | state | mode |
|---|---|---|---|---|
| `sddk-domain/src/legacy` | legacy agent-output decode | compat namespace | EPHEMERAL | KEEP |
| `sddk-storage` decode helpers (`PlanningEvidenceKind` decoders) | legacy evidence decode | compat | EPHEMERAL | KEEP |

---

## 2. Special-concept classification (required)

| concept | canonical path | context | state | owner | duplicate candidate |
|---|---|---|---|---|---|
| CanonicalEventLog | `sddk-engine/src/canonical_event_log` + `sddk-storage/src/event_store` | shared | FACT | one append authority | none |
| Evidence | `sddk-domain/src/evidence` (+ `evidence_ref`, `evidence_backed_promotion`, `evidence_relation_mapping`) | shared | OBJECT | one Evidence model | `PlanningEvidenceKind` decode-only |
| Revision substrate | `sddk-engine/src/revision_substrate` (+ `sddk-storage/src/cas`) | shared | OBJECT | generic CAS | Decision Memory keeps decision semantics |
| DecisionMemory | `sddk-engine/src/decision_memory` | decision | OBJECT | decision authority | none (distinct, per ADR-0097) |
| SemanticGraph | `sddk-engine/src/semantic_graph` (+ stores) | knowledge | PROJECTION | rebuildable | ActiveGraph is derived |
| ActiveGraph | `sddk-engine/src/active_graph*` | knowledge | PROJECTION | derived view only | must not become authority |
| ContextCapsule | `sddk-engine/src/context_capsule` (+ `context_compiler`) | agent_experience | EPHEMERAL | compiler | none |
| AuthorityEngine | `sddk-engine/src/authority_engine` | governance | OBJECT | single decision path | legacy `AuthorityContext::validate` is defense-in-depth |
| target/task | `sddk-engine/src/target_task/**` | execution | EPHEMERAL | execution | none |
| workflow runtime | `sddk-engine/src/workflow_runtime` (+ domain `workflow*`) | execution | FACT | execution | none |
| agent profiles/instructions/skills/receipts | `sddk-cli/src/{agent_profile,instruction_compiler,skill_definition,execution_receipt}` | agent_experience | PROJECTION | agent experience | none |
| provider router |  `sddk-engine/src/completion_provider_router` | agent_experience (completions) | EPHEMERAL | agent experience | NOT intelligence provider SPI |
| WHY | `sddk-engine/src/why_queries` | knowledge | PROJECTION | knowledge | none |
| telemetry/analytics/views | `sddk-engine/src/cockpit_*`, `workflow_metrics`, `sddk-cli/src/telemetry`,`metrics` | observability | PROJECTION | observability | none |
| legacy compatibility | `sddk-domain/src/legacy`, legacy decoders | compatibility | EPHEMERAL | compat namespace | decode-only |

---

## 3. Candidate semantic consolidations (A1-backed)

1. `pub use legacy::*` → already replaced by explicit `sddk_domain::legacy`
   namespace (A0). `KEEP`.
2. Duplicate `projects` DDL stub → removed (A0). `KEEP`.
3. Independent migration runner → removed (A0). `KEEP`.

No further consolidation is introduced in A2 unless a duplicate is demonstrated
after the moves.

---

## 4. Ambiguous names (review, no blind redesign)

- **`provider_router`** — is completion/model provider failover (ADR-027/SPEC-026),
  explicitly "do not conflate" with the future Intelligence Provider SPI.
  Ambiguity is demonstrable (the Extension Platform will also have "providers").
  Plan: behavior-preserving `RENAME` to `completion_provider_router` in a
  dedicated slice with a temporary `pub use` facade.
- **`agent_host`** — is the agent *execution* host substrate (identity/lease/
  fencing/retry), not an Agentic Workspace host. Documented; no rename now.
  Future JCode work must not reuse the name for `HostAdapter`.
- **existing verifiers** — recorded as the current verification orchestration,
  explicitly NOT the future `Verify`/`DebVerify`.

---

## 5. Move plan (slices)

The current tree is already largely aligned at the crate level (`domain` =
pure types, `engine` = orchestration). A2 physical moves are therefore minimal
and targeted at **module ownership clarity** and **dependency direction**, not
mass relocation. Slices (each = MOVE → VERIFY → CONSOLIDATE → DELETE):

- **A2-S1**: dependency-fitness rules (tests/lints) for the forbidden edges.
- **A2-S2**: `provider_router` → `completion_provider_router` (RENAME; **done**, no facade retained).
- **A2-S3**: make ownership explicit for the contexts that lack a module anchor
  (`alignment` boundary doc only; no empty module), and record the ownership map
  as the registry.
- **A2-S4** (only if demonstrable): relocate any module whose owner is currently
  wrong; otherwise record `KEEP` with rationale.

Slices are recorded in `18-A2-MOVE-LEDGER.md`; fitness evidence in
`19-A2-DEPENDENCY-FITNESS-RECEIPT.md`; closure in `A2-CONTEXT-CUT-RECEIPT.md`.
