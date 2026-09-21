# Roadmap Integration — Engineering Assurance

**Status:** Proposed amendment to the current Decision Kernel roadmap.

## Strategy

Do not create a new monolithic phase. Add **Engineering Assurance** as a cross-phase vertical slice, similar to durable debt remediation, so it lands incrementally on existing kernel capabilities.

## Cross-phase slice

| Existing roadmap dependency | Engineering Assurance capability unlocked |
|---|---|
| Phase 1 — Hexagonal convergence | Dogfood focused boundaries and architecture obligations on SDDK itself |
| Phase 2 — Canonical Event Ledger | `engineering.*` lifecycle events become replayable |
| Phase 3 — Workflow Runtime v2 | Assurance workflows compile to Task/Sequence/Parallel/Gate/SubWorkflow |
| Phase 4 — Dynamic workflow engine | Dynamic review dimensions and Map/Join specialist fan-out |
| Phase 5 — AgentHost/routing | Route one capability across local model, remote model, analyzer or human |
| Phase 6 — Supervisor/reactive | Risk/context can propose additional review capability nodes |
| Phase 7 — Context Compiler | Minimal assessment capsules, source revision and stale-evidence handling |
| Phase 8 — SDD Adaptive | SHAPE/CONVERGE consume assurance obligations and evidence |
| Phase 9 — Workflow Laboratory | Compare review depth, handoffs, cost and accepted-defect escape rate |
| Phase 10 — Active Graph | Assessment → finding → obligation → evidence causal graph; `sddk why` |
| Phase 11 — Static Cockpit | Assurance report, invariant map and hot-path trace views |
| Phase 12 — UAT | Technical evidence can support defects/retests without replacing human oracle |
| Phase 13 — Multi-pack proof | SDD/UAT/Incident consume the same assurance capabilities with zero kernel special cases |
| Phase 14 — Hardening | Signed/verified high-risk evidence and supply-chain linkage |

## Proposed milestones

### EA-0 — Contract and content hygiene

Deliver:

- ADR-041..043;
- SPEC-042..045;
- compact `systems-reasoning` skill;
- refactored `rust-systems-reasoning` delta skill;
- no kernel code changes.

Exit:

- no overlap with `rust-patterns`;
- skill style checks pass;
- pack/event/profile schemas frozen at v1 draft.

### EA-1 — Pack skeleton and deterministic evidence core

Deliver:

- `sddk-pack-engineering-assurance` manifest;
- assessment/finding/obligation/evidence structs inside pack crate/module, not kernel;
- deterministic verdict evaluator;
- fixtures and replay-safe event payloads.

Exit:

- pack validates through Pack Registry;
- kernel has no `Engineering*` domain types;
- verdict fixtures cover PASS/PW/FAIL/INCONCLUSIVE.

### EA-2 — Rust reference profile + dogfooding

Deliver:

- `engineering.systems.v1`;
- `engineering.rust.v1`;
- provider manifests for compiler/clippy/tests;
- optional Miri/fuzz/Kani evidence adapters;
- SDDK self-assessment workflow.

Dogfood targets:

- focused ports vs legacy `Ledger`;
- Event Envelope/Registry invariants;
- WorkflowIR determinism;
- event replay/projection rebuild;
- adapter/core dependency direction;
- concurrency/lease state transitions.

Exit:

- assessment produces useful findings without Rust concepts entering kernel APIs;
- zero-copy/formal checks remain conditional, not blanket gates.

### EA-3 — Existing SDD verification bridge

Deliver:

- adapter from current architecture/test/design verification outputs to normalized assurance findings/evidence;
- no duplicate analyzer execution;
- ChangeContract verification obligations bridge for `sdd-adaptive`.

Exit:

- A-full/A-lite/A-min behavior remains compatible;
- adaptive SDD can request `architecture.review` / `systems.review` semantically.

### EA-4 — Dynamic assurance composition

Deliver:

- risk/signal-driven dimension selection;
- dynamic `Map` of specialist review scopes;
- deterministic Join/adjudication;
- bounded budgets/convergence.

Exit:

- representative low-risk workflow does not pay deep-review cost;
- high-risk workflow expands reproducibly and eventfully.

### EA-5 — Active Graph + Cockpit

Deliver:

- assessment/finding/obligation/evidence projection;
- `sddk why` hooks;
- static views: Assurance Report, Invariant Map, Hot Path Trace.

Exit:

- all views rebuild from ledger + artifacts.

### EA-6 — Multi-language and laboratory evaluation

Deliver at least two non-Rust profiles and run controlled comparisons.

Candidate profiles:

- Go;
- TypeScript/Node;
- JVM;
- C/C++.

Promotion criteria:

- no regression in accepted quality;
- measurable reduction in duplicate reviews/context;
- bounded false-positive rate;
- evidence completeness improvement;
- no kernel domain leakage.

## Roadmap text amendment

Add after the existing durable technical-debt cross-phase slice:

> **Cross-phase slice — Engineering Assurance:** reusable evidence-backed engineering review capabilities live in an optional domain pack. Skills supply reasoning, technology profiles supply language/runtime specialization, deterministic tools supply evidence, and kernel capability routing remains the integration surface. Rust is the first reference profile and is dogfooded on SDDK itself; no language-specific ontology enters the kernel.
