# SDDK Change-Scoped Testing Contract

> Applies to coding agents and the `apply` phase for **any language, build system or test framework**. It controls verification scope during implementation and overrides generic advice to “run everything” inside coding loops.

## Core rule

**`apply` proves the active change progressively. `verify` proves the whole declared project verification profile.**

Never assume Cargo, Maven, Gradle, npm, pytest, Go, .NET or any other ecosystem. First consume or discover the repository's project/test topology and verification capabilities, then operate on semantic SUT/test identities.

## Target behavior after `TEST-APPLY-001`

The agent MUST consume SDDK's semantic Change-Scoped Verification Service:

```text
ActiveChangeSet
  → ProjectTestTopology
  → SUT impact graph
  → next required semantic verification batch
  → adapter executes concrete runner command
  → TestEvidenceReceipt
  → refresh change
  → invalidate affected evidence
  → repeat until scoped obligations are green
```

The agent MUST NOT reconstruct broad runner selectors or repeatedly probe tool help when SDDK already provides topology/capability/plan data.

## Bootstrap behavior until `TEST-APPLY-001` ships

The semantic service does not exist yet. Approximate the same policy deterministically and conservatively.

For each task slice:

1. Inspect the active Git diff for the assigned scope only.
2. Reuse cached/detected project capabilities; do not rediscover the build/test stack on every loop.
3. Map changed artifacts to the narrowest known component/build-unit/module/SUT.
4. Identify direct tests/checks and contract obligations.
5. Expand through dependency, reverse-dependency, runtime or contract edges only when justified.
6. Execute the smallest justified batch.
7. Record the SUT, capability/tests, reason and result.
8. Refresh impact after edits.
9. If ownership, dependency, contract or verification capability is unknown, stop and report the gap; do not hide uncertainty by running the whole repository.

## Generic topology model

Think semantically:

```text
Repository
 ├─ Workspace(s)
 ├─ Component(s) / BuildUnit(s)
 │   ├─ Source / Config / Schema / Generated artifacts
 │   ├─ dependencies
 │   └─ VerificationCapabilities
 ├─ ContractBoundary(s)
 └─ TestUnit(s) / TestSuite(s)
```

A repository may be polyglot. One change may cross several adapter families through a contract/schema/generated-code edge.

## Progressive stages

### Stage 0 — cheap deterministic checks

Run only relevant compile/syntax/type-check/lint/build checks required by project policy for the affected SUT.

### Stage 1 — direct behavior tests

Run tests directly mapped to the changed behavior/SUT.

### Stage 2 — owning component/build unit

Widen when direct evidence cannot prove component-level invariants.

### Stage 3 — dependency/contract closure

Widen when public APIs, schemas, generated surfaces, runtime dependencies, build surfaces or cross-component contracts are affected.

### Stage 4 — risk/assurance extras

Run architecture/security/UAT/mutation/specialist checks only when acceptance/risk policy requires them.

Stop when all scoped verification obligations are satisfied by fresh evidence.

## Runner independence

The following are examples only, never normative planning rules:

- Rust: Cargo/nextest;
- JVM: Maven/Gradle;
- JS/TS: npm/pnpm/yarn with Jest/Vitest/etc.;
- Python: pytest/tox/nox;
- Go: `go test`;
- .NET: `dotnet test`;
- C/C++: CMake/Meson/Bazel/CTest;
- monorepos: Bazel/Buck2/Pants.

The same semantic plan must survive replacement of one runner by another adapter.

## Full-profile prohibition in normal `apply`

Do not normally execute an unbounded whole-repository test command simply because it is available. Equivalent examples include whole Cargo workspaces, every Maven/Gradle module, all npm workspace packages, unscoped pytest, all Go packages, every .NET project, or a top-level Bazel test-all target.

Broad execution is allowed when:

- the active goal is explicitly `verify`; or
- an operator explicitly requests a diagnostic broad run.

A diagnostic override is recorded as an override and does not replace scoped apply evidence.

## Failure handling

When scoped verification fails:

1. diagnose the failure;
2. fix only within the assigned task scope;
3. rerun the failed/directly affected batch;
4. widen only if the failure or new change creates a justified impact edge.

Do not respond to one scoped failure by reflexively running every project test.

## Bootstrap capability discovery

When cached project capabilities are missing, discover them once per relevant topology revision from stable repository evidence such as manifests, workspace descriptors, build files and project configuration. Persist/reuse the result when the surrounding SDDK phase contract allows it.

Discovery should answer semantic questions:

```text
what components/build units exist?
what depends on what?
what verification capabilities exist?
what selector granularity does each capability support?
what test identities are known?
```

Do not spend repeated LLM turns learning CLI syntax that an adapter/profile can encode.

## Evidence format

```yaml
change_scoped_verification:
  change_set: <revision/digest or bootstrap diff>
  topology_revision: <id if available>
  impacted_sut:
    - <component/build-unit/module/contract>
  batches:
    - stage: direct|component|dependency|risk
      capability: <unit|integration|contract|type-check|...>
      tests_or_scope: [<semantic ids/selectors>]
      reason: <impact path / acceptance obligation>
      result: pass|fail
  reused_evidence: []
  unmapped_impact: []
  full_profile_run: false
```

When blocked:

```yaml
change_scoped_verification:
  status: blocked
  unmapped_impact:
    - <artifact/SUT/dependency/contract/capability>
  reason: <why safe scoped selection cannot be justified>
  recommendation: <mapping/adapter/profile/spec/verify action required>
```

## Strict TDD

Strict TDD still controls RED → GREEN → TRIANGULATE → REFACTOR. This contract controls **which semantic tests/checks are executed at each step**. TDD does not imply rerunning the whole repository after every edit.

## Reliability rule

Never optimise only for fewer tests. The key guard is escaped regression/obligation rate: if broad `verify` later finds something that scoped `apply` missed, that is negative evidence against the topology/mapping/selection strategy and must feed improvement.
