# Control Flows

## 1. Goal → Template → Workflow IR

```mermaid
sequenceDiagram
  participant U as User
  participant A as Application
  participant S as Supervisor
  participant C as Workflow Compiler
  participant V as Workflow Validator
  participant W as Runtime
  U->>A: goal
  A->>S: interpret(goal, context, risk)
  S-->>A: template + planning hints
  A->>C: compile(template, registry, budget, context)
  C-->>V: WorkflowIR candidate
  V-->>A: validated IR / violations
  A->>W: start(IR, inputs)
  W-->>A: WorkflowRunStarted
```

## 2. Dynamic graph expansion

```mermaid
sequenceDiagram
  participant N as Discovery/Planner Node
  participant W as Workflow Runtime
  participant V as Expansion Validator
  participant L as Event Ledger
  N-->>W: ExpansionProposal(work_units[])
  W->>V: validate capability/policy/budget/conflicts
  V-->>W: approved expansion
  W->>L: workflow.graph.expansion.approved
  W->>L: workflow.node.created x N
  W->>L: workflow.graph.revision.created
  W-->>W: schedule newly-ready nodes
```

The LLM never mutates scheduler structures directly.

## 3. Map/Fan-out → Join

```text
DiscoverAffectedAreas
  → WorkUnits[auth, db, api, cli]
  → Map(implementation.worker, concurrency=adaptive)
  → Join(all | quorum | first-valid)
  → Integrate
```

## 4. Node execution

```mermaid
sequenceDiagram
  participant W as Workflow Runtime
  participant R as Execution Router
  participant C as Context Compiler
  participant H as AgentHost
  participant V as Verifier
  W->>R: execute(NodeRun)
  R->>C: compile context
  C-->>R: ContextCapsule
  R->>H: execute(route, capsule)
  H-->>R: events/result
  R->>V: verify(result,evidence)
  V-->>W: verification
```

## 5. SDD Adaptive convergence

```text
SHAPE
  ↓
ChangeContract
  ↓
BUILD
  ↓
CONVERGE
  ├─ PASS → INTEGRATE
  ├─ GAPS → create WorkUnits → BUILD
  └─ AMBIGUOUS/HIGH-RISK → Supervisor/Human → revised contract or work
```

## 6. Reactive provider failure

```mermaid
sequenceDiagram
  participant H as AgentHost Adapter
  participant L as Ledger
  participant B as ProviderHealthBehavior
  participant R as Execution Router
  participant S as Supervisor
  H->>L: provider.quota.exhausted
  L->>B: event
  B->>L: provider.circuit.opened
  B->>L: execution.route.invalidated
  alt compatible alternate route deterministic
    B->>R: retry same NodeRun on new route
  else semantic choice required
    B->>L: orchestrator.signal.created
    L->>S: signal
    S->>R: replan route/strategy
  end
```

## 7. Cognitive signal injection
Never concatenate raw log text. Compile:

```text
Canonical Event + projection state + workflow/node + progress
+ allowed actions + route candidates + relevant ChangeContract fragment
→ OrchestratorSignal → ContextCapsule delta → AgentHostControl
```

## 8. Governed effect

```mermaid
flowchart LR
  A[Agent Proposal] --> P[Policy]
  P -->|deny| D[Denied Event]
  P -->|approval required| H[Human Approval]
  P -->|allow| C[Scoped Capability]
  H -->|approve| C
  C --> E[Execute effect]
  E --> V[Verify postcondition]
  V --> R[Receipt]
  R --> L[(Ledger)]
```
