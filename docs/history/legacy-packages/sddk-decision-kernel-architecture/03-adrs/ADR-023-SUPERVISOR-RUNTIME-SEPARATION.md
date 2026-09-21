# ADR-023 — Separate Cognitive Supervisor from Deterministic Workflow Runtime

**Status:** Accepted (refined 2026-08-19)

## Context
A central orchestrator is valuable for goal interpretation and replanning, but using an LLM as the scheduler creates non-deterministic retries, joins, budgets and recovery behavior. Dynamic workflows add another risk: letting the model directly mutate execution control structures.

## Decision
Maintain a strict separation:

**Supervisor** may select a template, propose/compile an execution strategy, request graph expansion, resolve ambiguity and replan.

**Runtime** validates and owns execution state: graph revisions, scheduling, joins, loops, retries, timeouts, budgets, leases, idempotency, side-effect governance and persistence.

The Supervisor never receives direct authority to mutate runtime state. Its output is a typed command/proposal validated by application/runtime policy.

## Consequences
- Dynamic workflows remain auditable and replayable.
- Provider/model changes do not change control semantics.
- Most event reactions remain deterministic and cheap.
- The Supervisor itself can fail over like any other AgentExecution.
- Some apparent agent autonomy is intentionally constrained in favor of operational reliability.
