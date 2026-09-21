# SPEC-032 — Execution Observability & Usage

**Status:** Proposed

## Per Attempt metrics

```yaml
workflow_run: wf-...
node_run: nr-...
attempt: at-...
logical_agent: sddk-architect
host: opencode
provider: openai
model: gpt-x
timing:
  started_at: ...
  first_token_at: ...
  finished_at: ...
  wall_ms: 1000
  tool_ms: 300
  waiting_ms: 50
usage:
  input_tokens: 0
  output_tokens: 0
  reasoning_tokens: null
  cache_read_tokens: null
  cache_write_tokens: null
  cost: null
  currency: null
result:
  class: success
retries: 0
failovers: 0
```

Unknown fields remain null/unknown; do not synthesize provider metrics unavailable from the host.

## Aggregates
By:
- day/week/month;
- workspace/repository;
- workflow type;
- capability;
- logical agent;
- model/provider/host;
- result/failure class.

## Outcome metrics
- first-pass verification rate;
- total success after retry;
- infrastructure failure rate;
- human intervention rate;
- median/p95 latency;
- token/cost efficiency;
- context reuse;
- failover success.

## Session reconstruction
A “work session” can be a projection grouped by explicit session ID plus time/repository heuristics, but inferred grouping must be labeled as inferred.
