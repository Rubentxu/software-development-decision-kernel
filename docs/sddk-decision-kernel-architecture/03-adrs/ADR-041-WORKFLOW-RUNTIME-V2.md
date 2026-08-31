# ADR-041 — WorkflowRuntime v2: Deterministic Core with Canonical Event Emission

**Status:** Accepted

## Context

The SDDK kernel needs a deterministic, event-driven workflow execution engine that:
1. Is replayable and auditable from its event log
2. Emits canonical typed events that serve as the operational authority
3. Decouples the operator evaluation model from runtime concerns (state, storage, event emission)

Prior cycles established the Event Ledger architecture (ADR-021) and the WorkflowIR model (ADR-024). This ADR establishes the **WorkflowRuntime** as the durable execution primitive.

## Decision

The SDDK kernel ships a **WorkflowRuntime** that:

### State Machine

```
Pending → Running → Completed
              ↓
            Failed
```

- `start()`: Pending → Running (transitions run to Running, emits `workflow.run.started`)
- `tick()`: evaluates ready operators, transitions node states, emits `workflow.node.running` / `workflow.node.completed` / `workflow.node.failed`
- `complete()`: Running → Completed (emits `workflow.run.completed`)
- `fail(error)`: Running → Failed (emits `workflow.run.failed`)

### Event Emission

All 5 canonical events are registered in `EventSchemaRegistry` and emitted via `emit_*` functions:

| Event | Schema | Trigger |
|---|---|---|
| `workflow.run.started` | `WorkflowRunStartedSchema` | `start()` |
| `workflow.run.completed` | `WorkflowRunCompletedSchema` | `complete()` |
| `workflow.run.failed` | `WorkflowRunFailedSchema` | `fail()` |
| `workflow.node.running` | `WorkflowNodeRunningSchema` | node evaluation begins |
| `workflow.node.completed` | `WorkflowNodeCompletedSchema` | node succeeds |
| `workflow.node.failed` | `WorkflowNodeFailedSchema` | node fails |

`stream_id == run_id` on all events, enabling partition-based projection.

### Operator Trait

```rust
pub trait Operator: Send + Sync {
    fn kind(&self) -> &'static str;
    fn evaluate(&self, ctx: &mut OperatorContext<'_>) -> Result<NodeOutcome, OperatorError>;
}

pub enum NodeOutcome {
    Succeeded { node_id: NodeId, outputs: BTreeMap<String, Value> },
    Failed { node_id: NodeId, error: String },
    Pending { node_id: NodeId },
}
```

### Ports

- `GraphStore`: persists execution graph, revisions, node runs
- `EventStore`: appends typed event envelopes (via `Arc<Mutex<dyn EventStore>>` adapter)
- `Clock`: provides wall/monotonic time for event timestamps

### WorkflowRuntime Fields

```rust
pub struct WorkflowRuntime<R: Runtime> {
    pub(crate) ir: WorkflowIR,
    pub(crate) run: WorkflowRun,
    pub(crate) graph: ExecutionGraph,
    pub(crate) store: R,
    pub(crate) clock: Clock,
    pub(crate) event_store: Option<Arc<Mutex<dyn EventStore>>>,
}
```

`event_store` is optional — if `None`, events are silently dropped (test mode).

### Design Principles

1. **No async in the runtime core**: cycle-16 is sync-only; async is deferred to cycle-17
2. **Task is a no-op in cycle-16**: capability dispatch deferred to cycle-17
3. **Events are fire-and-forget**: runtime does not wait for subscriber acknowledgment
4. **Arc<Mutex<Box<dyn EventStore>>>**: ergonomic adapter so tests can spy on events without cloning

## Alternatives Considered

- **Actor model**: overkill for deterministic sequential execution; adds message-passing overhead
- **STM (Software Transactional Memory)**: appropriate for optimistic concurrency but adds complexity for single-writer semantics
- **Async runtime (Tokio)**: would block on I/O in the core; async deferred to cycle-17

## Consequences

- Workflow runs are replayable from the event log
- Event projection enables Cockpit dashboard without querying the runtime
- Operator trait enables capability routing without hard-coding agent types
- Cycle-17 focus: async capability dispatch, retry/backoff policies, fan-out parallelism
