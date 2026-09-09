# Software Development Decision Kernel — Semantic Core + Agent Experience Consolidation

**Status:** proposed normative replacement package  
**Date:** 2026-09-09  
**Scope:** `Rubentxu/software-development-decision-kernel`  
**Supersedes:** the previous `SDDK-Semantic-Core-Consolidation-2026-09-09` proposal and, once adopted, the competing live roadmap narratives listed in `05-INTEGRATION/SUPERSESSION.md`.

## Purpose

SDDK has enough functionality. The next evolution is to make that functionality **coherent, explainable, agent-friendly and difficult to misuse**.

This package consolidates two coupled problems:

1. **Semantic Core Consolidation** — one authority per concept, one canonical fact log, one Evidence model, one Semantic Graph, one Revision substrate, one Authority Engine and a strict Goal → WorkItem → WorkflowDefinition → ExecutablePlan → Run chain.
2. **Agent Experience Consolidation** — agents, prompts, skills, examples and CLI knowledge become typed/versioned contracts compiled from the same product model instead of hand-maintained prompt folklore.

The package is designed to replace the current roadmap through a strangler migration, not a flag-day rewrite.

## Target product sentence

> **SDDK is a deterministic decision kernel for agent-assisted software development: it records canonical facts and evidence, governs side effects, preserves decision provenance and dissent, derives executable workflow state, compiles bounded context and effective instructions, and exposes a versioned command contract that humans and agents can use safely.**

SDDK is not an IDE, graph database, vector-memory product, wiki engine, CI server or general-purpose agent framework.

## The essential architecture

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

## What an agent knows

An agent is **not expected to discover the CLI by trial/error or repeatedly call `--help`**. For each task it receives a generated `AgentCommandSurface` containing only relevant command contracts, syntax, examples, preconditions, output schemas and side-effect class.

Knowing that a command exists does **not** grant permission to run it. Authority remains a separate runtime decision.

## Read order

1. [`00-EXECUTIVE-PROPOSAL.md`](00-EXECUTIVE-PROPOSAL.md)
2. [`01-ARCHITECTURE/TARGET-ARCHITECTURE.md`](01-ARCHITECTURE/TARGET-ARCHITECTURE.md)
3. [`01-ARCHITECTURE/RESPONSIBILITY-MAP.md`](01-ARCHITECTURE/RESPONSIBILITY-MAP.md)
4. [`07-AGENT-EXPERIENCE/OVERVIEW.md`](07-AGENT-EXPERIENCE/OVERVIEW.md)
5. [`03-ROADMAP/ROADMAP.md`](03-ROADMAP/ROADMAP.md)
6. [`03-ROADMAP/MIGRATION-AND-DEPRECATION.md`](03-ROADMAP/MIGRATION-AND-DEPRECATION.md)
7. [`03-ROADMAP/AGENT-ASSET-MIGRATION.md`](03-ROADMAP/AGENT-ASSET-MIGRATION.md)
8. [`05-INTEGRATION/SUPERSESSION.md`](05-INTEGRATION/SUPERSESSION.md)

## Normative priority

Within this package:

1. accepted ADRs define architecture decisions;
2. specifications define required behavior and contracts;
3. roadmap defines implementation dependency/order;
4. migration documents define compatibility/deprecation;
5. examples and cheat sheets are generated/tested reference material, not independent authority.

Older SDDK evolution folders remain historical evidence only after this package is adopted.
