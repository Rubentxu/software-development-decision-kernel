# SPEC-043 — Change-Scoped Verification Service

**Status:** Accepted design target
**Horizon:** H0 — Reconcile & Deterministic Foundations
**Decision:** ADR-043

## 1. Purpose

Provide a reliable SDDK service that answers, for the code that is changing **right now**:

1. What is the active change set?
2. Which SUT nodes can be affected?
3. Which tests provide relevant evidence for those nodes and boundaries?
4. What is the cheapest safe next test batch?
5. What evidence is already fresh and reusable?
6. Why was each test selected, widened, reused, invalidated or omitted?

The service exists to shorten the implementation feedback loop without moving integration risk out of sight. Full-system/workspace verification remains the responsibility of the `verify` goal.

## 2. Non-goals

The MVP does **not**:

- predict impact from an LLM alone;
- trust coverage history as the only selection signal;
- make the agent author nextest/cargo selectors by trial and error;
- replace `verify`;
- create a second graph database or authority model;
- silently run every test when impact cannot be resolved;
- optimize for minimum test count at the expense of escaped regressions.

## 3. Domain model

### 3.1 `ActiveChangeSet`

```text
ActiveChangeSet
├─ project_id
├─ work_item_id / run_id
├─ base_revision
├─ head_revision
├─ working_tree_revision/digest
├─ changed_paths[]
│  ├─ path
│  ├─ change_kind: added | modified | deleted | renamed
│  ├─ staged
│  ├─ hunks[]?             # adapter-provided
│  └─ symbols[]?           # when deterministically resolvable
└─ change_set_digest
```

The digest MUST change when any verification-relevant source/configuration input changes.

### 3.2 SUT node

Initial node kinds:

```text
Workspace
Package
Target
Module/File
Symbol?                # optional in MVP
ContractBoundary
ConfigOrBuildSurface
Test
TestSuite
EvidenceReceipt
```

The abstraction is language-neutral. Rust adapters map Cargo workspace/packages/targets into it.

### 3.3 Typed graph edges

Minimum edge types:

```text
TOUCHES(change -> artifact)
OWNS(package/target -> artifact)
DEPENDS_ON(sut -> sut)
REVERSE_DEPENDS_ON(sut -> sut)
TESTS(test -> sut)
COVERS(test -> sut)                 # explicit/empirical provenance required
CONTRACT_DEPENDENCY(sut -> boundary)
PRODUCED_EVIDENCE(test_run -> receipt)
INVALIDATES(change -> receipt)
```

Every non-explicit/inferred edge records:

- provenance source;
- graph revision;
- confidence class;
- inference rule/version.

The graph is a projection over canonical SDDK evidence/graph semantics, not a new source of truth.

### 3.4 `ImpactReason`

Typed reasons include:

- `DirectSourceTouch`
- `PackageOwnership`
- `DependencyPropagation`
- `ReverseDependencyPropagation`
- `PublicContractChange`
- `BuildOrWorkspaceChange`
- `SchemaOrConfigurationChange`
- `ExplicitTestAssociation`
- `ColocatedUnitTest`
- `PackageIntegrationTest`
- `RiskPolicyEscalation`
- later: `ObservedCoverage`, `HistoricalFailureCorrelation`

### 3.5 `TestSelectionPlan`

```text
TestSelectionPlan
├─ plan_id
├─ change_set_digest
├─ sut_graph_revision
├─ policy_revision
├─ impacted_sut[]
├─ batches[]
│  ├─ stage
│  ├─ selectors[]
│  ├─ test_ids[]
│  ├─ reasons[]
│  ├─ expected_cost?
│  └─ stop/escalation conditions
├─ reused_receipts[]
├─ unmapped_nodes[]
├─ confidence
└─ verdict: executable | blocked | verify_required
```

The plan is data. CLI/AgentHost render it; agents do not recreate it from prose.

### 3.6 `TestEvidenceReceipt`

A receipt binds:

```text
receipt_id
change_set_digest
source_revision
sut_graph_revision
policy_revision
selected_test_ids/selectors
executor/tool_version
started_at / completed_at
result
failures[]
coverage_refs?                 # optional enrichment
stdout/stderr artifact refs
```

A receipt is reusable only while every bound verification-relevant input remains fresh.

## 4. Ports and adapters

Domain/application ports:

- `ActiveChangeSetPort`
- `SutGraphPort`
- `TestCatalogPort`
- `TestImpactPlannerPort`
- `TestExecutorPort`
- `TestEvidenceRepository`
- `VerificationPolicyPort`

Rust bootstrap adapters:

- Git adapter: base/head, staged/unstaged, path/hunk changes;
- Cargo metadata adapter: workspace/package/target/dependency graph;
- Rust test catalog adapter: colocated unit tests + package integration tests;
- explicit mapping adapter: project-owned ambiguous/cross-package test relations;
- cargo/nextest execution adapter.

The planner is application/domain logic. Cargo/nextest are mechanisms, not selection authorities.

## 5. Deterministic MVP mapping

### 5.1 Ownership

For every changed path, resolve the narrowest known owning SUT:

1. package/target from workspace metadata;
2. module/file within package;
3. symbol only when deterministic resolution exists;
4. special classification for workspace/build/config/schema surfaces.

Unknown ownership is not ignored; it becomes an unmapped node.

### 5.2 Test associations

Resolve tests from:

1. colocated unit tests;
2. package integration tests;
3. explicit repository mapping for cross-package/system/contract tests;
4. dependency/reverse-dependency expansion when boundary rules require it.

Recommended explicit project mapping surface:

```text
.sddk/test-map.yaml
```

Example conceptual entry:

```yaml
mappings:
  - sut: crate:sddk-domain/workflow_ir
    tests:
      - test: crate:sddk-domain::workflow_ir_roundtrip
        kind: unit
      - test: tests:workflow_runtime_contract
        kind: contract
    reason: explicit_contract_mapping
```

The exact schema is delivered by `TEST-IMPACT-002`; paths/IDs must be stable and versioned.

## 6. Progressive verification algorithm

### Stage 0 — cheap deterministic checks

Run only checks relevant to affected targets/surfaces, for example compile/check/lint/type validation where the project profile requires them.

### Stage 1 — direct SUT tests

Run tests directly mapped to touched modules/symbols/contracts.

### Stage 2 — owning package/target tests

Widen to affected package/target evidence when direct tests are insufficient or a package-level invariant is involved.

### Stage 3 — dependency-boundary tests

Add reverse-dependent/integration/contract tests when:

- a public API changed;
- a contract boundary changed;
- dependency closure is affected;
- schema/build/workspace semantics cross package boundaries.

### Stage 4 — policy/risk extras

Add architecture, security, UAT, mutation or specialist evidence only when the active change contract/policy requires them.

### Stop rule

Stop scoped execution when all verification obligations for the active change are satisfied by fresh evidence.

### Escalation/block rules

Block or mark `verify_required` when any of the following makes scoped proof unsafe:

- unmapped changed artifact;
- ambiguous SUT ownership that affects required evidence;
- unknown dependency boundary;
- global workspace/build/test infrastructure mutation;
- test-catalog corruption or unavailable required runner;
- policy requires evidence not available in the current scoped executor.

Do **not** silently replace these outcomes with a full workspace test inside `apply`.

## 7. `apply` contract

Normal `apply` loop:

```text
ActiveChangeSet
      ↓
SUT Impact Graph
      ↓
TestSelectionPlan
      ↓
execute next smallest required batch
      ↓
TestEvidenceReceipt
      ↓
refresh change set after code edits
      ↓
invalidate only intersecting stale evidence
      ↓
repeat until scoped obligations satisfied
```

A coding agent MUST:

1. consume the assigned Work Item/task slice;
2. ask SDDK for the current scoped verification plan;
3. execute only the next admitted batch;
4. record the receipt;
5. refresh after code changes;
6. stop/escalate on blocked mapping;
7. never run a full project/workspace suite as normal `apply` behavior.

After `TEST-APPLY-001`, an agent manually inventing test scope instead of consuming the service is a protocol error.

## 8. `verify` contract

`verify` is the normal authoritative full-integration boundary.

It MUST run the project's declared full verification profile, including the full test suite and global checks/assurance required by policy.

`verify` compares its result against scoped evidence and emits selection-quality telemetry, especially escapes.

Release may reuse a fresh successful verify receipt. If relevant inputs changed, the receipt is stale and verify runs again.

## 9. Evidence cache and invalidation

Receipt reuse key conceptually includes:

```text
(change_set/source digest,
 SUT graph revision,
 verification policy revision,
 test identity + test-input digest,
 executor/toolchain identity)
```

Invalidation is graph-driven:

- a new change invalidates receipts whose tested SUT/dependency closure intersects the changed nodes;
- unrelated fresh evidence remains reusable;
- policy/toolchain/test-source changes invalidate affected receipts even when production source does not change.

This is an optimization only after correctness of identity/invalidation is proven.

## 10. Explainability

For any selected test, SDDK must answer:

```text
why selected?
change -> touched artifact -> owning SUT -> dependency/contract edge -> test
```

For any widened batch:

```text
why widened?
public contract / reverse dependency / risk policy / failed narrower batch / missing evidence
```

For any reused receipt:

```text
why still fresh?
identity inputs unchanged + no intersecting invalidation path
```

For a blocked plan:

```text
what mapping is missing and how can it be repaired?
```

Human/agent surfaces should prefer these semantic explanations over dumping runner syntax.

## 11. Agent-facing semantic surface

Target semantic operations (names illustrative; exact CLI/API naming belongs to implementation design):

```text
verification.impact()
verification.plan()
verification.next_batch()
verification.record(receipt)
verification.explain(test_or_plan)
verification.status()
```

Normal high-level goals remain simple:

```text
sddk apply    # scoped progressive verification automatically
sddk verify   # authoritative full verification profile
```

The agent should not need to probe Cargo/nextest flags to discover the right command.

## 12. Metrics and promotion gates

Track at minimum:

- scoped feedback latency;
- selected-test ratio;
- test execution time saved vs full verify baseline;
- evidence reuse ratio;
- unmapped SUT/test ratio;
- mapping-confidence distribution;
- scoped-plan failures;
- **escape rate**: verify regression after scoped apply passed;
- token/tool-call count spent on testing decisions/execution.

Future adaptive/learned selectors may be evaluated in Workflow Lab, but promotion requires non-inferior escape rate/invariant coverage plus bounded cost improvement.

## 13. Bootstrap policy before implementation ships

The design is admitted in H0 before the service exists. Therefore documentation must not pretend semantic service calls are already available.

Until `TEST-APPLY-001` is SHIPPED:

- agents approximate the deterministic policy from Git change scope + package ownership + direct/reverse dependencies + explicit tests;
- they run the narrowest justified package/test targets;
- they record what was run and why in apply progress/evidence;
- they do not run the full workspace/project suite except when explicitly executing `verify`;
- unknown impact is reported rather than hidden.

Once `TEST-APPLY-001` is SHIPPED, this bootstrap manual selection path is removed from normal agent behavior.

## 14. UAT acceptance scenarios

### UAT-1 — local implementation change

**Given** one internal module changes with mapped unit tests and no public-boundary change,
**when** `apply` requests verification,
**then** only direct/owning scoped checks run and no full workspace suite is invoked.

### UAT-2 — public contract change

**Given** a public API/contract changes,
**when** impact is computed,
**then** reverse-dependent/contract tests are included with an explainable path.

### UAT-3 — unrelated package

**Given** package A changes and package Z has no dependency/contract/test relation to the affected closure,
**when** `apply` runs,
**then** Z's tests are not executed.

### UAT-4 — unmapped change

**Given** a changed artifact has unknown ownership or required test mapping,
**when** the planner cannot justify adequate scoped evidence,
**then** it fails closed with a typed mapping gap and does not silently run every test.

### UAT-5 — evidence reuse

**Given** a fresh receipt exists for an unaffected SUT closure,
**when** another unrelated file changes,
**then** the receipt remains reusable and is not re-executed.

### UAT-6 — invalidation

**Given** a previously tested SUT or dependency changes,
**when** the active change set is refreshed,
**then** intersecting receipts become stale while unrelated receipts remain fresh.

### UAT-7 — verify boundary

**Given** scoped apply evidence is green,
**when** `sddk verify` runs,
**then** the declared full verification profile executes and any escaped regression is recorded against the selection strategy.

### UAT-8 — agent discipline

**Given** `TEST-APPLY-001` is shipped,
**when** a coding agent performs apply work,
**then** it consumes the semantic SDDK verification plan rather than discovering/constructing broad runner commands itself.
