# SPEC-043 — Language-Agnostic Change-Scoped Verification Service

**Status:** Accepted design target  
**Horizon:** H0 — Reconcile & Deterministic Foundations  
**Decision:** ADR-043

## 1. Purpose

For the code/configuration that is changing **right now**, SDDK must answer without assuming a language or runner:

1. What is the active change set?
2. What project/build/test topology exists in this repository?
3. Which SUT nodes can be affected?
4. Which verification capabilities and tests provide relevant evidence?
5. What is the cheapest safe next verification batch?
6. What evidence is already fresh and reusable?
7. Why was each check selected, widened, reused, invalidated or omitted?

`apply` uses this service to keep implementation feedback fast. `verify` remains the authoritative whole-project verification boundary.

## 2. Non-goals

The MVP does **not**:

- encode one language/build system in the kernel;
- predict impact from an LLM alone;
- trust historical coverage as the only selection signal;
- make agents discover runner flags by trial and error;
- replace `verify`;
- create a second graph database or authority model;
- silently execute every test when impact cannot be resolved;
- optimise for minimum test count at the expense of escaped regressions.

## 3. Language-neutral domain model

### 3.1 `ActiveChangeSet`

```text
ActiveChangeSet
├─ project_id
├─ work_item_id / run_id
├─ base_revision
├─ head_revision
├─ working_tree_digest
├─ changed_artifacts[]
│  ├─ path
│  ├─ change_kind: added | modified | deleted | renamed
│  ├─ staged
│  ├─ hunks[]?             # adapter/parser-provided
│  └─ symbols[]?           # optional when deterministically resolvable
└─ change_set_digest
```

The digest changes whenever a verification-relevant source/configuration input changes.

### 3.2 `ProjectTestTopology`

The topology represents one or many ecosystems in the same repository:

```text
ProjectTestTopology
├─ repository/workspaces[]
├─ components[]
├─ build_units[]
├─ source_artifacts[]
├─ configuration_surfaces[]
├─ schemas/contracts[]
├─ generated_surfaces[]
├─ tests/test_suites[]
├─ verification_capabilities[]
├─ dependency_edges[]
├─ explicit_mappings[]
└─ topology_revision
```

A polyglot monorepo may contain, for example, a JVM backend, TypeScript frontend, Python data service and Terraform/configuration surface under one topology revision.

### 3.3 SUT node kinds

Initial generic kinds:

```text
Repository
Workspace
Component
BuildUnit
SourceArtifact
ModuleOrNamespace
Symbol?                  # optional MVP enrichment
RuntimeService
ContractBoundary
Schema
ConfigurationSurface
GeneratedArtifact
TestUnit
TestSuite
VerificationCapability
EvidenceReceipt
```

Adapters may attach ecosystem-specific metadata, but core planning only depends on the generic kinds.

### 3.4 Typed graph edges

Minimum edge types:

```text
TOUCHES(change -> artifact)
OWNS(component/build-unit -> artifact)
BUILDS(build-unit -> artifact/component)
DEPENDS_ON(sut -> sut)
RUNTIME_DEPENDS_ON(sut -> sut)
REVERSE_DEPENDS_ON(sut -> sut)
GENERATES(source/schema -> generated-artifact)
TESTS(test -> sut)
COVERS(test -> sut)                       # explicit/empirical provenance required
VALIDATES_CONTRACT(test -> contract)
CONTRACT_DEPENDENCY(sut -> contract)
USES_CAPABILITY(test-suite -> capability)
PRODUCED_EVIDENCE(run -> receipt)
INVALIDATES(change -> receipt)
```

Every inferred relation records provenance source, adapter/rule version, topology revision and confidence/source kind.

### 3.5 `VerificationCapability`

A capability says **what** the project can verify, not how an agent should type the command.

```text
VerificationCapability
├─ capability_id
├─ kind: compile | type-check | lint | unit | integration | contract |
│        e2e | security | mutation | architecture | uat | custom
├─ supported_sut_kinds[]
├─ selector_granularity: repository | workspace | component | build-unit |
│                        file | symbol | test-id | tag/filter
├─ adapter_id
├─ toolchain_identity
├─ estimated_cost?
└─ constraints[]
```

The planner selects a semantic capability and scope. The adapter translates that into runner syntax.

### 3.6 `ImpactReason`

Typed reasons include:

- `DirectSourceTouch`
- `ComponentOwnership`
- `BuildUnitOwnership`
- `DependencyPropagation`
- `ReverseDependencyPropagation`
- `RuntimeDependencyPropagation`
- `PublicContractChange`
- `SchemaChange`
- `BuildOrWorkspaceChange`
- `ConfigurationChange`
- `GeneratedSurfaceChange`
- `ExplicitTestAssociation`
- `LocalUnitTest`
- `ComponentIntegrationTest`
- `CrossComponentContractTest`
- `RiskPolicyEscalation`
- later: `ObservedCoverage`, `HistoricalFailureCorrelation`

### 3.7 `TestSelectionPlan`

```text
TestSelectionPlan
├─ plan_id
├─ change_set_digest
├─ topology_revision
├─ sut_graph_revision
├─ policy_revision
├─ impacted_sut[]
├─ batches[]
│  ├─ stage
│  ├─ capability_id
│  ├─ semantic_scope[]
│  ├─ test_ids/selectors[]
│  ├─ reasons[]
│  ├─ expected_cost?
│  └─ stop/escalation_conditions[]
├─ reused_receipts[]
├─ unmapped_nodes[]
├─ confidence
└─ verdict: executable | blocked | verify_required
```

The plan is domain data. CLI/AgentHost render it; agents never recreate it from prose.

### 3.8 `TestEvidenceReceipt`

```text
receipt_id
change_set_digest
source_revision
topology_revision
sut_graph_revision
policy_revision
capability_id
semantic_scope
selected_test_ids/selectors
adapter/toolchain identity
started_at / completed_at
result
failures[]
coverage_refs?                 # optional enrichment
stdout/stderr artifact refs
```

A receipt is reusable only while all bound verification-relevant inputs remain fresh.

## 4. Ports and adapter SPI

Domain/application ports:

- `ActiveChangeSetPort`
- `ProjectTopologyPort`
- `SutGraphPort`
- `VerificationCapabilityRegistry`
- `TestCatalogPort`
- `TestImpactPlannerPort`
- `VerificationExecutorPort`
- `TestEvidenceRepository`
- `VerificationPolicyPort`

Adapter responsibilities are deliberately narrow:

1. detect manifests/workspaces/tooling;
2. expose component/build/test topology;
3. expose stable test/capability identities and supported selector granularity;
4. translate semantic batches to concrete tool invocations;
5. parse results into canonical receipts.

Adapters **do not choose** what should be tested.

## 5. Ecosystem examples — adapters, not kernel rules

The initial registry should be extensible. Candidate adapters include:

| Ecosystem | Topology sources | Example execution mechanisms |
|---|---|---|
| Rust | `Cargo.toml`, `cargo metadata` | cargo test, nextest, clippy/check |
| Java/Kotlin/JVM | Maven POMs, Gradle model | Maven/Gradle test tasks, JUnit/Kotest/etc. |
| JS/TS | package/workspace manifests | npm/pnpm/yarn + Jest/Vitest/Mocha/Playwright |
| Python | pyproject/setup/tox/nox metadata | pytest/unittest/tox/nox |
| Go | go.mod/go.work/package graph | go test/go vet/static tools |
| .NET | `.sln`/project/MSBuild graph | dotnet build/test |
| C/C++ | CMake/Meson/Bazel graph | CTest/framework-specific runners |
| Generic monorepo | Bazel/Buck2/Pants graph | native target/test selectors |

This list is illustrative, not exhaustive. Unknown ecosystems can use an explicit generic project profile until a native adapter exists.

## 6. Explicit project mapping

When discovery cannot express a cross-cutting relation, the project may declare it using a stable language-neutral mapping surface, conceptually:

```text
.sddk/test-map.yaml
```

Example:

```yaml
schema_version: 1
mappings:
  - sut: component:payments-api
    tests:
      - id: contract:checkout-consumer
        kind: contract
      - id: integration:payments-database
        kind: integration
    reason: explicit_contract_mapping

  - sut: schema:payments/openapi.yaml
    affects:
      - component:web-checkout
      - component:mobile-gateway
```

The exact schema is delivered by `TEST-IMPACT-002`. IDs must remain stable and versioned.

## 7. Deterministic topology and impact mapping

For every changed artifact:

1. resolve the narrowest known owning component/build unit;
2. classify source/config/schema/build/generated/runtime boundary semantics;
3. add direct dependency and contract relations;
4. propagate reverse/runtime dependencies only under deterministic rules;
5. resolve direct and boundary verification obligations;
6. surface unknown ownership/edges as unmapped nodes instead of hiding them.

For a polyglot boundary, the graph can cross adapters. Example:

```text
OpenAPI schema changed
 → backend contract boundary
 → TypeScript client generated surface
 → web component
 → contract tests + selected frontend integration tests
```

No language-specific special case is needed in the planner.

## 8. Progressive verification algorithm

### Stage 0 — cheap deterministic checks

Run only verification capabilities relevant to affected SUTs/surfaces: compile, syntax, type-check, lint or equivalent when project policy requires them.

### Stage 1 — direct behavior evidence

Run tests directly mapped to changed behavior/SUT nodes.

### Stage 2 — owning component/build-unit evidence

Widen when direct tests cannot prove component-level invariants.

### Stage 3 — dependency/contract closure

Add reverse-dependent, integration or contract evidence when a public API, schema, runtime boundary, build surface or cross-component contract changes.

### Stage 4 — risk/assurance extras

Add architecture, security, UAT, mutation or specialist evidence only when acceptance/risk policy requires it.

### Stop rule

Stop scoped execution as soon as every verification obligation for the active change is satisfied by fresh evidence.

### Block/escalation rules

Return `blocked` or `verify_required` when scoped proof is unsafe because of:

- unmapped changed artifact;
- ambiguous component/SUT ownership;
- unknown dependency or contract boundary;
- unsupported required capability/adapter;
- global build/test-infrastructure mutation whose closure cannot be bounded;
- corrupt/unavailable test catalog;
- policy requiring evidence unavailable to the scoped executor.

Do not silently replace these states with a complete repository test run inside `apply`.

## 9. `apply` contract

```text
ActiveChangeSet
      ↓
ProjectTestTopology
      ↓
SUT Impact Graph
      ↓
TestSelectionPlan
      ↓
execute next smallest required semantic batch
      ↓
TestEvidenceReceipt
      ↓
refresh change set after edits
      ↓
invalidate intersecting stale evidence only
      ↓
repeat until scoped obligations are satisfied
```

A coding agent MUST:

1. consume the assigned Work Item/task slice;
2. consume the project topology/capability snapshot rather than rediscover tooling repeatedly;
3. ask SDDK for the scoped verification plan;
4. execute only admitted batches;
5. record canonical receipts;
6. refresh impact after code/config changes;
7. stop/escalate on unmapped impact;
8. never run the complete project verification profile as normal `apply` behavior.

After `TEST-APPLY-001`, manually inventing test scope instead of consuming the service is a protocol error.

## 10. `verify` contract

`verify` runs the project's declared **full verification profile**, which may span multiple languages/tools in a polyglot repository.

It compares broad results against scoped evidence and emits selection-quality telemetry, especially escaped regressions and missing obligations.

Release may reuse a fresh successful verify receipt. Any relevant source/topology/policy/toolchain change makes the receipt stale.

## 11. Evidence cache and invalidation

Receipt reuse identity conceptually includes:

```text
(change/source digest,
 topology revision,
 SUT graph revision,
 verification policy revision,
 test/capability identity,
 test-input digest,
 adapter/toolchain identity)
```

Invalidation is graph-driven:

- changes invalidate receipts whose tested SUT/dependency/contract closure intersects changed nodes;
- unrelated fresh evidence remains reusable;
- project topology, test source, policy or toolchain changes invalidate affected receipts even if production source did not change.

## 12. Explainability

SDDK must answer semantically:

```text
why selected?
change -> artifact -> owner/SUT -> dependency/contract obligation -> verification capability/test

why widened?
public contract / reverse dependency / schema / runtime boundary / risk policy /
failed narrower batch / missing evidence

why reused?
identity inputs unchanged and no intersecting invalidation path

why blocked?
which topology/capability/mapping edge is missing and how to repair it
```

Runner command lines are implementation detail, not the primary explanation surface.

## 13. Agent-facing semantic surface

Illustrative semantic operations:

```text
verification.topology()
verification.impact()
verification.plan()
verification.next_batch()
verification.record(receipt)
verification.explain(test_or_plan)
verification.status()
```

Normal goals remain simple:

```text
sddk apply    # progressive scoped verification automatically
sddk verify   # full declared verification profile
```

## 14. Bootstrap policy before implementation ships

Until `TEST-APPLY-001` is `SHIPPED`, agents approximate the deterministic model using the repository's already-known capabilities:

- inspect Git changes only for the assigned slice;
- identify manifests/workspaces/build units once and cache the result;
- map changes to the narrowest SUT/component available;
- use known test commands/selectors rather than repeatedly probing help output;
- widen through dependencies/contracts only when justified;
- record what ran and why;
- reserve the complete project profile for explicit `verify`;
- report unknown impact rather than hiding it.

The bootstrap is language-neutral. Ecosystem-specific examples belong in adapters/docs, never in the normative algorithm.

## 15. Metrics

Track at minimum:

- scoped feedback latency;
- selected-test/check ratio;
- verification time saved vs full-verify baseline;
- evidence reuse ratio;
- unmapped SUT/test/capability ratio;
- adapter/topology coverage;
- mapping-confidence distribution;
- scoped-plan failures;
- **escape rate**: broad verify regression after scoped apply passed;
- token/tool-call count spent on test discovery and execution.

Future adaptive selectors may be promoted only with non-inferior escape rate/invariant coverage plus bounded efficiency improvement.

## 16. UAT acceptance scenarios

### UAT-1 — local single-language change

Given an internal component changes with mapped direct tests and no public-boundary change, when apply requests verification, then only direct/owning scoped checks run and no full repository suite is invoked.

### UAT-2 — public contract change

Given a public API/contract changes, when impact is computed, then reverse-dependent/contract tests are included with an explainable graph path.

### UAT-3 — unrelated component

Given component A changes and component Z has no dependency/contract/test relation to the affected closure, when apply runs, then Z's tests are not executed.

### UAT-4 — polyglot contract

Given a backend schema change affects a generated client in another language, when impact is computed, then the plan crosses adapter boundaries and selects required contract/generated-client/component evidence.

### UAT-5 — unmapped ecosystem or change

Given a changed artifact has unknown ownership or a required verification capability has no adapter/profile mapping, when the planner cannot justify adequate evidence, then it fails closed and explains the missing mapping instead of running everything.

### UAT-6 — evidence reuse

Given fresh evidence exists for an unaffected SUT closure, when another unrelated file changes, then that evidence remains reusable.

### UAT-7 — invalidation

Given a previously tested SUT/dependency/contract changes, when the active change set refreshes, then intersecting receipts become stale while unrelated receipts remain fresh.

### UAT-8 — verify boundary

Given scoped apply evidence is green, when `sddk verify` runs, then the declared full verification profile across all relevant ecosystems executes and any escaped regression is attributed to the selection strategy.

### UAT-9 — agent discipline

Given `TEST-APPLY-001` is shipped, when a coding agent performs apply work, then it consumes semantic topology/impact/plan data rather than inventing language-specific broad runner commands.
