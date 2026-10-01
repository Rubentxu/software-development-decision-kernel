---
type: requirement
title: "REQ-E14-Generate-Pipeline-Approval"
slug: "REQ-E14-Generate-Pipeline-Approval"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "sddk uat generate + integration test test_uat_generate_*.rs"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-Generate-Pipeline-Approval

## Requirement

The `sddk uat generate` subcommand MUST consume
`--requirements <DIR>`, `--changelog <FILE>`, `--last-plan <FILE>`,
and `--app-url <URL>` (when `--discover`), and MUST fail loudly when
`--discover` is set without `--app-url` (exit non-zero with
`--app-url required when --discover is set`).

It MUST emit provenance on the produced draft plan:
`{ generated_by: "uat-planner (via generate)", based_on: {
requirements_refs, changelog_ref, last_plan_ref, aam_ref },
confidence, human_reviewed: false }`.

Each stage failure (discover, plan, enrich, quality, validate) MUST
be reported in stdout with the stage tag, the failure reason, and
MUST abort the pipeline with non-zero exit; subsequent stages MUST
NOT run.

A successful draft MUST have `features.len() > 0` after Stage 2 —
an empty plan MUST NOT be written. The pipeline MUST reject
`features.len() == 0` after Stage 2 with `error: empty plan after
Stage 2 (planner)`.

When `--interactive` is set, the pipeline MUST block on a human
approval gate before Stage 5; the gate MUST be a blocking prompt
that records `approved_by: {id, display}, approved_at` in the final
plan's provenance. On rejection the pipeline exits non-zero.

## Scenarios

### Scenario: Stage failure aborts pipeline
- **GIVEN** Stage 4 (quality) returns `status != 0`
- **WHEN** `sddk uat generate` runs
- **THEN** stdout contains `quality: FAIL`, the line `pipeline
  stopped at quality gate`, and exit code is non-zero; Stage 5
  MUST NOT run

### Scenario: Empty plan after Stage 2 is rejected
- **GIVEN** planner produces 0 features
- **WHEN** Stage 2 completes
- **THEN** `generate` exits non-zero with
  `error: empty plan after Stage 2 (planner)`; no
  `uat-plan-*.yaml` is written

### Scenario: Discover without app-url rejected upfront
- **GIVEN** `--discover` set, `--app-url` absent
- **WHEN** `generate` runs
- **THEN** exit non-zero with
  `--app-url required when --discover is set`

### Scenario: Interactive gate blocks before validate
- **GIVEN** `--interactive`
- **WHEN** Stages 1-3 complete
- **THEN** the process blocks on a human prompt before Stage 4;
  on approval, the final plan's provenance includes
  `approved_by`, `approved_at`; on rejection, exit non-zero

### Scenario: Provenance links upstream inputs
- **GIVEN** inputs `--requirements docs/ --changelog CHANGELOG.md
  --last-plan uat-plan-v1.yaml`
- **WHEN** the draft plan is written
- **THEN** `provenance.based_on` contains `requirements_refs,
  changelog_ref, last_plan_ref` populated

### Scenario: Non-interactive run skips approval gate
- **GIVEN** no `--interactive` flag
- **WHEN** `generate` runs through Stages 1-5
- **THEN** no blocking prompt is presented; provenance carries
  `human_reviewed: false`

## Traceability

- **Decision authority:** TBD by sddk-design
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Supersedes:** `specs/E14-uat-guided-pipeline/E14.5-PIPELINE-WIRING.md`
  (kept for history)
- **Last modified:** 2026-08-12
- **Tested by:** `sddk uat generate` + integration test in
  `tests/test_uat_generate_*.rs` (TDD, ≥1 test required)
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:33 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞
- 2026-08-12T07:33 | supersedes | E14.5-PIPELINE-WIRING.md | contradiction C-2 (FormQuality/UXForm order) resolved: canonical order is enrich→quality→schema→approval | valid_from=2026-08-12 | valid_to=∞