# ADR-043 — Language-Agnostic Change-Scoped Verification as a First-Class SDDK Service

**Status:** Accepted (user-approved 2026-09-03)

## Context

SDDK must orchestrate software-development work for arbitrary repositories, languages, build systems and test frameworks. Repeatedly running broad test suites during implementation wastes execution time and LLM/tool budget, but hard-coding a Rust/Cargo optimisation would simply move the problem into the kernel.

A coding agent should not decide test scope from intuition, runner probing or a language-specific recipe. SDDK should know the active Git change, the affected systems under test (SUTs), the repository's project/test topology, dependency and contract edges, available verification capabilities, and the evidence already collected. It should then return the smallest conservative progressive plan that proves the current implementation step.

The same semantic model MUST work for a single-language project, a multi-module repository and a polyglot monorepo. Rust/Cargo is only one adapter family used by SDDK's own repository.

## Decision

SDDK SHALL provide a first-class **Change-Scoped Verification Service** with language-neutral domain concepts:

1. `ActiveChangeSet` — exact Git/source/configuration delta currently under implementation.
2. `ProjectTestTopology` — detected and declared repository topology: components, build units, test units, contracts, generated surfaces and verification capabilities.
3. `SutImpactGraph` — provenance-aware projection connecting changes to affected SUTs, dependencies, contracts and tests.
4. `TestSelectionPlan` — ordered explainable progressive verification batches.
5. `TestEvidenceReceipt` — durable evidence binding executed verification to change, graph, policy and toolchain revisions.

### Kernel versus adapters

The kernel owns semantic identity, impact propagation, selection policy, evidence freshness, invalidation, explanation and fail-closed behavior.

Language/build/test tooling lives behind adapters/capability providers. No runner is allowed to become planning authority.

Illustrative adapter families include, but are not limited to:

- Rust: Cargo metadata, cargo test/nextest;
- JVM: Maven/Gradle plus JUnit/TestNG/Kotest/Spock;
- JavaScript/TypeScript: npm/pnpm/yarn workspaces plus Jest/Vitest/Mocha/Playwright;
- Python: pyproject/tox/nox/Poetry/PDM plus pytest/unittest;
- Go: modules/workspaces plus `go test`;
- .NET: solution/project/MSBuild graph plus `dotnet test`;
- C/C++: CMake/Meson/Bazel plus CTest/framework adapters;
- generic build graphs such as Bazel, Buck2 or Pants;
- additional ecosystems through the same SPI/capability contract.

A repository MAY activate several adapter families simultaneously. Cross-language contract edges remain first-class graph relations.

### Canonical lifecycle rule

- `shape` MAY preview expected verification impact.
- `apply` MUST use progressive change-scoped verification.
- `converge` MAY extend/reuse scoped evidence according to risk and missing obligations.
- `verify` MUST run the authoritative full verification profile declared by the project.
- `release` consumes a fresh successful `verify` receipt and does not invent another testing authority.

**Normal `apply` execution MUST NOT run the complete repository verification profile merely to be safe.** The normal broad integration boundary is `verify`.

### Fail-closed rule

If SDDK cannot conservatively map a change to adequate verification evidence, it MUST NOT silently omit tests and MUST NOT silently run everything inside `apply`.

It returns a typed insufficient-mapping result containing:

- unmapped changed artifacts/SUTs;
- missing ownership, dependency, contract or test relations;
- unavailable verification capability/adapter when relevant;
- why the scoped plan cannot be justified;
- remediation required to restore mapping confidence;
- whether explicit full `verify` is required before progression.

An operator may explicitly request a broader diagnostic run. The override is receipted and does not redefine the normal `apply` contract.

### Deterministic MVP

The first implementation is conservative and deterministic. It combines:

- Git active changes (`base..HEAD`, staged and working-tree delta);
- detected component/build-unit ownership;
- build/runtime dependency and reverse-dependency relations supplied by adapters or explicit declarations;
- test discovery and stable test identity supplied by adapters;
- explicit SUT-to-test/contract mappings for ambiguous and cross-cutting verification;
- build/config/schema/generated-code change classification;
- public/contract-boundary escalation rules.

Coverage traces, historical failures and learned strategies MAY enrich later confidence, but MUST NOT become the sole reason for omitting deterministic required evidence.

### Project capability discovery

SDDK SHALL represent project tooling as data, not prompt assumptions. A `VerificationCapability` describes semantically what can be run (compile, type-check, lint, unit, integration, contract, E2E, security, mutation, UAT, etc.), the SUT scope it supports, selector abilities, cost hints and the adapter that executes it.

The planner requests semantic capabilities. Adapters translate a `TestBatch` into runner syntax. Agents MUST NOT probe CLI flags to rediscover a command that project capabilities already describe.

### Graph authority

The SUT/test/change topology is a typed projection/subgraph of SDDK's canonical evidence/graph model, not a second graph of authority.

Minimum node kinds include repository/workspace, component/build unit, source/config/schema/generated artifact, optional symbol, runtime/service boundary, contract, test/test-suite, verification capability and evidence receipt.

Minimum edge semantics include `TOUCHES`, `OWNS`, `BUILDS`, `DEPENDS_ON`, `RUNTIME_DEPENDS_ON`, `REVERSE_DEPENDS_ON`, `GENERATES`, `TESTS`, `COVERS`, `VALIDATES_CONTRACT`, `CONTRACT_DEPENDENCY`, `PRODUCED_EVIDENCE` and `INVALIDATES`.

Every inferred relation carries provenance, adapter/rule version and confidence/source kind.

### Agent authority

Coding/apply agents consume the semantic service. They do not choose broad test scope by language intuition.

Until the service ships, agents follow the language-neutral bootstrap policy in `prompts/sddk/change-scoped-testing.md`: discover the project topology once, reuse known commands/capabilities, map Git changes to SUTs and execute only justified progressive evidence.

After `TEST-APPLY-001` ships, manually constructing test scope in normal `apply` instead of consuming the service is a protocol violation.

## Consequences

### Positive

- one testing model works across languages and polyglot repositories;
- much shorter coding feedback loops;
- lower LLM/tool-call/token cost;
- test execution becomes explainable and auditable;
- unchanged components are not repeatedly retested in `apply`;
- project-specific tools remain replaceable adapters;
- full integration confidence remains at `verify`;
- selection quality can be measured and improved later in Workflow Lab.

### Costs / risks

- ecosystem adapters and topology discovery must be implemented incrementally;
- under-selection is a correctness risk;
- unknown mappings fail closed rather than optimise aggressively;
- polyglot contract boundaries require explicit cross-component modeling;
- evidence invalidation requires deterministic identities for source, topology, policy and toolchain.

## Reliability metric

Efficiency alone is never a promotion criterion. The primary quality guard is **escape rate**:

> `verify` detects a regression or violated obligation that passed the scoped `apply` plan.

Future adaptive selectors require non-inferior escape rate and invariant coverage, in addition to lower latency/cost.

## Relationship to existing decisions

- ADR-039 remains the broader risk/evidence-driven CONVERGE strategy; this ADR establishes the earlier deterministic apply-time foundation.
- ADR-042 continues to govern test-tooling ownership/migration; this ADR governs what evidence is selected and when.
- H8 adaptive verification builds on this service rather than replacing its deterministic topology/evidence core.
