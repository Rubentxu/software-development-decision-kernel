# SDDK — Canonical Architecture & Roadmap

> **This is the current normative architecture and roadmap for SDDK.**
> All other architecture, consolidation, and evolution packages in the
> repository are **historical / superseded** unless explicitly imported as
> current WorkItems through reconciliation.

## Package

- **Canonical source**: `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/`
- **MANIFEST.sha256** (72 documents, 0 broken internal links)
- **Adopted**: 2026-09-09
- **Supersedes**: prior competing roadmap narratives listed in
  `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/05-INTEGRATION/SUPERSESSION.md`

## Purpose

SDDK has enough functionality. The next evolution is to make that
functionality **coherent, explainable, agent-friendly and difficult to
misuse**. The package consolidates two coupled problems:

1. **Semantic Core Consolidation** — one authority per concept, one
   canonical fact log, one Evidence model, one Semantic Graph, one
   Revision substrate, one Authority Engine and a strict
   Goal → WorkItem → WorkflowDefinition → ExecutablePlan → Run chain.
2. **Agent Experience Consolidation** — agents, prompts, skills,
   examples and CLI knowledge become typed/versioned contracts
   compiled from the same product model instead of hand-maintained
   prompt folklore.

## Target product sentence

> **SDDK is a deterministic decision kernel for agent-assisted software
> development: it records canonical facts and evidence, governs side
> effects, preserves decision provenance and dissent, derives
> executable workflow state, compiles bounded context and effective
> instructions, and exposes a versioned command contract that humans
> and agents can use safely.**

SDDK is **not** an IDE, graph database, vector-memory product, wiki
engine, CI server or general-purpose agent framework.

## Architecture (essential)

```text
                Humans / Agent Adapters
                         │
                  Targets / Tasks
                         │
          ┌──────────────┴──────────────┐
          ▼                             ▼
  Context Compiler              Instruction Compiler
          │                             │
          │                    Agent Command Surface
          │                             │
          └──────────────┬──────────────┘
                         ▼
                   ExecutionRequest
                         │
                         ▼
                       Agent
                         │
               ┌─────────┴─────────┐
               ▼                   ▼
       ExecutionOutcome       Contribution
               │                   │
               └─────────┬─────────┘
                         ▼
               Synthesis / Decisions
                         │
      ┌──────────────────┼──────────────────┐
      ▼                  ▼                  ▼
 Canonical Event Log     CAS          Authority Engine
      │                                      │
      └──────────► projections / memory ◄────┘
                         │
                  Semantic Graph
```

## Roadmap (M0 → M9)

| Milestone | Title | Status |
|---|---|---|
| **M0** | Inventory + semantic/agent freeze | delivered — 7 deliverables D1..D7 closed by cycle `m0-inventory-baseline` (v1.151.4): responsibility registry (44 entries, schema v1), CLI golden fixtures (5 subcommands), crate dependency graph (mermaid, 0 SCCs >1), duplicate concept map, agent-asset inventory (320+ files across 69 agents/204 skills/41 prompts/2 templates with 11 smell detectors), architecture lints advisory (12 ARCH-SC + 8 ARCH-A), supersession banners across 6 historical docs |
| **M1** | Canonical fact log + CAS + Evidence | delivered — cycle `m1-canonical-fact-log` (v1.153.0): `CanonicalEventLog` port + `CasObjectStore` + universal `EvidenceRef/Bundle` + 4-state-class rule ratchet (Fact/Object/Projection/Ephemeral) |
| **M2** | Lifecycle consolidation | delivered — `pln-ledger-002-persist-planning-graph` (v1.86.0) + `pln-ledger-003-cross-storage-spine-import` (v1.87.0): Slim Cycle contract + Planning SSOT + Spine reconciliation deterministic + idempotent |
| **M3** | SemanticGraph + Vault + Context | delivered — cycle `m3-semantic-graph-vault` (v1.154.0): `SemanticGraphProjection` v2 + ActiveGraph reduced to typed view layer + Vault as `KnowledgeSource` + `ContextCompiler` with provenance/staleness |
| **M4** | Decision Memory extension (substrate DONE in CDD-MEMORY-001..005) | delivered — CDD-MEMORY-001..005 cycles (v1.146.0..v1.151.0) closed the Revision/Object/Ref/CAS substrate; CDD-MEMORY-004 (v1.150.0) added reflog history persistence + git-like branch mutation ops (cherry_pick / revert / amend / reset / delete_ref); CDD-MEMORY-005 (v1.151.0) closed Soft/Mixed reset + tombstone GC + true inverse-patch revert + ADR-093 acceptance |
| **M5** | Agent protocol + AuthorityEngine facade | delivered — cycle `m5-authority-engine-facade` (v1.152.0) introduces the unified `AuthorityEngine` facade + bridge policy table; `m5-unified-authority-runner` (v1.156.0) wires the runner as pre-gate across 6 CLI call sites (engine-internal `auth.validate()` calls remain as defense-in-depth compat mirrors until M9 cleanup) |
| **M6** | Packs + Targets + Tasks + CLI convention-first | delivered — three sub-cycles: `m6-1-convention-first-cli` (v1.157.0) introduced the convention-first CLI surface (5 router modules + typed `CommandSpec` introspection); `m6-2-target-task-dag` (v1.158.0) added `Target/Task` DAG registry with Kahn topo sort + 4 built-in targets; `m6-3-dag-execution` (v1.159.0) closed with `DagExecutor` running tasks in topological order with per-task authority gate |
| **M7** | Agent Experience Contract (**NEW**) | delivered — M7.1 (v1.160.0) + M7.1B (v1.161.0) + M7.1C (v1.162.0) + M7.2 (v1.163.0) + M7.4 (v1.164.0) + M7.3 (v1.165.0) + M7.5 (v1.166.0) + M7.6 (v1.166.2) shipped + AGENTS.md governance merged (v1.166.1) + M7.7 runtime admission wire (v1.166.3) + M7.8 operator surface `sddk dev skills list/verify` (v1.166.4) + M7.9 placeholder-to-real skills shipped (v1.167.8): the three M7.5 contractual placeholders (`core.contract-review@v1`, `core.workflow-orchestration@v1`, `core.release-planning@v1`) materialize as real `SKILL.md` files in the bundle, with a bridge extension that recognizes dotted frontmatter names so the runtime admission gate admits them instead of warning |
| **M8** | WHY engine + causal explanation | delivered — M8.0 `sddk dev graph list/edges/projection` operator surface for the H9 Active Graph & Cockpit engine (v1.166.5) + M8.1 `sddk dev graph why` causal queries (why/debt-why/decision-why) for the WHY engine (v1.166.6) + M8.2 `sddk dev cockpit view {overview,journal,timeline,execution}` operator surface for the H9 Cockpit Views engine (v1.166.7) + M8.3 `sddk dev cockpit obs {providers,usage,assurance,handoff,experiments}` operator surface for the H9 Cockpit Observability engine (v1.166.8) + M8.4 `--from-cycle <cycle-id>` auto-derives `ActiveGraphInput` from the cycle's archive manifest for both `view` and `obs` (v1.166.9) + M8.5 `## Commit parents` manifest section ⇒ `parent_of` edges with Unicode `→` / ASCII `->` / fat `=>` arrow support, both SHAs must appear in the known commit list (v1.167.2) + M8.6 per-node + per-edge `ProvenanceRef` metadata threaded through `ActiveGraphInput` ↔ `ActiveGraphProjection` so every projected entity is attributable back to the byte locator that produced it (v1.167.3) + M8.7 `sddk dev cockpit diff --cycle-a <id> --cycle-b <id>` (or `--input-a` / `--input-b`) cross-input drift detection via `sddk_engine::active_graph_drift::DefaultDriftEngine`, classifies every node + edge as `added` / `removed` / `changed` with field-level attribution (v1.167.4) + M8.8 `sddk dev cockpit digest --cycle <id>` (or `--input`) stable SHA-256 projection digest via `sddk_engine::active_graph_digest::ProjectionDigest`, two kinds (`strict` includes `recorded_at`; `content` strips it), cheap equality check before expensive diff (v1.167.5); all five engine modules ship CLI surfaces (`active_graph.rs` + `active_graph_drift.rs` M8.7 + `active_graph_digest.rs` M8.8 + `why_queries.rs` + `cockpit_views.rs` + `cockpit_observability.rs`); M9+ (live-mode streaming from kernel ledger, interactive drift sessions) remain future work |
| **M9** | Remove compat debt | delivered — INC-DEBT-018 (3-event supersede invariant doc) + INC-DEBT-019 (deterministic `now_ms` in `Engine::cycle_supersede`) closed in v1.167.0 (M9.1 + M9.2) + INC-DEBT-021 (pre-existing clippy baseline) + INC-DEBT-022 (pre-existing `dist_succeeds_with_valid_bundle` flake) closed in v1.167.1 (M9.3 + M9.4); the four vault-side records corresponding to these (INC-021-9b3e7f1a + INC-022-4c7a1e2f + INC-027-4e2a3c26 + INC-013 closed as side effect of DW-RUNTIME-005 S6b slice 2) were reconciled from `status: open` to `status: closed` at the inc-hygiene-2026-09-11 audit, restoring the doc/ledger coherence that the M9 row's "zero open debt records at v1.167.1" claim had implied; 32 open vault records remain (genuine pre-existing debt across clusters CL-01..CL-DKA-MANAGED-CLOSURE, all out of scope for the M9 scope-bounded cycles); debt-report schema `cycle_id` pattern widened to admit product cycles |

## 12 non-negotiable invariants

1. One canonical authority per concept.
2. Four state classes: Fact / Object / Projection / Ephemeral.
3. SDLC lifecycle semantics are explicit (PlanRevision lifecycle).
4. Common Revision substrate shared by all durable history.
5. One Semantic Graph (per concept) with optional projections.
6. The Vault is human/knowledge source, never a runtime authority.
7. Universal Evidence model spans all subsystems.
8. **Prompt text is never architecture** (EffectiveInstructions compiled).
9. **Skill ≠ Capability** (skill implies nothing about permissions).
10. **CLI docs from one source** (CommandRegistry).
11. **Examples are executable** (validation at compile/registry time).
12. **Knowledge of command ≠ permission** (declared vs granted).

## Reading order

1. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/00-EXECUTIVE-PROPOSAL.md`
2. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/01-ARCHITECTURE/TARGET-ARCHITECTURE.md`
3. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/ROADMAP.md`
4. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/UAT-ACCEPTANCE.md`
5. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/MIGRATION-AND-DEPRECATION.md`
6. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/` (17 ADRs)
7. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/` (18 SPECs)
8. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/07-AGENT-EXPERIENCE/OVERVIEW.md` (M7)
9. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-005-SEMANTIC-GRAPH-AND-WHY.md` (M8)

## Repository-native surfaces (crosswalk)

The package's `02-ADRS/` and `02-SPECS/` are mirrored into this entry point so
the canonical architecture is reachable without traversing the package folder.
Per SUPERSESSION.md ADR preservation rule, the package's original IDs are kept
in each file's frontmatter (`package_local_id`); the repository-native ID is the
authoritative one for new references.

### Architecture ADRs (`adrs/`)

| Repo-native ID | Title | Package ID |
|---|---|---|
| `ADR-0094` | One canonical fact log | `ADR-001` |
| `ADR-0095` | Four state classes | `ADR-002` |
| `ADR-0096` | SDLC lifecycle semantics | `ADR-003` |
| `ADR-0097` | Common Revision substrate | `ADR-004` |
| `ADR-0098` | One Semantic Graph | `ADR-005` |
| `ADR-0099` | Vault as human knowledge source | `ADR-006` |
| `ADR-0100` | Universal Evidence | `ADR-007` |
| `ADR-0101` | Agent outcome contribution synthesis | `ADR-008` |
| `ADR-0102` | Unified Authority Engine | `ADR-009` |
| `ADR-0103` | Target/Task porcelain | `ADR-010` |
| `ADR-0104` | Pack extension boundary | `ADR-011` |
| `ADR-0105` | Configuration conventions | `ADR-012` |
| `ADR-0106` | Typed instruction compilation | `ADR-013` |
| `ADR-0107` | One Command Registry | `ADR-014` |
| `ADR-0108` | Skill is not Capability | `ADR-015` |
| `ADR-0109` | Provider-independent Agent Profiles | `ADR-016` |
| `ADR-0110` | Agent execution provenance | `ADR-017` |

Range ADR-0094..0110 was chosen because the previous repository surface
(`docs/adr/`) ends at ADR-0080, leaving ADR-0094..0110 free and avoiding
collision with the `docs/sddk-decision-kernel-architecture/03-adrs/` range
(ADR-020..074). See [`adrs/ADR-0094-ONE-CANONICAL-FACT-LOG.md`](adrs/ADR-0094-ONE-CANONICAL-FACT-LOG.md)
for the canonical example of the crosswalk frontmatter.

### Architecture SPECs (`specs/`)

| Repo-native ID | Title | Package ID |
|---|---|---|
| `arch-spec-001` | Canonical authority | `SPEC-001` |
| `arch-spec-002` | Lifecycle model | `SPEC-002` |
| `arch-spec-003` | Revision substrate | `SPEC-003` |
| `arch-spec-004` | Decision Memory | `SPEC-004` |
| `arch-spec-005` | Semantic Graph and why | `SPEC-005` |
| `arch-spec-006` | Knowledge vault context | `SPEC-006` |
| `arch-spec-007` | Agent protocol and handoff | `SPEC-007` |
| `arch-spec-008` | Authority and side effects | `SPEC-008` |
| `arch-spec-009` | Target task workflow | `SPEC-009` |
| `arch-spec-010` | Pack SDK | `SPEC-010` |
| `arch-spec-011` | Observability views | `SPEC-011` |
| `arch-spec-012` | Configuration | `SPEC-012` |
| `arch-spec-013` | Agent experience contract | `SPEC-013` |
| `arch-spec-014` | Instruction compiler | `SPEC-014` |
| `arch-spec-015` | Command registry and agent surface | `SPEC-015` |
| `arch-spec-016` | Skill contract | `SPEC-016` |
| `arch-spec-017` | Agent profiles and provider adapters | `SPEC-017` |
| `arch-spec-018` | Agent execution provenance | `SPEC-018` |

The `arch-spec-` prefix avoids collision with a future top-level `docs/specs/`.

## Repository historical packages (superseded)

These folders contain previous design context. They remain available for
provenance, but **are no longer authoritative**:

- `../sddk-decision-kernel-architecture/` (pre-canonical ADR-020..074 home)
- `../sddk-2.0-architecture-consolidation/` (2026-08-11 v1.9.1 baseline consolidation)
- `../sddk-complete-evolution-2026-08-23/` (CURRENT/HISTORY bundle)
- `../sddk-stabilization-plan/` (stabilization phase)
- `../SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/` (HX0..HX7 human-experience line)
- `../responsibility-separation/SPEC.md` (cero-intrusion rule)
- `../ARCHITECTURE-MODEL.md` (older model draft)
- `../agent-models-registration.md`, `../agent-reconciliation.md` (agent evolution notes)
- `../evolutivo-*.md` (older evolutivo notes)

When reconciling WorkItems, import concepts from these only via
explicit reconciliation — they do not have authority on their own.

## Provenance

- **Adopted by**: cycle
  `p-63676b11dc0ef88f-architecture-adoption-m0-supersession`
- **Adoption date**: 2026-09-09
- **Total documents covered**: 72
- **ADRs**: 17 (`ADR-001..017` within the package)
- **SPECs**: 18 (`SPEC-001..018` within the package)
- **UAT cases**: 22 (`UAT-01..UAT-22`)
- **Manifest SHA-256**: see
  `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/MANIFEST.sha256`

## See also

- **AGENTS.md §2.7..§2.10** — semantic ownership, agent experience,
  extension discipline, consolidation rule.
- **Cycle dir**: `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-semantic-core-agent-experience-adoption/`
- **Planning artifacts**: 9 (~1726 lines total) covering M0/M7/M8
  planning notes, CLI crosswalk, engine crosswalk, LAUNCH-READY.
