# Event Journal

## Purpose
Human-readable projection of canonical events. The Event Ledger is optimized for correctness; Journal is optimized for investigation.

## Example

```text
13:01:14 INFO     workflow  Architecture review started
13:02:03 AGENT    route     sddk-architect → Claude
13:08:32 ERROR    provider  Weekly quota exhausted
13:08:32 WARN     routing   Anthropic circuit opened
13:08:33 INFO     routing   Recovery route GPT selected
13:08:34 AGENT    execution Architect resumed as Attempt #2
13:19:02 SUCCESS  workflow  Node completed
```

## Filters
- time range;
- repository/workspace;
- session;
- workflow/node/attempt;
- event category/type;
- severity;
- logical agent;
- host;
- provider/model;
- capability;
- correlation ID.

## Journal entry projection

```yaml
timestamp: ...
severity: warning
category: routing
summary: "Anthropic circuit opened"
details:
  reason: weekly_quota_exhausted
links:
  event: evt-...
  provider: anthropic
  attempt: at-...
```

## Severity rules
Severity is computed from event + policy/context; it is not part of immutable semantic event type.

## CLI

```bash
sddk journal
sddk journal --workflow wf-123 --severity warn,error
sddk journal --provider anthropic --since 7d
```
