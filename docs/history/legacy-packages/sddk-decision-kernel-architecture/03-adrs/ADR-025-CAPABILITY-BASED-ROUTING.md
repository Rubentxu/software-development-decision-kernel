# ADR-025-CAPABILITY-BASED-ROUTING — Route semantic capabilities rather than hard-coded agents/models

**Status:** Accepted


## Decision
Workflow nodes request capabilities such as `architecture.review` or `testing.execute`. A Capability Registry resolves eligible logical agents/providers; the Execution Router selects a concrete route.

## Route dimensions
- capability fit;
- health/availability;
- model quality tier;
- cost/latency;
- privacy/local-only constraints;
- historical outcome metrics;
- context window;
- tool support;
- side-effect class.

## Consequences
Agent definitions become stable while models/providers can change independently.
