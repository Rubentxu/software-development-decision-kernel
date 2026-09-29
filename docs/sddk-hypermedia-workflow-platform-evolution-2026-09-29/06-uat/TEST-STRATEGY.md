# Test strategy

## Philosophy

Development uses surgical tests around the changed semantic unit. Full suite is reserved for integration/release boundaries, consistent with project practice.

## Pyramid by contract

### T0 — Pure/domain

- URI parsing/identity;
- StepDefinition validation;
- capability merge lattice;
- provider selection ordering;
- affordance derivation inputs;
- staleness rules;
- WorkflowDefinition type checking;
- canonical digest properties.

Use property testing where useful: canonicalization, idempotency, ordering, bounded graphs.

### T1 — Component

- registries;
- compiler;
- augmentor;
- ContextCompiler adapters;
- ContributionReconciler against fake ports;
- Observation normalization.

### T2 — Storage integration

- persistent CapsuleStore;
- SessionBindingStore;
- StepRun/Contribution projections;
- migrations;
- crash/reopen;
- idempotent retries;
- cross-table referential integrity.

### T3 — Runtime integration

- WorkflowRuntime operators;
- restart mid-run;
- expansion/replan;
- hypermedia stale actions;
- action invocation through authority/gateway.

### T4 — Real provider

- CogniCode real negotiated capability → Observation → Context/consumer;
- Chronos real capture → Observation → Evidence/consumer;
- unavailable/incompatible/timeout cases.

### T5 — Agent/harness UAT

A provider/harness instance receives only resource representation + action schemas, not a copied lifecycle recipe, and completes selected vertical.

### T6 — Second-domain UAT

Book workflow proves portability.

## Mutation tests prioritarios

- remove active-cycle inference => CTX tests fail;
- allow stale ContextDelta => tests fail;
- drop authority recheck => HYP stale-action tests fail;
- treat preferred as required/vice versa => AUG tests fail;
- map provider timeout to Complete => OBS tests fail;
- bypass output schema => STEP tests fail;
- replay contribution duplicates row => CON tests fail.

## Fixtures

Maintain hermetic fixtures for:

- single/multiple cycles;
- previous storage schema;
- capability snapshots;
- provider observation payloads;
- legacy workflow manifests;
- equivalent WorkflowDefinitions;
- Book workflow outline with 3 chapters.

## Performance baselines

No hard SLO sin baseline. Medir inicialmente:

- context bootstrap p50/p95 for small/large project state;
- hypermedia resource projection p95;
- augmentation p95 without provider invocation;
- compile time for 10/100/1000 node definitions;
- ContextCapsule size/token estimate;
- number of provider calls avoided by observation reuse.

Sólo convertir en gate después de tres runs/release contexts comparables y decisión documentada.
