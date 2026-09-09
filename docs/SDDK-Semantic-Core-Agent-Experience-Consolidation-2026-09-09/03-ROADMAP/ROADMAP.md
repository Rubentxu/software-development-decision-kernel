# Replacement roadmap — Semantic Core + Agent Experience Consolidation

**Strategy:** consolidate authority first; expose stable agent contracts only after the underlying semantics are stable enough to describe.  
**Compatibility:** strangler migration, no flag-day rewrite.  
**Primary outcome:** one coherent decision kernel and one coherent machine/human operating surface whose dependency chain is explainable end-to-end.

## M0 — Freeze semantics and inventory architectural + agent-facing drift

**Goal:** safe baseline without behavior changes.

Deliverables:
- responsibility/authority registry for persistent models/stores;
- CLI golden contract fixture and current command inventory;
- crate/module dependency graph;
- duplicate concept inventory (graph, evidence, frontier, revision, event, execution state);
- inventory of `AGENTS.md`, prompts, agents, skills, templates, rules, embedded prompt strings and CLI examples;
- architecture lints advisory;
- supersession banners for competing roadmaps.

Exit/UAT:
- critical current CLI flows unchanged;
- every persistent model has owner/state class;
- every agent-facing asset has an owner and migration disposition;
- no new top-level domain during M0–M3 without ADR.

## M1 — Canonical facts, immutable objects and universal Evidence

**Goal:** eliminate data-authority ambiguity.

Deliverables:
- logical CanonicalEventLog and one canonical write path;
- legacy ledger/event mirror classification;
- canonical object/CAS rules;
- universal EvidenceRef/Bundle across planning/governed capabilities;
- Fact/Object/Projection/Ephemeral rule ratchet.

Exit/UAT:
- selected projections rebuild deterministically;
- mirror divergence cannot alter canonical reads;
- no new planning/UAT-specific evidence hierarchy required.

## M2 — Lifecycle and runtime consolidation

**Goal:** establish Goal → WorkItem → WorkflowDefinition → ExecutablePlan → Run.

Deliverables:
- slim Cycle contract;
- runtime-specific CycleStatus compatibility mapping/deprecation;
- Spine as desired planning manifest with explicit reconciliation;
- ExecutionFrontier as legal transition projection;
- ContinuationOptions as advisory next-step candidates;
- AgentResult compatibility adapter to ExecutionOutcome + Contribution.

Exit/UAT:
- one Cycle hosts multiple Runs without state ambiguity;
- approval/UAT waits are Run facts, correctly rendered in legacy views;
- planning reconciliation is deterministic/idempotent.

## M3 — SemanticGraph, Vault and Context consolidation

**Goal:** one graph substrate and explicit knowledge boundaries.

Deliverables:
- SemanticGraphProjection v2;
- adapter from current GraphProjection;
- ActiveGraph reduced to typed view/compatibility layer;
- Vault enforced/documented as KnowledgeSource;
- ContextCompiler sourcing Planning/Run/Memory/Graph/Vault adapters;
- context provenance/staleness explanation.

Exit/UAT:
- graph delete/rebuild equivalence;
- Vault edit cannot mutate canonical Decision;
- compatibility ActiveGraph derives from same facts;
- ContextCapsule reports source/provenance/staleness.

## M4 — Common Revision substrate + Decision Memory

**Goal:** implement durable decision memory without another history subsystem.

Deliverables:
- Revision/Object/Ref/CAS primitives;
- PlanRevision adapter/migration where practical;
- Decision Memory object model;
- HEAD/refs/reflog/reachability/merge-base;
- semantic memory diff/audit;
- compatible Fork reuse without forcing one payload semantics.

Exit/UAT:
- deterministic OIDs;
- CAS ref race test;
- branch/diverge/merge-base scenarios;
- semantic diff identifies decision/assumption/dissent/evidence changes;
- fresh-process recovery from canonical project state.

## M5 — Agent protocol, handoff and unified Authority

**Goal:** remove ambiguity in agent outputs and side-effect governance.

Deliverables:
- ExecutionOutcome / Contribution split;
- stabilize OrchestrationSynthesisReceipt with item dispositions;
- first-class Dissent/materiality;
- unified AuthorityEngine facade over permission/gate/risk/approval internals;
- one ActionProposal → AdmissionDecision → Capability → Receipt path.

Exit/UAT:
- material risk/evidence cannot silently disappear;
- denied action causes zero effect;
- approval-required action cannot execute early;
- admission is explainable from policy/facts/evidence.

## M6 — Packs, Targets, Tasks and convention-first CLI

**Goal:** make common workflows easy and extension boundaries safe before teaching them to agents.

Deliverables:
- Pack SDK vNext/conformance;
- Target/Task DAG registry;
- built-in `change`, `verify`, `ship`, `recover`, `audit` targets;
- porcelain CLI;
- configuration precedence + config explain;
- stable machine output conventions (`--format json` or equivalent);
- initial typed CommandSpec metadata behind public commands.

Exit/UAT:
- `run verify --dry-run` deterministic;
- side-effect task bypass rejected;
- UAT pack disable leaves core operational;
- legacy commands route same use cases or emit explicit deprecation;
- machine-output schemas are fixture tested.

## M7 — Agent Experience Contract: profiles, skills, instructions and CLI cheat sheets

**Goal:** migrate all agent-facing behavior to generated/versioned contracts and stop relying on prompt folklore.

Deliverables:
- provider-independent AgentProfile contract;
- SkillDefinition contract with Skill ≠ Capability invariant;
- InstructionCompiler and EffectiveInstructions;
- one CommandRegistry generating human help + machine schema + agent surfaces;
- contextual AgentCommandSurface/cheat sheet with tested examples;
- provider adapters consuming ExecutionRequest rather than internal stores;
- AgentExecutionReceipt provenance hashes;
- migration of AGENTS/prompts/agents/skills/templates/rules/embedded prompt strings;
- `context explain --instructions --commands` diagnostics;
- agent help/contract renderer.

Exit/UAT:
- representative agent completes `verify` without exploratory help calls;
- every command example in its surface parses and passes fixture tests;
- changing a command breaks a contract test before an agent prompt becomes stale;
- same AgentProfile runs through at least two fake/provider adapters without semantic change;
- enabling a Skill never grants a Capability;
- conflicting normative instruction sources fail closed with diagnostics;
- execution receipt identifies exact context/instruction/skill/command/tool/model contract hashes.

## M8 — Explanation engine and causal WHY

**Goal:** explain not just decisions and actions, but also what an agent was told and why.

Deliverables:
- ExplanationProof schema;
- `why`, `why-not`, `impact`;
- decision/debt/action/context/agent-execution profiles;
- historical `--at` where supported;
- unknown/counter-evidence handling;
- paths into AdmissionDecision, Memory, Evidence, Contribution and AgentExecutionReceipt;
- explanation of instruction/skill/command selection without chain-of-thought.

Exit/UAT:
- why decision reaches original Evidence/Contributions;
- why action reaches AdmissionDecision/policy snapshot;
- why agent-execution reports selected context, skills, instructions and command surface refs;
- no-path is reported as unknown/no recorded path, never fabricated causality.

## M9 — Remove compatibility debt and ratchet stable core

**Goal:** finish strangler migration.

Deliverables:
- remove expired duplicate event paths;
- remove planning Evidence duplicates;
- remove ActiveGraph authority path;
- remove AgentResult production writes;
- retire runtime-specific Cycle statuses after compatibility window;
- retire handwritten agent cheat sheets/duplicated command docs;
- remove obsolete monolithic prompt paths after parity;
- architecture + agent-contract drift rules become blocking;
- one normative roadmap/architecture entry point.

Exit/UAT:
- full critical workflow suite on clean and migrated repos;
- projection rebuild from scratch;
- no deprecated production path in dependency graph;
- no active agent asset references removed CLI/internal semantic names;
- docs have one normative roadmap and generated CLI examples are green.

## Dependency graph

```text
M0 → M1 → M2 → M3 → M4 → M5 → M6 → M7 → M8 → M9
           \          \              \         /
            └ context  └ handoff       └ agent ─┘
```

Spikes may run earlier. Production changes must respect authority/data-contract dependencies.
