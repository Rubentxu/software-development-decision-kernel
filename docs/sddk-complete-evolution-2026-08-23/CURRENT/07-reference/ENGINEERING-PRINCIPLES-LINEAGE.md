# Engineering Principles Lineage

Purpose: preserve where an idea came from, how SDDK interpreted it, and whether it is mandatory or heuristic.

| Inspiration | Original lesson | SDDK interpretation | Location | Strength |
|---|---|---|---|---|
| Netstack3 Core/Bindings | isolate platform-dependent complexity | kernel/application boundaries explicit; adapters own incidental environment concerns | Architecture / systems-reasoning | strong heuristic |
| Netstack3 traceable core | keep core behavior understandable | stable deterministic flows belong in Rust services/WorkflowIR, not prompts | ADR-045 | principle |
| Netstack3 type/invariant reasoning | encode guarantees explicitly | WorkflowIR/domain types/obligations/evidence | Assurance specs | principle |
| zerocopy work | prove representation preconditions | Rust profile activates layout/unsafe checks conditionally | SPEC-043/045 | conditional |
| Kani/formal methods | targeted proof where valuable | verification ladder based on risk/consequence | Rust profile | optional |
| Maven lifecycle | users request an outcome | agents request Goal instead of listing all deterministic steps | ADR-045/SPEC-047 | principle |
| Gradle task graph | dependencies form DAG | GoalPlanner uses OperationContracts | SPEC-047 | principle |
| Gradle UP-TO-DATE | skip valid repeated work | input/revision fingerprints enable safe reuse | SPEC-047 | conditional |
| controller reconciliation | converge actual toward desired | GoalRun observe/plan/execute/verify/reobserve | SPEC-047 | principle |
| Hermes skill curation | procedural knowledge needs lifecycle | skill staleness/consolidation candidates | SPEC-046 | incremental |
| GEPA | rich traces can generate better candidates | optional GCI candidate generator | SPEC-046 | research later |
| DGM | preserve candidate diversity/lineage | parent refs + lineage projection, not self-modifying kernel | SPEC-046 | optional |
| AFlow | workflow structure can be optimized | WorkflowIR candidates in Laboratory | SPEC-046 | research later |
| agent tool research | narrow semantic tools beat overlapping mechanics | Agent Tool Surface | SPEC-048 | principle |
| process mining | real traces expose process/interface problems | deterministic tool trajectory projection | Tool-use plan | later |

## Important distinction

A lineage entry documents **inspiration**, not dependency.

SDDK keeps only the abstraction that fits its goals.
