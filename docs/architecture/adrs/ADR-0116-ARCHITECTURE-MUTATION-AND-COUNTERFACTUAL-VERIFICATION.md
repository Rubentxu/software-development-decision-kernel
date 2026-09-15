---
id: ADR-0116-ARCHITECTURE-MUTATION-AND-COUNTERFACTUAL-VERIFICATION
status: accepted
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-6-ac6-mutation-probes
implementation_evidence:
  - "crates/sddk-engine/src/architecture_mutation/mod.rs (public surface; state-class + safety + anti-encroachment doc)"
  - "crates/sddk-engine/src/architecture_mutation/types.rs (MutationKind 4-closed, GuardCheck 2-closed, MutationInjection 3-closed, MutationSpec/MutationProbe/MutationSuiteReceipt)"
  - "crates/sddk-engine/src/architecture_mutation/sandbox.rs (MutationSandbox disposable in-memory copy; apply_injection; evaluate_guard)"
  - "crates/sddk-engine/src/architecture_mutation/run.rs (run_mutation_probe/run_mutation_suite; critical_mutations + critical_guards covering AC-038-003)"
  - "crates/sddk-engine/src/architecture_mutation/tests.rs (28 tests incl. AC-UAT-010)"
  - "crates/sddk-engine/tests/ac6_ac4_witness_bridge.rs (AC6 witnesses drive AC4 contradiction_witnesses)"
  - "docs/architecture/specs/arch-spec-A3-S6-ac6-mutation-probes.md (cycle-bounded spec, 20 REQs)"
  - "NOTE: counterfactual stages (ADR-0116 decisions 1b) remain unimplemented; they are AC13"
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
