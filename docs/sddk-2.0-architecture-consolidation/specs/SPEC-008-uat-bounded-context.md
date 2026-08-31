# SPEC-008 — UAT Bounded Context and Guided Runner

**Status:** Proposed

## 1. Goal

Preserve the rich v1.9 UAT feature set while extracting it from oversized CLI/domain modules into a coherent bounded context and first-class pack.

## 2. Domain concepts

The UAT bounded context owns:

- plan;
- scenario;
- step;
- check;
- risk/blast radius;
- runner mode;
- blind check;
- checkpoint;
- diagnostic;
- execution;
- evidence link;
- review;
- acceptance/sign-off;
- staleness;
- history.

Universal evidence storage, actor identity and capability authorization remain platform services.

## 3. Suggested module split

```text
sddk-uat-domain
sddk-uat-app
sddk-uat-adapters
sddk-uat-web
```

or equivalent modules inside a pack if separate crates would create needless overhead.

## 4. Ports

Recommended ports:

- `UatPlanRepository`;
- `UatExecutionRepository`;
- `EvidenceRecorder`;
- `BrowserDriver` / `ComputerUseDriver`;
- `DiagnosticAgent`;
- `HumanReviewPort`;
- `AcceptanceSigner`;
- `Clock`;
- `ArtifactStore`.

## 5. Runner UX

The guided runner MUST preserve:

- designer / runner / reviewer modes;
- wizard-like progression;
- clear preconditions and expected results;
- checkboxes/check states;
- blind checks where appropriate;
- evidence gates;
- checkpoints;
- AI diagnostics as assistive, not authoritative;
- immutable acceptance/sign-off records;
- stale warnings when implementation changes.

## 6. Release acceptance

Acceptance is modeled as a signed/hashed domain record linked to exact plan/scenario/evidence versions. Re-signing after stale changes creates a new acceptance event; it does not mutate history.

## 7. Autogeneration by agents

Agents MAY generate candidate scenarios, steps and checks from requirements, code changes, risk analysis and historical failures. Generated plans MUST pass schema validation and SHOULD expose provenance showing which requirements/code/evidence led to each scenario.

## 8. UAT as first pack proof

The UAT extraction is the recommended first proof that the pack runtime can host a complex domain with CLI, UI, agents, evidence and release integration without leaking into core.
