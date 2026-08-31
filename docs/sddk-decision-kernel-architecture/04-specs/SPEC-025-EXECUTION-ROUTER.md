# SPEC-025 — Execution Router

**Status:** Proposed

## Purpose
Select a concrete execution backend for a logical task.

## Route

```yaml
route_id: route-...
capability: architecture.review
logical_agent: sddk-architect
host: opencode-local
provider: openai
model: gpt-x
credential_route: personal-openai
constraints:
  privacy: local-or-approved-cloud
```

## Candidate filtering
Hard constraints run before scoring:
- health not disabled/open;
- capability supported;
- tools supported by host;
- context window sufficient or compilable;
- policy allows provider/data class;
- remaining hard budget;
- credentials available.

## Scoring
Example configurable score:

```text
score =
  capability_fit * 0.30 +
  quality          * 0.20 +
  availability     * 0.20 +
  historical_rate * 0.10 +
  latency_score    * 0.08 +
  cost_score       * 0.07 +
  cache_affinity   * 0.05
```

Weights are policy, not kernel constants.

## Explainability
Each selection produces:

```yaml
selected: route-gpt
considered:
  - route-claude:
      rejected: provider_circuit_open
  - route-gpt:
      score: 0.91
  - route-qwen-local:
      score: 0.73
```

and appends `model.route.selected`.

## Failover
On infrastructure failure, request a new route excluding the failed route/provider according to failure class/circuit state. Keep `NodeRun`; create new Attempt.

## Budget reservation
Before launch, reserve expected budget where estimable. Final actual usage reconciles the reservation.

## Manual override
CLI/UI may pin host/provider/model for a workflow/node if policy allows. Override is recorded as a decision event.
