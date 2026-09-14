---
id: ADR-0116-ARCHITECTURE-MUTATION-AND-COUNTERFACTUAL-VERIFICATION
status: proposed
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: null
accepted_by_cycle: null
implementation_evidence: []
superseded_by: []
related_adrs:
  - "ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS"
  - "ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY"
stale_after: 2027-03-14
---

# ADR-0116 — Architecture Mutation and Counterfactual Verification

## Context

A passing fitness test does not prove that it would detect the forbidden regression. Agents also benefit from evaluating a refactor before editing files.

## Decision

Add two verification techniques in stages:

1. **MutationProbe** — in an isolated sandbox/worktree, inject a bounded architectural violation and require the expected guard to fail.
2. **CounterfactualAssessment** — apply a proposed ownership/dependency graph delta to an ephemeral candidate projection and evaluate contracts without mutating the repository.

Mutation testing for a small critical contract set is part of Base conformance; broader counterfactual planning is post-Base/P2.

## Safety

- mutations never persist to the working tree;
- counterfactuals are EPHEMERAL and advisory;
- neither mechanism grants capability or bypasses AuthorityEngine;
- LLM-proposed graph deltas require deterministic validation before assessment.

## Consequences

SDDK can demonstrate that architectural ratchets actually protect boundaries and can guide agents toward safer refactors before code churn.
