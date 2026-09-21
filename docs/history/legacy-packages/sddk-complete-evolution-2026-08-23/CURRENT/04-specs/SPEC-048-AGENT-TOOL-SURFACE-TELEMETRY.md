# SPEC-048 — Agent Tool Surface, Schema Compilation & Usage Telemetry

**Status:** Proposed

## Purpose

Expose a small semantic tool surface and measure whether it improves real agent behavior.

## Canonical semantic functions

Conceptually:

```text
state
goal.plan
goal.apply
query
evidence.submit
```

Adapters may use different external naming, but canonical contracts remain stable.

## Dynamic tool exposure

Select tools using:

```text
logical agent
current capability
goal
state
technology profile
authority/effect policy
```

Do not expose the complete CLI catalog to every agent.

If information is already fresh in the ContextCapsule, redundant read tools may be omitted.

## Canonical ToolDescriptor

```yaml
tool:
  semantic_id: goal.apply
  use_when:
    - desired SDDK state is known
  avoid_when:
    - only current state is needed
  inputs:
    schema: goal-apply/v1
  outputs:
    schema: goal-result/v1
  effects:
    dynamic: true
  retry:
    semantics: resume_or_reconcile
  common_blockers:
    - policy_denied
    - missing_evidence
    - approval_required
```

## Tool Schema Compiler

One semantic source should generate or validate:

```text
Rust types
JSON Schema
CLI help
MCP/tool descriptor
compact model-facing schema
human docs/examples
```

Provider-specific compression is allowed only if semantics are unchanged.

## Tool examples

Examples can be curated fixtures or sanitized successful historical calls.

Example identity/version is tracked.

## Usage events

Normalize:

```text
agent.tool.exposed
agent.tool.requested
agent.tool.completed
agent.tool.failed
agent.tool.invalid_arguments
agent.tool.help_requested
agent.tool.redundant_read
agent.goal.started
agent.goal.completed
```

## ToolUseRecord

```yaml
tool_use:
  run_id: ...
  agent_id: ...
  model_route: ...
  semantic_tool: ...
  goal_context: ...
  snapshot_fingerprint: ...
  input_hash: ...
  status: success|failure|invalid|blocked
  latency_ms: ...
  result_bytes: ...
  result_token_estimate: ...
```

No secrets/private chain-of-thought.

## Metrics

```text
tool_calls_per_goal
low_level_calls_per_goal
help_calls_per_goal
invalid_argument_rate
tool_error_rate
repeated_query_rate
same_state_repeated_query_rate
redundant_read_rate
result_tokens_per_goal
goal_completion_rate
goal_latency
manual_recovery_rate
```

## Agent Interface Entropy

Operational heuristic:

For a normalized goal, measure dispersion/variation of tool trajectories.

High dispersion combined with failures/help/retries is a signal to investigate the interface.

Do not optimize this metric without preserving quality/report parity.

## GCI integration

Usage trajectories feed:

```text
ExperienceEpisode
→ PatternSignal
→ InterfaceImprovementProposal
```

No automatic command consolidation.

A new high-level goal/tool is evaluated against the existing interface in Workflow Laboratory.

## Tool trajectory completeness

Telemetry itself must not replace domain reports.

It measures interface behavior while the underlying GoalRun continues to create all normal reports/evidence.
