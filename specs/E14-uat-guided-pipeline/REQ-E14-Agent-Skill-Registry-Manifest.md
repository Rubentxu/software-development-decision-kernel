---
type: requirement
title: "REQ-E14-Agent-Skill-Registry-Manifest"
slug: "REQ-E14-Agent-Skill-Registry-Manifest"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "sddk inventory + integration test test_e14_registry_*.rs"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-Agent-Skill-Registry-Manifest

## Requirement

E14 MUST register the agents `uat-form-quality`, `uat-ux-form`,
`uat-discovery` in `MANIFEST.sha256` (with content sha256 of each
`agents/uat-*.md`) and in `permissions.yaml` (default-deny →
`phases: [uat], capabilities: [...]`).

Their corresponding skills MUST also be registered:

- `skills/uat-form-quality/SKILL.md`
- `skills/uat-ux-form/SKILL.md`
- `skills/uat-discovery/SKILL.md`

Each appears in `MANIFEST.sha256` with its current sha256. The
release script (`scripts/dev-bundle.sh` or `sddk-release` equivalent)
MUST treat these entries as required to publish a release. `sddk
inventory` MUST list these agents/skills without `missing` flags.

`permissions.yaml` MUST grant at least one capability (e.g.
`run-uat-quality`, `transform-ux-form`, `discover-flows`) to each of
the three agents; an empty capability list MUST be flagged as a
release-blocker.

## Scenarios

### Scenario: All E14 agents present in MANIFEST
- **GIVEN** `MANIFEST.sha256`
- **WHEN** `grep -E "agents/uat-form-quality.md|agents/uat-ux-form.md|agents/uat-discovery.md"` runs
- **THEN** 3 entries match, each with the live sha256 of the file

### Scenario: Permissions register E14 agents with capabilities
- **GIVEN** `permissions.yaml`
- **WHEN** the file is parsed
- **THEN** `uat-form-quality`, `uat-ux-form`, `uat-discovery` are
  present with `phases: ["uat"]` and `capabilities` non-empty

### Scenario: Skills present in MANIFEST
- **GIVEN** `MANIFEST.sha256`
- **WHEN** the file is scanned for `skills/uat-form-quality/SKILL.md`,
  `skills/uat-ux-form/SKILL.md`, `skills/uat-discovery/SKILL.md`
- **THEN** all 3 entries are present with their sha256

### Scenario: Inventory reports no missing entries
- **GIVEN** `sddk inventory`
- **WHEN** the agent runs against this repo
- **THEN** no `missing` flag appears for any E14 agent or skill

### Scenario: Empty capability list is rejected
- **GIVEN** `permissions.yaml` with
  `uat-form-quality: {phases: [], capabilities: []}`
- **WHEN** the release gate runs
- **THEN** it MUST abort with
  `uat-form-quality: empty capabilities`

## Traceability

- **Decision authority:** TBD by sddk-design
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Last modified:** 2026-08-12
- **Tested by:** `sddk inventory` + integration test in
  `tests/test_e14_registry_*.rs` (TDD)
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:35 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞