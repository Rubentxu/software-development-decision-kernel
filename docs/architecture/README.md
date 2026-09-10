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
| **M0** | Inventory + semantic/agent freeze | pending |
| **M1** | Canonical fact log + CAS + Evidence | pending |
| **M2** | Lifecycle consolidation | pending |
| **M3** | SemanticGraph + Vault + Context | pending |
| **M4** | Decision Memory extension (substrate DONE in CDD-MEMORY-001..005) | partial |
| **M5** | Agent protocol + AuthorityEngine facade | pending |
| **M6** | Packs + Targets + Tasks + CLI convention-first | pending |
| **M7** | Agent Experience Contract (**NEW**) | partial — M7.1 (v1.160.0) + M7.1B (v1.161.0) + M7.1C (v1.162.0) + M7.2 (v1.163.0) + M7.4 (v1.164.0) + M7.3 (v1.165.0) + M7.5 (v1.166.0) + M7.6 (v1.166.2) shipped + AGENTS.md governance merged (v1.166.1) + M7.7 runtime admission wire (v1.166.3) + M7.8 operator surface `sddk dev skills list/verify` (v1.166.4); placeholder-to-real skills still M9+ |
| **M8** | WHY engine + causal explanation | partial — M8.0 `sddk dev graph list/edges/projection` operator surface for the H9 Active Graph & Cockpit engine (v1.166.5) + M8.1 `sddk dev graph why` causal queries (why/debt-why/decision-why) for the WHY engine (v1.166.6) + M8.2 `sddk dev cockpit view {overview,journal,timeline,execution}` operator surface for the H9 Cockpit Views engine (v1.166.7) + M8.3 `sddk dev cockpit obs {providers,usage,assurance,handoff,experiments}` operator surface for the H9 Cockpit Observability engine (v1.166.8); engine modules shipped (`active_graph.rs` 588 LoC + `why_queries.rs` 574 LoC + `cockpit_views.rs` 495 LoC + `cockpit_observability.rs` 464 LoC) and all four now have CLI surfaces; live-cycle `ActiveGraphInput` auto-derivation remains M8.4+ |
| **M9** | Remove compat debt | pending |

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
