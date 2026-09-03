# SDDK Change-Scoped Testing Contract

> Applies to coding agents and the `apply` phase. This contract is subordinate to accepted architecture/specs but overrides generic advice to "run all tests" during implementation.

## Core rule

**`apply` proves the active change progressively. `verify` proves the whole project.**

During normal implementation, never run the full project/workspace test suite merely to be safe. Full-suite execution belongs to `sddk verify` (and release/GA gates that consume a fresh verify result).

## Why

Broad test execution inside every coding loop wastes developer time, runner time and LLM/tool budget. It also hides the real question: *which system-under-test and contracts can this exact change affect?*

SDDK therefore treats testing as an impact/evidence problem, not a shell-command habit.

## Target behavior after `TEST-APPLY-001`

The agent MUST consume SDDK's Change-Scoped Verification Service:

```text
active change
  → SUT impact graph
  → next required test batch
  → execute
  → evidence receipt
  → refresh change
  → invalidate affected evidence
  → repeat until scoped obligations are green
```

The agent MUST NOT independently reconstruct broad cargo/nextest/pytest/etc. selectors when the service can provide the semantic plan.

Use semantic output such as impacted SUT, reasons, test IDs/selectors, evidence freshness and escalation reason. Runner syntax is an adapter concern.

## Bootstrap behavior until `TEST-APPLY-001` ships

The semantic service does not exist yet. Until it ships, approximate the same policy deterministically and conservatively.

For each task slice:

1. Read the active Git diff for the assigned scope only.
2. Map changed files to the narrowest known package/module/target/SUT.
3. Identify directly associated tests.
4. Expand through dependency/reverse-dependency or contract boundaries only when the change can cross them.
5. Run the smallest justified test/check batch.
6. Record what ran and why in apply evidence.
7. Recompute the scope after additional code changes.
8. If impact is ambiguous or required mapping is unknown, stop and report the gap. Do not conceal uncertainty by automatically running every test.

For Rust, prefer this evidence hierarchy:

```text
Git change set
 → owning Cargo package/target
 → direct module/unit/integration tests
 → package tests when needed
 → reverse-dependent/contract tests for public-boundary impact
```

Do not probe runner flags repeatedly. Reuse known project commands/selectors and keep the selection rationale in evidence.

## Progressive stages

### Stage 0 — relevant cheap checks

Compile/check/lint/type-check only the affected target/surface where practical and required by project policy.

### Stage 1 — direct tests

Run tests directly covering the changed behavior/SUT.

### Stage 2 — owning target/package

Widen only when direct evidence does not prove package-level invariants.

### Stage 3 — dependency/contract closure

Widen when a public API, schema, build surface or cross-package contract is affected.

### Stage 4 — risk/assurance extras

Run architecture/security/UAT/mutation/specialist checks only when the active acceptance/risk policy requires them.

Stop when the active change's scoped verification obligations are satisfied.

## Full-suite prohibition in `apply`

The following are NOT normal apply actions:

```text
cargo test --workspace
nextest run over the whole workspace
pytest with no justified scope over the whole project
npm/pnpm test over every package
any equivalent "run everything" command
```

They are allowed only when:

- the active goal is explicitly `verify`; or
- an operator explicitly requests a broad debugging run.

A debugging override MUST be reported as an override. It does not replace scoped apply evidence and does not change the lifecycle contract.

## Failure handling

When a scoped test fails:

1. diagnose the failure;
2. fix only within the assigned task scope;
3. rerun the failed/directly affected batch;
4. widen only if the failure or change creates a new dependency/contract impact reason.

Do not respond to one scoped failure by launching the full suite.

## Evidence format

Every completed apply slice should be able to report:

```yaml
change_scoped_verification:
  change_set: <sha/digest or described bootstrap diff>
  impacted_sut:
    - <package/module/contract>
  batches:
    - stage: direct|package|dependency|risk
      tests: [<ids/selectors>]
      reason: <impact path / acceptance obligation>
      result: pass|fail
  reused_evidence: []
  unmapped_impact: []
  full_suite_run: false
```

When blocked:

```yaml
change_scoped_verification:
  status: blocked
  unmapped_impact:
    - <artifact/SUT/relation>
  reason: <why safe scoped selection cannot be justified>
  recommendation: <mapping/spec/verify action required>
```

## Strict TDD

Strict TDD still uses RED/GREEN/TRIANGULATE/REFACTOR. This contract controls **test execution scope** inside those steps.

A TDD loop runs the relevant tests needed for the current behavior. The full suite remains a `verify` responsibility.

## Reliability rule

Never optimize selection only for fewer tests. The key guard is escaped regression rate: if `verify` later finds regressions that scoped apply missed, that is evidence against the selection/mapping strategy and must feed its improvement.
