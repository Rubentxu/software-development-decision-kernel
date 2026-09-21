# ADR-027-PROVIDER-FAILOVER — Treat provider/model failure as recoverable execution routing

**Status:** Accepted


## Decision
A provider/model failure creates a failed `Attempt` while preserving the logical `NodeRun`. A failure classifier, provider-health registry and circuit breaker determine whether to retry, back off, reroute or escalate.

## Examples
- transient 503 -> retry/backoff;
- weekly quota exhausted -> open circuit and reroute;
- auth invalid -> disable route and notify;
- context overflow -> recompile/compact context;
- task/test failure -> do not automatically treat as provider failure.

## Consequences
Workflows survive exhausted quotas and outages. Routing behavior becomes observable and testable.
