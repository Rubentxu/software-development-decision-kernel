# SPEC-030 — UAT Bounded Context & Pack

**Status:** Proposed

## Preserve existing strengths
The current design already separates:
- execution result;
- machine assessment;
- human decision/acceptance;
- executor kinds;
- evidence bundle;
- multiple oracle types;
- review triggers;
- runner modes;
- plan approval;
- session/history.

Do not regress these semantics.

## Extend lifecycle

```text
UatCampaign
  ├── UatPlan
  ├── UatScenario
  │   └── UatScenarioRun
  │       ├── Evidence
  │       ├── OracleAssessment
  │       ├── MachineAssessment
  │       └── HumanDecision
  ├── UatDefect
  │   └── UatRetest
  ├── UatWaiver
  └── UatSignoff
      └── UatReleaseDecision
```

## Traceability

```text
Release
 -> Signoff
 -> Feature/Requirement
 -> Scenario
 -> ScenarioRun
 -> Evidence/Oracle/Reviewer
 -> exact source/build/artifact provenance
```

## Change impact

```text
Git diff
 → code/architecture graph impact
 → requirements/features affected
 → scenarios affected
 → evidence invalidated/stale
 → minimal safe retest set
```

## Execution adapters
CLI, API, Script, Playwright/browser, ComputerUse and Human remain adapter/executor strategies behind ports.

## Observability correlation
Each UAT run may carry correlation IDs linking:
- browser/DOM evidence;
- console/network;
- backend trace;
- logs/metrics;
- build/deployment version.

## Workflow integration
UAT is both:
- a bounded context with its own domain;
- a pack exposing workflows/capabilities/gates to the generic runtime.
