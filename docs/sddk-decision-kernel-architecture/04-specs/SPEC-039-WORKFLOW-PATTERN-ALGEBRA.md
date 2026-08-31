# SPEC-039 — Workflow Pattern Algebra

**Status:** Proposed

## Goal
Support a small set of generic runtime primitives that can express common workflow and agentic orchestration patterns without hardcoding every pattern in the kernel.

## Kernel primitives

| Primitive | Meaning |
|---|---|
| Task | Execute one capability/subplan |
| Sequence | A then B |
| Parallel | Fixed independent branches |
| Map | Dynamic N-way fan-out over discovered items |
| Join | all/quorum/partial aggregation |
| Race | first acceptable completion |
| Choice | guarded exclusive/multi choice |
| Loop | bounded repetition |
| Gate | policy/verification condition |
| Wait | event/time/human wait |
| SubWorkflow | invoke another workflow contract |
| Compensate | rollback/semantic compensation |

## Agentic composites
- Prompt Chain = Sequence(Task...)
- Router = Choice + Task
- Orchestrator/Workers = Planner Task + Map + Join
- Generator/Evaluator = Loop(Task generate → Task evaluate)
- Convergence = Loop(Verify → Choice(remediate|pass))
- Adversarial Review = Parallel reviewers + Join/Adjudicate
- Saga = Sequence + compensation stack
- Event Reaction = Wait/event + behavior + Task/command

## Pattern selection hints
Use simple deterministic patterns for known structures. Use orchestrator-worker/dynamic Map when decomposition emerges from discovery. Use evaluator/convergence for verifiable iterative quality. Use Human Gate for risk/policy, not as a default phase boundary.

## Constraints
Packs may name patterns for readability but MUST compile to kernel primitives.
