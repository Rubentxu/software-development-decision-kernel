# ADR-043 — Change-Scoped Verification as a First-Class SDDK Service

**Status:** Accepted (user-approved 2026-09-03)

## Context

SDDK currently spends too much execution time and agent/token budget repeatedly running broad test suites during implementation. The repository also contains conflicting guidance: some apply/TDD instructions already prefer focused tests, while global agent guidance still treats `cargo test --workspace` as a routine pre-commit gate.

This is both a performance problem and an authority problem. A coding agent should not decide test scope by intuition, shell-command probing or "run everything to be safe". SDDK should know the active change, the system under test (SUT), the relevant dependency/test graph and the evidence already collected, then return the smallest conservative test plan that can justify the current implementation step.

The full suite remains valuable as a final integration boundary. It should not be paid repeatedly inside every `apply` iteration.

## Decision

SDDK SHALL provide a first-class **Change-Scoped Verification Service** with four explicit domain concepts:

1. `ActiveChangeSet` — the exact code/configuration delta currently being implemented.
2. `SutImpactGraph` — a typed, provenance-aware projection connecting changed artifacts, SUT nodes, dependency boundaries and tests.
3. `TestSelectionPlan` — an ordered, explainable set of progressive test batches selected for the current change.
4. `TestEvidenceReceipt` — durable evidence binding the executed selection to the source revision/change-set and graph revision.

### Canonical lifecycle rule

- `shape` MAY preview expected verification impact.
- `apply` MUST use progressive change-scoped verification.
- `converge` MAY extend/reuse scoped evidence according to risk and missing obligations.
- `verify` MUST run the authoritative full test/assurance suite required by the project profile.
- `release` consumes a fresh successful `verify` receipt; it does not invent another testing authority.

**Normal `apply` execution MUST NOT run the full suite.** The normal full-suite boundary is `verify`.

### Fail-closed rule

If SDDK cannot conservatively map a change to an adequate test set, it MUST NOT silently skip tests and MUST NOT silently escalate to the full suite inside `apply`.

Instead it returns a typed blocked/insufficient-mapping outcome containing:

- unmapped change/SUT nodes;
- missing dependency/test relations;
- reason the scoped plan cannot be justified;
- remediation needed to restore mapping confidence;
- whether an explicit `verify` is required before progression.

An operator may explicitly request broader debugging tests, but that override is recorded as evidence and does not redefine the normal `apply` contract.

### Deterministic MVP

The first implementation is conservative and deterministic. It uses:

- Git active changes (`git diff`, staged/unstaged/base-to-head);
- workspace/package/target ownership;
- build dependency and reverse-dependency relations;
- colocated unit-test and package integration-test ownership;
- explicit SUT-to-test mappings for ambiguous/cross-cutting tests;
- contract/public-boundary escalation rules.

For Rust, `cargo metadata` is an adapter input for the workspace/dependency graph and cargo/nextest are execution adapters. SDDK owns the selection semantics and explanation; agents do not construct ad-hoc nextest filters as planning logic.

Coverage traces, historical failures and learned strategies MAY enrich later selection confidence, but MUST NOT become the sole reason for omitting deterministic required tests.

### Graph authority

The SUT/test/change model is a typed projection/subgraph of SDDK's canonical graph/evidence model. It MUST NOT create a second independent graph of authority.

Minimum node/edge semantics include:

- nodes: change, source artifact, package/module/symbol where available, contract/capability, test, evidence receipt;
- edges: `TOUCHES`, `OWNS`, `DEPENDS_ON`, `REVERSE_DEPENDS_ON`, `TESTS`/`COVERS`, `CONTRACT_DEPENDENCY`, `PRODUCED_EVIDENCE`;
- every inferred edge carries provenance and confidence/source kind.

### Agent authority

Coding/apply agents consume the service. They do not choose broad test scope by intuition.

Until the service is shipped, agents follow the bootstrap policy defined in `prompts/sddk/change-scoped-testing.md`: use repository evidence to approximate the same bounded policy and reserve full-suite execution for explicit `verify`.

After `TEST-APPLY-001` ships, manual test-scope selection in normal `apply` becomes a protocol violation.

## Consequences

### Positive

- much faster implementation feedback loops;
- lower LLM/tool-call/token cost;
- test executions become explainable and auditable;
- unchanged areas are not repeatedly retested during `apply`;
- full-suite integration confidence is preserved at `verify`;
- test selection itself becomes measurable and improvable through Workflow Lab later.

### Costs / risks

- SUT/test mapping must be maintained and measured;
- under-selection is a serious correctness risk;
- unknown mappings must fail closed rather than optimize aggressively;
- evidence invalidation needs deterministic source/change/graph identities.

## Reliability metric

Efficiency is never the only promotion criterion. The primary quality guard is the **escape rate**:

> `verify` fails for a regression that passed the scoped `apply` plan.

Any future learned/adaptive selection strategy must demonstrate non-inferior escape rate and invariant coverage before promotion, in addition to time/token savings.

## Relationship to existing decisions

- ADR-039 remains the broader risk/evidence-driven CONVERGE strategy; this ADR establishes the earlier deterministic apply-time foundation.
- ADR-042 continues to govern test-tooling ownership/migration; this ADR governs *which tests are selected and when*.
- H8 adaptive verification builds on this service rather than replacing its deterministic graph/evidence core.
