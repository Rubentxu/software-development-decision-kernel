# Study & Adoption Guide

## Track A — Understand the idea (60–90 min)
1. `NAMING.md`
2. `PRODUCT-VISION.md`
3. `PRINCIPLES.md`
4. `../01-architecture/DESIGN.md`
5. `../01-architecture/EVENT-AND-GRAPH-MODEL.md`

Goal: understand why SDDK becomes a Decision Kernel and why Event Ledger + reactive graph + deterministic runtime is the central model.

## Track B — Dynamic workflow architecture
1. ADR-024 — Generic Workflow IR.
2. ADR-037 — Dynamic Workflow Compilation.
3. `../04-specs/SPEC-023-WORKFLOW-RUNTIME-V2.md`.
4. `../04-specs/SPEC-037-DYNAMIC-WORKFLOW-COMPILER.md`.
5. `../04-specs/SPEC-039-WORKFLOW-PATTERN-ALGEBRA.md`.
6. `../05-workflows/WORKFLOW-PATTERNS.md`.

Goal: understand Template → Compiler → IR → Runtime → Dynamic Execution Graph.

## Track C — Compact/adaptive SDD
1. ADR-038 — Invariant-Driven SDD.
2. ADR-039 — Adaptive Verification.
3. `../04-specs/SPEC-038-SDD-ADAPTIVE.md`.
4. `../05-workflows/SDD-ADAPTIVE-WORKFLOW.md`.
5. `../04-specs/SPEC-040-WORKFLOW-LABORATORY.md`.
6. `../08-spikes/SPIKE-007-SDD-ADAPTIVE-ABLATION.md`.

Goal: preserve quality guarantees with fewer mandatory phase boundaries.

## Track D — Plan repository evolution
1. `../02-roadmap/MIGRATION-PLAN.md`
2. `../09-implementation/ARCHITECTURE-FITNESS-FUNCTIONS.md`
3. `../09-implementation/REPOSITORY-TARGET-LAYOUT.md`
4. ADR-032 focused ports.
5. `../09-implementation/DYNAMIC-WORKFLOW-IMPLEMENTATION-PLAN.md`.

## Track E — Solve provider quota/failover first
1. AgentHost Protocol.
2. Execution Router.
3. Provider Health & Failover.
4. Reactive Behaviors.
5. `../08-spikes/SPIKE-001-OPENCODE-EVENT-CONTROL.md`
6. `../08-spikes/SPIKE-002-PROVIDER-FAILOVER.md`

Goal: demonstrate `same NodeRun → failed Attempt A → successful Attempt B`.

## Track F — Build independent visibility
1. Event Taxonomy.
2. Observability & Usage.
3. Cockpit spec.
4. `../06-control-plane/EVENT-JOURNAL.md`
5. `../06-control-plane/MOLDABLE-VIEWS.md`
6. Static Cockpit spike.

## Track G — Advanced agent intelligence
1. Supervisor Runtime.
2. Context Capsules.
3. Active Graph model.
4. Agent Evaluation.
5. Workflow Laboratory.
6. Fork/Replay/Diff.

## Recommended implementation order
Do **not** start by replacing the current SDD flow. Deliver two vertical slices:

```text
Slice 1 — resilience
focused ports → canonical events → WorkflowRun/NodeRun/Attempt
→ OpenCode adapter → injected quota failure → failover → Journal

Slice 2 — adaptive workflows
WorkflowTemplate → WorkflowIR validator → dynamic Map/Join
→ sdd-adaptive SHAPE/BUILD/CONVERGE/INTEGRATE
→ compare against A-full in Workflow Laboratory
```
