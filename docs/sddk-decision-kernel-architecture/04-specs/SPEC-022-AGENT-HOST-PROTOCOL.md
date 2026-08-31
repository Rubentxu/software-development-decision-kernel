# SPEC-022 — AgentHost Protocol

**Status:** Proposed

## Purpose
Create a stable SDDK abstraction over OpenCode and other agentic IDEs/hosts.

## Split interfaces

### Event side

```rust
trait AgentHostEventAdapter {
    fn normalize(&self, raw: RawHostEvent) -> Result<Vec<CanonicalEvent>>;
}
```

### Control side

```rust
trait AgentHostControl {
    async fn create_session(&self, req: CreateSession) -> Result<HostSessionId>;
    async fn execute(&self, req: ExecuteAgentTurn) -> Result<HostExecutionId>;
    async fn send_context(&self, req: ContextInjection) -> Result<()>;
    async fn abort(&self, id: HostExecutionId) -> Result<()>;
    async fn resume(&self, req: ResumeExecution) -> Result<HostExecutionId>;
}
```

Optional capability methods are advertised rather than assumed:

```yaml
features:
  event_stream: true
  per_turn_model: true
  hot_switch_model: false
  context_injection: true
  abort: true
  resume_session: true
  usage_reporting: true
  tool_event_stream: true
```

## OpenCode adapter strategy
Use supported event/session APIs as adapter details. If hot switch is unstable/unavailable, SDDK can create a new attempt/turn with another model and inject a recovery capsule.

## Canonical mapping examples

```text
OpenCode session.error -> provider.* or agent.execution.failed
OpenCode tool event    -> tool.execution.*
OpenCode session idle  -> agent.session.idle
```

Raw payload may be stored as bounded diagnostic metadata, but domain decisions consume canonical fields.

## Host independence test
A fake host and OpenCode host must pass the same contract tests for supported capabilities.
