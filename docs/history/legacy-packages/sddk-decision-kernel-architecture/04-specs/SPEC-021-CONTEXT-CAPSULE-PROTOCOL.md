# SPEC-021 — Context Capsule Protocol

**Status:** Proposed

## Purpose
Deliver minimal, sufficient, traceable context to workers and across failover.

## Schema

```yaml
capsule_id: ctx-...
for:
  workflow_run: wf-...
  node_run: nr-...
  attempt: at-...
objective: "Review target architecture"
definition_of_done:
  - "risks identified"
  - "decision schema returned"
scope:
  include: ["architecture", "runtime"]
  exclude: ["UI implementation details"]
constraints:
  - "no repository mutation"
decisions:
  accepted: []
  rejected: []
assumptions:
  - text: "Event Ledger remains authority"
    confidence: 1.0
open_questions: []
artifacts:
  must_read: []
  relevant: []
  fetch_on_demand: []
negative_knowledge:
  - claim: "Graph is source of truth"
    status: ruled_out
changes_since_parent: []
evidence_required: []
tools_allowed: []
budget:
  max_tokens: 80000
return_contract: schema://architecture-review/v1
recovery:
  previous_attempt: null
```

## Compilation inputs
- workflow/node objective;
- ledger decisions;
- graph neighborhood;
- artifact metadata/content selection;
- context-read history;
- change/staleness projection;
- pack policy.

## Read tracing
A capsule records offered references. Separate events record what the host/agent actually read when observable. Never claim read status merely because an artifact was included.

## Delta capsules
On continuation/failover:

```yaml
delta:
  added: []
  modified: []
  invalidated: []
  unchanged_reusable: []
```

## Negative knowledge
Explicitly retain disproven hypotheses/rejected options with evidence/reason to avoid repeated investigation.

## Context overflow
If route context limit is insufficient, Context Compiler can summarize/compress lower-priority context while preserving must-read refs and content hashes. This is an execution recovery before provider reroute when appropriate.
