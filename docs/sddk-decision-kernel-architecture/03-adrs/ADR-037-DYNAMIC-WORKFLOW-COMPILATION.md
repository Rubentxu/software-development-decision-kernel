# ADR-037 — Dynamic Workflow Compilation and Evented Graph Expansion

**Status:** Proposed

## Context
Canonical YAML workflows are easy to inspect but become rigid when the number of subtasks, affected files or verification paths cannot be known before execution. Recent programmatic/dynamic workflow designs demonstrate that orchestration state can live outside model context and use ordinary control constructs for fan-out, loops and aggregation.

## Decision
SDDK will support **dynamic workflows without executing arbitrary LLM-generated scripts as authority**.

Pipeline:

```text
Goal + WorkflowTemplate + capability snapshot + budgets
  → Supervisor/Planner proposal
  → WorkflowCompiler
  → WorkflowIR
  → WorkflowValidator
  → durable runtime
```

During execution, nodes may emit typed `ExpansionProposal`s. The runtime validates:
- allowed capabilities;
- graph/dependency validity;
- concurrency conflicts/worktree constraints;
- policy/side effects;
- node/depth/token/cost/time budgets;
- required gates/verifiers.

Approved expansions append canonical events and create a new graph revision.

## Alternatives rejected
- LLM owns scheduler entirely: insufficient determinism/recovery.
- Pre-generate every possible node: poor fit for discovery-heavy tasks.
- Execute generated JavaScript/Python directly: unacceptable policy and replay boundary for the kernel.

## Consequences
- Dynamic behavior is replayable/auditable.
- Templates remain compact.
- Compiler/validator complexity increases.
- Packs can choose static or dynamic strategy without kernel changes.
