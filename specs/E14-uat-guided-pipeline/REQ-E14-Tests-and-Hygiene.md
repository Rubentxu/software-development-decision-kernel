---
type: requirement
title: "REQ-E14-Tests-and-Hygiene"
slug: "REQ-E14-Tests-and-Hygiene"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "cargo test --workspace + cargo fmt --check + cargo clippy -- -D warnings"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-Tests-and-Hygiene

## Requirement

E14 MUST ship, in strict TDD order:

- ≥ 1 unit or integration test per subcommand in `tests/`:
  `test_uat_quality_*.rs`, `test_uat_enrich_forms_*.rs`,
  `test_uat_discover_*.rs`, `test_uat_generate_*.rs`.
- ≥ 1 JS contract test in `tests/` for the video / annotation
  handlers: `tests/test_evidence_capture_contract.js`.
- `cargo fmt --check` MUST exit 0 with empty stderr.
- `cargo clippy --workspace -- -D warnings` MUST exit 0.

Tests MUST be authored **before** the implementation in this cycle
(`strict_tdd=true`); the cycle's commit log MUST show the test
commit preceding the implementation commit for each subcommand
(or, in a single squash, the test code MUST precede the impl code
within the file diff).

## Scenarios

### Scenario: cargo fmt passes
- **GIVEN** the workspace after E14 changes
- **WHEN** `cargo fmt --check` runs
- **THEN** exit code is 0 and stderr is empty

### Scenario: clippy with -D warnings passes
- **GIVEN** the workspace after E14 changes
- **WHEN** `cargo clippy --workspace -- -D warnings` runs
- **THEN** exit code is 0

### Scenario: ≥1 test per subcommand exists and passes
- **GIVEN** tests `test_uat_quality_*.rs`,
  `test_uat_enrich_forms_*.rs`, `test_uat_discover_*.rs`,
  `test_uat_generate_*.rs`
- **WHEN** `cargo test --workspace` runs
- **THEN** each of the 4 subcommands has at least one passing test

### Scenario: JS contract test for video/annotation
- **GIVEN** `tests/test_evidence_capture_contract.js` exercises
  the kit's `evidenceCaptureUI` for `video` and `annotation`
- **WHEN** the contract runner executes
- **THEN** the file exits 0 and asserts SHA-256 emission + duration
  cap

### Scenario: TDD order respected in commit history
- **GIVEN** the cycle's git log (`git log --oneline HEAD~10..HEAD`)
- **WHEN** the cycle ends
- **THEN** each subcommand's test files appear before (or earlier
  in the same commit than) the corresponding implementation

## Traceability

- **Decision authority:** TBD by sddk-design
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Last modified:** 2026-08-12
- **Tested by:** workspace test harness + JS contract runner
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:36 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞