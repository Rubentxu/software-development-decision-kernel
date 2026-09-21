# CTX-COMPILER-002 — Agent Host Cold-Start Recovery (Proposal ready)

- status: PROPOSAL READY
- cycle: CTX-COMPILER-002 (order 260, horizon H4 — Agent Host)
- depends on: CTX-COMPILER-001 (v1.96.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-082-AGENT-HOST-COLD-START.md` (proposed, P11-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ColdStartRecovery.md` (proposed, 276 lines)
- canonical sources: ADR-028, SPEC-021, H3 RunStateView

## What this cycle delivers

A `cold_start` method on `AgentHost` that lets a fresh session
reconstruct working context without prior chat memory:

1. **`AgentHost::cold_start(workflow_run, node_run)`** composes a
   `CapsuleInputs` adapter from `CapsuleStore` + `RunStateViewInputs`
   and delegates to `ContextCompiler::compile_delta` (parent exists)
   or `compile` (no parent).
2. **`CapsuleStore` trait** + `InMemoryCapsuleStore` reference impl
   for durable capsule history.
3. **`RunStateViewInputs` trait** for the H3 `RunStateView` seam.
4. **`CapsulePersistence` hook** for write-back (default no-op).
5. **`ColdStartSource { FromRecovery, Fresh }`** to distinguish
   "rebuilt from prior" vs "first cold-start".
6. **`ColdStartError { RunNotFound, NoFrontier, CompileFailed,
   InvalidRecoveryState }`** closed-set taxonomy.

## Shape chosen: P11-a (recommended, adopted)

- One new method on `AgentHost` (mirrors `execute_decision` /
  `execute_through_router`).
- Three small trait seams (`CapsuleStore`, `RunStateViewInputs`,
  `CapsulePersistence`).
- `InMemoryCapsuleStore` + `InMemoryRunStateViewInputs` are kernel
  reference impls.

P11-b (free function) and P11-c (builder/actor) rejected — see
ADR-082 §3 for rationale.

## Acceptance criteria (from spec §Determinism contract)

Two `cold_start` calls with identical `CapsuleStore` content +
identical `RunStateViewInputs` content + identical `Clock::now_ms()`
+ identical `CompilerPolicy` MUST produce byte-stable
`ContextCapsule` serialization. Implemented as
`cold_start_tests.rs::scenario_determinism_byte_stable`.

## Test scenarios (10 in spec)

1. Cold-start with no prior capsule produces Fresh
2. Cold-start with parent capsule produces FromRecovery
3. Cold-start stitches blockers into must_read
4. Run not found returns RunNotFound
5. No frontier returns NoFrontier
6. Determinism: byte-stable across cold-starts
7. Persistence hook records the new capsule
8. Blockers from RunStateView supersede parent's must_read
9. Compiled capsule honors budget
10. Stale refs from CTX-COMPILER-001 are preserved

## Apply-phase deliverables (next session)

- Extend `crates/sddk-engine/src/agent_host.rs`:
  - Add `CapsuleStore`, `RunStateViewInputs`, `CapsulePersistence`
    traits (or new module `crates/sddk-engine/src/cold_start.rs`).
  - Add `InMemoryCapsuleStore`, `InMemoryRunStateViewInputs`.
  - Add `ColdStartOutput`, `ColdStartSource`, `ColdStartError`.
  - Add `AgentHost::cold_start(...)` method.
- Wire `AgentHost::new` or add a builder to accept the new traits
  (preserve default in-memory impls).
- Re-export from `lib.rs`.
- `crates/sddk-engine/tests/cold_start_tests.rs` (10 scenarios).
- Target version: **v1.97.0** (next minor after v1.96.0).

## Toxicology check (decision sanity)

- No deviation from CTX-COMPILER-001 (uses its types directly).
- No deviation from ADR-028 rules (must-read, negative knowledge).
- P11-a follows the same architectural pattern as AGENT-HOST-001/002
  and CTX-COMPILER-001 (struct + trait + plain-data policy).
- Blockers-as-must-read ensures the cold-start worker sees the
  blocking evidence first.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ColdStartRecovery.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-082-AGENT-HOST-COLD-START.md`
- Spine: CTX-COMPILER-002 PROPOSED (unchanged from baseline)
- Local workspace: clean
- Binary: `sddk 1.96.0`
