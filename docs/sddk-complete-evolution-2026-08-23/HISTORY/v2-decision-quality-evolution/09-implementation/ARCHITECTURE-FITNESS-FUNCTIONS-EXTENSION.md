# Architecture Fitness Functions — Proposed Extension

Continue after current ARCH001..ARCH015.

- **ARCH016 Assurance remains pack-owned.** Kernel/domain cannot define Engineering Assurance bounded-context entities.
- **ARCH017 Technology neutrality.** Kernel/runtime cannot branch on Rust/Go/C++/TypeScript/JVM.
- **ARCH018 Skills have no authority.** Skill loading never grants effects.
- **ARCH019 Review is read-only by default.** Engineering Assurance v1 cannot mutate source.
- **ARCH020 Blocking finding requires evidence.** Prose-only blocking finding is invalid/inconclusive.
- **ARCH021 Profile isolation.** Adding technology profile cannot require WorkflowIR/CycleState/kernel event changes.
- **ARCH022 Deterministic assurance verdict.** Authoritative verdict comes from normalized deterministic inputs.
- **ARCH023 Source evidence freshness.** Stale required evidence cannot satisfy obligations.
- **ARCH024 Conditional systems depth.** Unsafe/zero-copy/formal/deep-concurrency checks activate only when relevant.
- **ARCH025 Learning does not mutate authority.** Pattern/reflection code cannot directly activate durable configuration.
- **ARCH026 Optimizer cannot read holdout.** Access separation is testable.
- **ARCH027 Candidate isolation.** Mutating experiments use fork/worktree/sandbox.
- **ARCH028 Promotion requires evaluation receipt.** No active-version update without evaluation + policy refs.
- **ARCH029 Replay never re-promotes/re-executes.** Replay only rebuilds lifecycle state.
- **ARCH030 No generic autoresearch ontology in kernel.** Scientific Theory/Hypothesis remain external pack concerns.
- **ARCH031 Configuration identity completeness.** Causal comparisons record hashes/versions of mutable harness dimensions.
- **ARCH032 Quality-first promotion.** Cost/token gains cannot bypass quality invariants.

## Ratchet

```text
document → measure → fixture → warning → baseline exception → error
```

Do not enable every proposed rule as a hard failure immediately.
