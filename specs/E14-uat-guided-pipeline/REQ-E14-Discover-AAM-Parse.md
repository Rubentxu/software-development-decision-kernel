---
type: requirement
title: "REQ-E14-Discover-AAM-Parse"
slug: "REQ-E14-Discover-AAM-Parse"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "sddk uat discover + integration test test_uat_discover_*.rs"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-Discover-AAM-Parse

## Requirement

The `sddk uat discover` subcommand MUST parse artefacts produced by
`assets/uat-driver/computer_use.mjs` (screenshots, HTTP log, DOM
snapshots, URL transitions, action trajectory) into a non-empty
`ActualApplicationModel` (AAM) YAML containing:

- `pages[]` (each with `path, title, elements[]`)
- `flows[]` (each with `pages[], steps[]` — `action, selector,
  value, target, expected, screenshot` per step)
- `http[]` (HTTP request/response pairs)
- `screenshots[]` (file references with sha256)
- `urls[]` (URL transitions during exploration)

The AAM MUST be non-empty when Fara returned any successful run
(`status.success() == true`); it MUST NOT silently emit `pages: []`
when input was produced. When Fara is **unreachable** (health
`/health` ≠ 200), the subcommand MUST print a structured
`WARN: Fara not reachable at <url>` line and exit 0 with an empty
`screenshots/http/urls` AAM that carries
`app.fara_version: "unreachable"` and `provenance.fallback:
"no-fara"`; this empty-but-marked AAM is the **explicit fallback**,
distinct from the silent-empty bug.

The `--budget` (max Fara steps per goal) MUST be honoured by passing
it through to `computer_use.mjs --max-steps`. The AAM MUST include
`provenance: { generated_by: "uat-discovery", fara_session, model,
confidence, human_reviewed: false }`.

The CLI MUST refuse to silently swallow errors from
`computer_use.mjs`: a non-zero exit from the script propagates as a
non-zero exit from `discover` with the script's stderr echoed.

## Scenarios

### Scenario: Successful run produces non-empty AAM
- **GIVEN** `computer_use.mjs` exits 0 with 3 screenshots, 7 HTTP
  entries, 5 URL transitions and 4 actions written to `output_dir`
- **WHEN** `sddk uat discover --app-url http://app --goal X` runs
- **THEN** `discovered-flows.yaml` has `pages.len() >= 1`,
  `flows.len() >= 1`, `screenshots.len() == 3`, `http.len() == 7`

### Scenario: Fara unreachable → explicit fallback
- **GIVEN** Fara is not running on `--fara-url`
- **WHEN** `sddk uat discover --app-url http://app --goal X
  --fara-url http://nope:0` runs
- **THEN** stdout contains `WARN: Fara not reachable at
  http://nope:0`, the AAM file is written with
  `app.fara_version: "unreachable"`,
  `provenance.fallback: "no-fara"`, and exit code is 0

### Scenario: Empty AAM after successful run is a regression
- **GIVEN** a successful Fara run produced 1+ screenshots
- **WHEN** the AAM is written
- **THEN** `pages.len() > 0 AND flows.len() > 0` (asserted by a
  unit test); failure ⇒ exit 1 with
  `error: empty AAM after successful run`

### Scenario: Budget honoured
- **GIVEN** `--budget 10`
- **WHEN** `discover` invokes `computer_use.mjs`
- **THEN** the call passes `--max-steps 10`

### Scenario: computer_use.mjs non-zero exit propagates
- **GIVEN** the script exits 1
- **WHEN** `discover` runs
- **THEN** `discover` exits non-zero with the script's stderr
  surfaced in the failure envelope

## Traceability

- **Decision authority:** TBD by sddk-design
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Supersedes:** `specs/E14-uat-guided-pipeline/E14.4-TEST-DISCOVERY-AGENT.md`
  (kept for history)
- **Last modified:** 2026-08-12
- **Tested by:** `sddk uat discover` + integration test in
  `tests/test_uat_discover_*.rs` (TDD, ≥1 test required)
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:32 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞
- 2026-08-12T07:32 | supersedes | E14.4-TEST-DISCOVERY-AGENT.md | valid_from=2026-08-12 | valid_to=∞