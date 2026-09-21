# Event Ledger & Active Graph Model

## Authority

```text
Event Ledger = authoritative operational history
Active Graph = rebuildable semantic/causal projection
Cockpit = read projection / moldable lenses
```

## New dynamic-workflow rule
The execution graph may change during a run, but every mutation is a validated evented transition. A graph mutation that is not represented in the ledger is invalid.

## Core event families

### Workflow lifecycle
- `workflow.run.started|paused|resumed|completed|failed|cancelled`
- `workflow.ir.compiled|validated|rejected`
- `workflow.graph.expansion.proposed|approved|rejected`
- `workflow.graph.revision.created`
- `workflow.node.created|ready|started|waiting|completed|failed`
- `workflow.join.ready|completed`
- `workflow.loop.iteration.started|completed|max_reached`

### SDD adaptive
- `change.contract.created|revised|accepted`
- `work.unit.discovered|scheduled|completed`
- `convergence.round.started|completed`
- `convergence.gap.detected|resolved`
- `verification.plan.adapted`

### Agent/provider
Existing execution/routing/failover event families remain unchanged.

## Canonical pipeline

```text
Raw Host/Internal Event
 → Adapter/domain normalization
 → CanonicalEvent validation
 → append
 → projection fanout
 → ReactionPolicy
 → Behavior/Supervisor if required
```

## Active Graph node types
- Goal
- WorkflowTemplate
- WorkflowIR
- WorkflowRun
- ExecutionGraphRevision
- NodeRun
- WorkUnit
- ChangeContract
- Requirement
- AcceptanceCriterion
- Attempt
- LogicalAgent
- Capability
- AgentHost
- Provider
- Model
- Artifact
- Evidence
- Decision
- ContextCapsule
- HumanDecision
- Failure
- Receipt
- Policy
- UatScenario/UatRun
- WorkflowExperiment

## Edge examples

```text
Goal --shaped-as--> ChangeContract
WorkflowTemplate --compiled-to--> WorkflowIR
WorkflowIR --instantiated-as--> WorkflowRun
WorkflowRun --has-revision--> ExecutionGraphRevision
WorkflowRun --contains--> NodeRun
ChangeContract --decomposed-into--> WorkUnit
WorkUnit --executed-as--> NodeRun
Requirement --implemented-by--> Artifact/CodeRef
Requirement --verified-by--> Evidence
NodeRun --attempted-by--> Attempt
Attempt --routed-to--> Model
Attempt --used-context--> ContextCapsule
Attempt --failed-by--> Failure
Decision --supported-by--> Evidence
WorkflowExperiment --compares--> WorkflowRun
```

## Behavioral semantics
Behaviors react to events/projections and emit commands/new events. Typical dynamic workflow behaviors:
- `NodeReadinessBehavior`;
- `MapExpansionBehavior`;
- `JoinBehavior`;
- `LoopGuardBehavior`;
- `ConvergenceBehavior`;
- `BudgetBehavior`;
- `ProviderHealthBehavior`.

## Replay
Projection replay recreates exact graph revisions from ledger events. It MUST NOT re-run side effects.

## Fork/replay for workflow experiments
A fork can change model, prompt, policy, WorkflowIR or workflow strategy from a selected causal point. The original history remains immutable. This underpins the Workflow Laboratory.
