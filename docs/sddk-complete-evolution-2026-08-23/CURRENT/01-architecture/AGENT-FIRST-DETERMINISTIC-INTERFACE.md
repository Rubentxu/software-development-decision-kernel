# Agent-First Deterministic Interface

**Status:** Proposed target architecture

## 1. Problem statement

SDDK's low-level CLI is a strong deterministic interface for humans and debugging, but it is often too mechanical as the normal LLM interface.

The agent can end up reconstructing:

```text
transition IDs
requirements
gate receipts
artifact locations
lease ownership
fencing tokens
program/argument tuples
status-query sequences
```

That knowledge belongs increasingly in Rust/Application Services.

## 2. Boundary rule

```text
Agent:
  intent
  uncertainty
  semantic goal
  typed domain/cognitive result

Rust:
  current state
  operation planning
  sequence/dependencies
  idempotency
  authorization
  effects
  caching
  postconditions
  persistence
  reporting
```

## 3. No-loss rule

The new interface is a **semantic façade over the full deterministic behavior**.

It MUST NOT eliminate:

- a gate;
- a verifier;
- an artifact;
- a report;
- a receipt;
- a metric;
- a knowledge update;
- a safety check;
- a human approval;
- a retry/no-progress rule.

A high-level GoalRun is successful only when all goal obligations are closed.

## 4. Three surfaces

```text
                 Rust Application Services
                          │
             ┌────────────┼────────────┐
             ▼            ▼            ▼
       Low-level CLI  High-level CLI  Agent Tool API
       debug/expert      goals        stdio/MCP/host
```

Business rules are never implemented in adapters.

## 5. DecisionSnapshot

Replace routine state-discovery call chains with one typed snapshot:

```yaml
decision_snapshot:
  project:
    id: ...
    adopted: true
  repository:
    branch: ...
    head: ...
    clean: true
  cycle:
    id: ...
    state: BUILD
    lease:
      valid: true
      fencing_token_ref: internal
  workflow:
    template: ...
    ir_revision: ...
  knowledge:
    quality: C2
    missing: []
    stale: []
  graph:
    revision: ...
  evidence:
    present: [...]
    missing: [...]
  reports:
    present: [...]
    required: [...]
  available_goals:
    - cycle.verified
  blockers: []
  fingerprint: sha256:...
```

Sensitive/internal mechanics can be referenced without requiring the LLM to propagate them manually.

## 6. Goal

A Goal expresses desired invariants, not shell steps:

```text
project.ready
cycle.started
cycle.verified
cycle.closed
assurance.complete
release.complete
knowledge.ready
```

## 7. OperationContract

Operations declare:

```text
requires
produces
requires_invariants
establishes_invariants
effects
idempotency
retry semantics
cache semantics
report/evidence obligations
```

An operation may be:

- deterministic Rust service;
- semantic capability;
- human gate;
- subworkflow.

## 8. Planning

```text
Actual State
+ Desired Goal
+ Operation Registry
+ Policy/Budget
          ↓
     GoalPlanner
          ↓
      Execution DAG
```

Planner v1 is deterministic.

## 9. Reconciliation

```text
observe
→ determine missing obligations
→ choose ready operations
→ execute/request capability
→ verify postconditions
→ persist detailed outputs
→ event
→ observe again
```

This makes retry/resume robust.

## 10. Cognitive suspension

A cognitive operation does not force the main LLM to understand the entire deterministic workflow.

```yaml
goal_run:
  status: waiting
  request:
    capability: architecture.review
    context_capsule_ref: ...
    output_schema: architecture-review/v1
  resume_token: ...
```

The AgentHost obtains the typed result and resumes the deterministic run.

## 11. Report/evidence bundle

Each GoalRun maintains a live completeness view:

```text
GoalRun
  ├ reports
  ├ artifacts
  ├ evidence
  ├ receipts
  ├ metrics
  ├ human decisions
  └ obligation statuses
```

The final summary points to them.

It does not replace them.

## 12. Work avoidance

For safe operations:

```text
same operation version
+ same declared inputs
+ same relevant revisions
+ fresh prior verified output
= UP_TO_DATE / REUSED
```

Reuse itself is evented.

Effectful operations require explicit idempotency/postcondition semantics.

## 13. Agent tool surface

Preferred semantic surface:

```text
state
goal.plan
goal.apply
query
evidence.submit
```

Only relevant tools are exposed per capability/goal/state.

## 14. Persistent local adapter

A later local hot process such as:

```text
sddk serve --stdio
```

may amortize:

- process startup;
- project identity resolution;
- storage opening;
- registry loading;
- workflow loading.

It is an optimization adapter, not a new authority.

## 15. Netstack3 lineage

The architectural lesson applied here is:

```text
understand complexity
→ encode invariants/contracts
→ preserve guarantees
→ expose simpler reliable boundary
```

We import that principle, not a networking architecture.
