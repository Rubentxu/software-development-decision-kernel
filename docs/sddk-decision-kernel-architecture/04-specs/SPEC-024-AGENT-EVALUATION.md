# SPEC-024 — Agent, Routing & Workflow Evaluation

**Status:** Proposed

## Goal
Measure whether agents/routes/workflow strategies produce useful verified outcomes, not merely low tokens or latency.

## Evaluation levels
1. Capability execution.
2. Provider/model route.
3. Workflow strategy/graph.

## Golden tasks
Group by capability and change class: architecture, security, testing, implementation, UAT, incident, refactor, migration. Define expected properties/invariants, not one exact prose answer.

## Workflow experiments
Compare `A-full`, `sdd-adaptive` and future variants on the same goal/starting revision when feasible.

Metrics:
- verifier/acceptance success;
- regressions and architecture/security violations;
- invariant/evidence coverage;
- first-pass rate;
- retries/remediation/convergence rounds;
- human corrections/escalations;
- latency/tokens/cost;
- agent calls;
- handoff count and handoff entropy proxy;
- context compiled vs actually read;
- WorkUnits generated/abandoned;
- provider failovers.

## Handoff entropy proxy
Track how much material is passed to the next agent versus actually read/used, plus repeated re-discovery events. This is an operational proxy, not a claim to measure semantic information theory exactly.

## Learning policy

```text
candidate policy
 → offline/fork evaluation
 → shadow mode
 → bounded rollout
 → compare
 → promote/revert
```

No automatic policy promotion solely from cost/token metrics.

## Test-tooling boundary

Test-tooling ownership for evaluation assets follows ADR-0069: Python owns external golden/evaluation/analytical assets (scripts/, SPEC-024/SPEC-040 output validation). See [ADR-0069-test-tooling-ownership.md](../../adr/ADR-0069-test-tooling-ownership.md) and [TEST-TOOLING-EVIDENCE-AUDIT.md](../09-implementation/TEST-TOOLING-EVIDENCE-AUDIT.md).
