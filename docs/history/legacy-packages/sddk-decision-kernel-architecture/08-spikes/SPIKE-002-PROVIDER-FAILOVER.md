# SPIKE-002 — Provider Quota Failover

## Question
Can one logical NodeRun survive provider quota exhaustion and continue on a compatible model/provider?

## Scenario

```text
NodeRun architecture.review
  Attempt #1 OpenCode + Provider A
  -> injected QuotaExhausted
  -> circuit opens
  -> route invalidated
  Attempt #2 OpenCode + Provider B
  -> recovery context injected
  -> success
```

## Implementation strategy
Use fake providers/errors first; then repeat with real host error shapes where practical.

## Assertions
- same `NodeRunId` across attempts;
- distinct Attempt IDs;
- no retry of opened route;
- recovery capsule references previous attempt;
- route-selection event explains rejected/selected candidates;
- final workflow result is success;
- Journal makes failure/recovery obvious.

## Failure conditions
- supervisor must manually parse raw log strings;
- work is duplicated from scratch without recovery state;
- provider error is indistinguishable from task failure;
- retry loops indefinitely.
