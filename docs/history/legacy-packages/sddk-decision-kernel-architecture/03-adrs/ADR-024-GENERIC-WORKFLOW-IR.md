# ADR-024 — Generic Workflow Template, IR and Runtime Algebra

**Status:** Accepted (supersedes the narrower “fixed node-kind DAG only” interpretation)

## Context
The current SDDK domain hard-codes SDD `Phase` and `CyclePath`. The first Workflow Runtime proposal removed those names but still assumed a mostly predeclared DAG. Recent dynamic workflow approaches show value in moving orchestration into executable runtime state with dynamic fan-out, loops and intermediate state kept outside the model context.

## Decision
The kernel SHALL distinguish three levels:

1. **WorkflowTemplate** — stable intent, invariants, policies and allowed capabilities.
2. **WorkflowIR** — validated executable provider-neutral plan using a small workflow algebra.
3. **ExecutionGraph** — durable runtime instance that may expand through validated events.

Kernel algebra:

```text
Task, Sequence, Parallel, Map, Join, Race, Choice,
Loop, Gate, Wait, SubWorkflow, Compensate
```

Agentic patterns are compositions over this algebra.

`Phase`, `CyclePath`, `Explore`, `Specify`, `Design` and similar domain labels are pack-owned metadata, not kernel control types.

An LLM may propose WorkflowIR or `ExpansionProposal`, but cannot execute arbitrary generated orchestration code with kernel authority.

## Consequences
- Supports static, adaptive and exploratory workflows on one runtime.
- Dynamic decomposition can create N work items discovered at runtime.
- Replay/fork/diff can compare graph revisions.
- The IR validator becomes a security and architecture boundary.
- Legacy path compilers remain possible during migration.
