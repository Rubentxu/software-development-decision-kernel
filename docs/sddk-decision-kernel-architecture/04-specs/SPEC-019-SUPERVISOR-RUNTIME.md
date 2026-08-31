# SPEC-019 — Supervisor Runtime

**Status:** Proposed — refined for dynamic workflows

## Purpose
Provide global cognitive coordination without turning an LLM into a scheduler.

## Supervisor responsibilities
The Supervisor MAY:
- interpret user goal and risk;
- select a canonical/adaptive/exploratory WorkflowTemplate;
- propose WorkflowIR through the compiler;
- request bounded graph expansion;
- resolve ambiguous branches;
- replan when assumptions/evidence become invalid;
- choose among semantically different recovery strategies;
- request human input or critic/adjudicator;
- alter priorities/budgets within policy.

The Supervisor MUST NOT directly implement or mutate:
- scheduler queues/locks/leases;
- retry/backoff timers;
- join/loop state;
- circuit breaker state;
- event persistence;
- capability policy;
- raw privileged side effects.

## Inputs
`OrchestratorSignal` and `PlanningContext` include only relevant graph/contract state, not raw event dumps.

## Outputs
Typed proposals:
- `SelectWorkflowTemplate`;
- `CompileWorkflowRequest`;
- `ExpansionProposal`;
- `RerouteDecision`;
- `ReplanProposal`;
- `RequestHumanDecision`;
- `Stop/Escalate`.

## Dynamic workflow rule
Supervisor output is not executable authority. Compiler/Validator/Runtime transform or reject it.

## Delegation
Workers cannot recursively spawn arbitrary workers by default. A worker can return discovered work units/expansion proposal; spawn authority remains with runtime after validation.

## Failure
Supervisor is itself routable and can fail over. Minimal deterministic safety still supports cancel, circuit breaker and bounded retry if no cognitive route is available.

## Acceptance criteria
- no dynamic node appears without graph-expansion events;
- same deterministic event set does not invoke Supervisor unnecessarily;
- Supervisor proposal schema validation is mandatory;
- every cognitive decision records context/model/policy/output hashes.
