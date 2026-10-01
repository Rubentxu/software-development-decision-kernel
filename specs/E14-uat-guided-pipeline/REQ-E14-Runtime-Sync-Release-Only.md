---
type: requirement
title: "REQ-E14-Runtime-Sync-Release-Only"
slug: "REQ-E14-Runtime-Sync-Release-Only"
domain: "[[E14-uat-guided-pipeline]]"
status: active
created: 2026-08-12
created_in_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_cycle: "[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]"
last_modified_version: v1.9.1
decision_authority: "TBD by sddk-design"
tested_by: "git log audit + sha256 of docs/sddk-2.0-architecture-consolidation/**"
verified_in_cycle:
incidences: []
rfc2119: MUST
stale_after: 2026-11-10
---

# REQ-E14-Runtime-Sync-Release-Only

## Requirement

The dev bundle (`sddk dev install`) MUST NOT be triggered as part
of this cycle. The runtime bundle at
`~/.local/share/sddk/framework/<v>/` MAY be synchronised only by the
release agent (`sddk-release`) when the milestone closes.

Mid-cycle agent / skill edits land in `agents/` and `skills/` in the
CWD (`sddk-framework/`); the next `sddk dev install` (post-release)
will refresh the runtime bundle.

`docs/sddk-2.0-architecture-consolidation/**` is **out of scope**
and **MUST** remain byte-identical to its pre-cycle state. Any
modification to those files during the cycle is a release blocker.

## Scenarios

### Scenario: No `sddk dev install` invocation in this cycle
- **GIVEN** the cycle's git log
- **WHEN** `git log --oneline | grep -i "dev install\|bundle sync"` runs
- **THEN** no commit message references dev install or bundle sync

### Scenario: docs/sddk-2.0-* byte-identical
- **GIVEN** `docs/sddk-2.0-architecture-consolidation/**` at cycle start
- **WHEN** the cycle ends and
  `git diff <start-sha>..HEAD -- docs/sddk-2.0-architecture-consolidation/`
  runs
- **THEN** the diff is empty

### Scenario: Runtime bundle is untouched mid-cycle
- **GIVEN** the runtime bundle at
  `~/.local/share/sddk/framework/v1.9.0/` at cycle start
- **WHEN** the cycle ends
- **THEN** no file in the runtime bundle has a newer mtime than the
  cycle start timestamp

## Traceability

- **Decision authority:** TBD by sddk-design
- **Created in cycle:** [[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]]
- **Last modified:** 2026-08-12
- **Tested by:** git log audit + sha256 comparison
- **Incidences:** none

## Changelog (bi-temporal)

- 2026-08-12T07:37 | created | cycle=[[CYC-2026-08-12-e14-uat-guided-pipeline-stabilization-deferred]] | valid_from=2026-08-12 | valid_to=∞