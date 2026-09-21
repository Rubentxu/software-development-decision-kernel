# Dynamic Workflow Runtime — Implementation Plan

## Increment 1 — Contracts
- `WorkflowTemplate`;
- `WorkflowIR`;
- operator enums/value objects;
- `ExecutionGraphRevision`;
- `ExpansionProposal`;
- event types and serializers.

## Increment 2 — Static interpreter
Implement Sequence/Parallel/Choice/Gate/Wait over existing NodeRun/Attempt model. No cognitive planning required.

## Increment 3 — Dynamic Map/Join
Discovery result → typed WorkUnits → expansion validation → events → revision → schedule → join.

## Increment 4 — Loop/Convergence
Bounded loop, no-progress guard, max rounds, convergence verdict/gap model.

## Increment 5 — Compiler/Validator
Canonical deterministic compiler first. Add Supervisor-generated candidate IR only after validator is robust.

## Increment 6 — Legacy SDD compiler
Map A-min/A-lite/A-full/B-direct to IR. Current behavior becomes a reference test fixture.

## Increment 7 — ChangeContract + adaptive SDD
Populate ChangeContract from existing A-full, then implement direct SHAPE and document projections.

## Increment 8 — Laboratory
Run baseline/adaptive/forks and render comparisons in Cockpit.

## Suggested module seams
```text
sddk-kernel::workflow_contract
sddk-app::compile_workflow
sddk-orchestration::compiler
sddk-orchestration::validator
sddk-orchestration::runtime
sddk-orchestration::operators
sddk-orchestration::expansion
sddk-packs-sdd::change_contract
sddk-packs-sdd::adaptive
```

## Definition of done
The runtime can execute a dynamic Map/Join workflow from ledger-backed state, kill/restart, reconstruct the same graph and reject a malicious/invalid expansion without invoking an LLM.
