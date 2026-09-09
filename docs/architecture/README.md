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
| **M7** | Agent Experience Contract (**NEW**) | pending |
| **M8** | WHY engine + causal explanation | pending |
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
