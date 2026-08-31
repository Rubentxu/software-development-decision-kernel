# Definition of Done

## For a kernel/application feature
- no forbidden dependency introduced;
- domain semantics covered by deterministic unit tests;
- application use case has fake-port tests;
- relevant events have versioned schemas;
- observability fields/correlation defined;
- no direct side-effect bypass;
- migration/compatibility considered.

## For an adapter
- host/provider API details isolated;
- contract tests;
- error mapping to canonical taxonomy;
- unknown/unavailable metrics remain unknown;
- version compatibility documented;
- secrets/redaction tested.

## For a workflow pack
- manifest validates;
- capabilities resolve;
- schemas exist;
- side effects classified;
- human gates explicit;
- can run using fake adapters;
- emits enough events to explain failure/progress;
- no kernel special case added.

## For a behavior
- deterministic where possible;
- idempotency key defined;
- loop/depth behavior tested;
- trigger/subscriptions documented;
- emits typed events/signals, not log-text prompts.

## For Cockpit view
- backed by rebuildable projection;
- filters/drill-down link to source event/evidence;
- unknown data represented honestly;
- sensitive data redacted;
- static `file://` mode tested.
