# SPIKE-003 — Zero-Server Static Cockpit

## Question
Can a useful operational UI be rendered directly from persisted SDDK data and opened through `file://`?

## Prototype views
1. Overview.
2. Event Journal.
3. Workflow timeline.
4. Provider/model health.
5. One causal trace.

## Constraints
- no HTTP server;
- no CDN;
- no external fetch;
- data embedded in generated artifact;
- works in Chromium/Firefox from local file;
- atomic regeneration.

## Dataset
Generate a deterministic fixture with:
- 2 workflows;
- 5 NodeRuns;
- 1 provider quota failure;
- 1 failover;
- usage metrics;
- 1 human approval.

## Success criteria
A user can answer in under one minute:
- what ran;
- what failed;
- why it failed;
- which model replaced it;
- how long the workflow took;
- what requires attention.
