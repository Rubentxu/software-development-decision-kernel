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

## ARCH033 — No prompt-owned deterministic procedure

Stable repeatable deterministic sequences representable by application services/WorkflowIR MUST NOT be authoritative only in prompt Markdown.

## ARCH034 — Goal preserves obligation closure

A GoalRun cannot succeed with an unsatisfied mandatory invariant/report/evidence/receipt obligation.

## ARCH035 — Reporting parity

Migrating a legacy command sequence to a high-level goal requires a behavioral parity fixture for mandatory outputs.

## ARCH036 — Semantic tool surface, not CLI mirroring

Agent API MUST NOT mechanically expose every CLI subcommand unless an explicit use case justifies it.

## ARCH037 — No recursive CLI shell-out

Application services implementing high-level goals MUST NOT invoke the `sddk` binary to perform low-level operations.

They call ports/services directly.

## ARCH038 — Adapter parity

CLI, stdio/MCP and AgentHost adapters cannot define independent policy/gate semantics.

## ARCH039 — Operation idempotency declared

Every operation eligible for GoalPlanner execution declares idempotency/retry semantics.

## ARCH040 — Cached result must prove freshness

`UP_TO_DATE` cannot be returned without a valid input/revision fingerprint and compatible operation version.

## ARCH041 — Interface efficiency cannot trade away quality

Tool-call/token/latency metrics cannot satisfy a promotion gate if behavioral/report completeness regresses.

## ARCH042 — Sensitive mechanics stay internal where possible

Agents should not need to propagate fencing tokens, internal paths or provider executable details when the runtime can resolve them safely.

## ARCH043 — Low-level recovery surface retained during migration

No high-level migration deletes a required expert/debug/recovery path before parity and replacement evidence exist.
