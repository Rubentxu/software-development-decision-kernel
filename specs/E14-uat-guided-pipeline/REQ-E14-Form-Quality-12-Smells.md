---
type: requirement
title: "REQ-E14-Form-Quality-12-Smells"
slug: "REQ-E14-Form-Quality-12-Smells"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "sddk uat quality + integration test test_uat_quality_*.rs"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-Form-Quality-12-Smells

## Requirement

The `sddk uat quality` subcommand MUST detect the **12 test-smell
categories** of `arXiv:2308.01386` plus extensions defined in
`agents/uat-form-quality.md`: `AMBIGUOUS_INSTRUCTION`,
`EXPECTED_ABSENT`, `MACHINE_OBSERVABLE`, `DUPLICATED_CHECK`,
`NO_RECOVERY_PATH`, `LEADING_QUESTION`, `SUBJECTIVE_NO_SCALE`,
`FAIL_NO_EVIDENCE`, `STEP_TOO_LARGE`, `EXCESSIVE_STEPS`,
`HIDDEN_PREREQUISITE`, `NO_BRANCHING`, `BLIND_CHECK_WITHOUT_HIDDEN`.

Severity assignment MUST match the catalog: `BLOCKER` for
`EXPECTED_ABSENT` and `FAIL_NO_EVIDENCE`; `WARNING` for
`AMBIGUOUS_INSTRUCTION` and `STEP_TOO_LARGE`; `SUGGESTION` for
`NO_BRANCHING` and `DUPLICATED_CHECK`.

The `--threshold` flag MUST be honoured (`BLOCKER` = stop on
blockers; `WARNING` = stop on any smell at-or-above WARNING).
The subcommand MUST exit 0 only when no smell at-or-above
threshold exists. The output report MUST contain
`schema_version`, `analyzer: "uat-form-quality"`, `model`,
`analyzed_at`, `plan_ref`, `smells[]`, `summary.{total, blockers,
warnings, suggestions, pass}`, `verdict ∈ {PASS, NEEDS_REVISION}`,
`threshold_applied`.

The agent MUST NEVER mutate the plan file (verified by sha256 of
input plan before/after).

## Scenarios

### Scenario: All 12 categories detected
- **GIVEN** a plan fixture with one example of each of the 12 smell types
- **WHEN** `sddk uat quality --plan fixture.yaml` runs
- **THEN** the report `smells[]` contains 12 entries with the expected
  `smell_id` set; `summary.blockers >= 2`; `verdict = NEEDS_REVISION`;
  exit code 1

### Scenario: Threshold escalation from BLOCKER to WARNING
- **GIVEN** the same fixture
- **WHEN** `sddk uat quality --plan fixture.yaml --threshold WARNING` runs
- **THEN** exit code 1 with `threshold_applied: "WARNING"`

### Scenario: Clean plan passes
- **GIVEN** a plan with no smells
- **WHEN** `sddk uat quality --plan clean.yaml` runs
- **THEN** exit code 0, `verdict: "PASS"`, `summary.total = 0`

### Scenario: Plan file is not mutated
- **GIVEN** an input plan with sha256 `S0`
- **WHEN** `sddk uat quality` runs against it
- **THEN** sha256 of the plan file is byte-equal before and after

### Scenario: Report schema is complete
- **GIVEN** any plan
- **WHEN** the report is serialised
- **THEN** it contains all required keys (`schema_version`,
  `analyzer`, `model`, `analyzed_at`, `plan_ref`, `smells`,
  `summary`, `verdict`, `threshold_applied`)

## Traceability

- **Decision authority:** TBD by sddk-design (ADR candidate)
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Supersedes:** `specs/E14-uat-guided-pipeline/E14.2-FORM-QUALITY-AGENT.md` (kept for history)
- **Last modified:** 2026-08-12
- **Tested by:** `sddk uat quality` + integration test in
  `tests/test_uat_quality_*.rs` (TDD, ≥1 test required)
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:30 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞
- 2026-08-12T07:30 | supersedes | E14.2-FORM-QUALITY-AGENT.md (text typo / incomplete implementation; replaced by executable spec) | valid_from=2026-08-12 | valid_to=∞