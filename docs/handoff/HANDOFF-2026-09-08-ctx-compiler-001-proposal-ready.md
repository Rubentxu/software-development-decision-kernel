# CTX-COMPILER-001 — Context Capsule Compiler Substrate (Proposal ready)

- status: PROPOSAL READY
- cycle: CTX-COMPILER-001 (order 250, horizon H4 — Agent Host)
- depends on: AGENT-HOST-002 (v1.95.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-081-CONTEXT-CAPSULE-COMPILER.md` (proposed, P10-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ContextCapsuleCompiler.md` (proposed, 391 lines)
- canonical sources: ADR-028, SPEC-021

## What this cycle delivers

A `ContextCompiler` substrate that produces bounded, deterministic,
provenance-aware context capsules per SPEC-021:

1. **`ContextCapsule`** — kernel first-class type implementing
   SPEC-021 §Schema verbatim (`capsule_id`, `for_target`,
   `objective`, `definition_of_done`, `scope`, `constraints`,
   `decisions`, `assumptions`, `artifacts`, `negative_knowledge`,
   `changes_since_parent`, `evidence_required`, `tools_allowed`,
   `budget`, `return_contract`, `recovery`, `provenance`,
   `staleness`).
2. **`CapsuleInputs` trait** — kernel seam for the workflow/ledger/
   artifact sources. Implementations: `InMemoryCapsuleInputs` for
   tests, pluggable adapters in future cycles.
3. **`ContextCompiler`** struct with `compile(target)` and
   `compile_delta(target, parent)`.
4. **Staleness rules** (5 reasons: ContentChangedSinceCompile,
   DecisionOverridden, ArtifactRemoved, ProviderRevoked, BeyondTtl).
   Stale refs are **annotated**, not removed (SPEC-021 §Mark stale).
5. **`CompilerPolicy`** (max_tokens, staleness_ttl_ms,
   preserve_negative_knowledge).
6. **Determinism contract**: capsule_id =
   `<workflow_run>:<node_run>:<attempt>`; iteration order =
   `CapsuleInputs` return order; `as_of_ms = clock.now_ms()`.

## Shape chosen: P10-a (recommended, adopted)

- `ContextCompiler` struct (mirrors `AgentHost` and `ProviderRouter`
  patterns).
- One trait seam (`CapsuleInputs`) — input contract only; no
  internal policy engine.
- `InMemoryCapsuleInputs` for tests + the receipt-replay adapter
  in future cycles.
- Chars/4 token estimator in apply phase; real tokenizers in
  CTX-COMPILER-002.

P10-b (free-function compiler) and P10-c (actor compiler) rejected —
see ADR-081 §3 for rationale.

## Acceptance criteria (from spec §Determinism acceptance demo)

Two `compile` calls with identical inputs and `MockClock::new(1000)`
MUST produce byte-stable serialized capsules
(`serde_json::to_vec(&capsule1) == serde_json::to_vec(&capsule2)`).
Implemented as `context_capsule_tests.rs::scenario_determinism_byte_stable`.

## Test scenarios (10 in spec)

1. Fresh compile produces bounded capsule
2. Stale ref is annotated, not removed
3. Delta compile preserves negative knowledge
4. Budget exceeded fails closed
5. Decision overridden marks stale
6. Determinism: byte-stable across runs
7. Privacy: excluded scope does not leak
8. Recovery preserves attempt chain
9. TTL-based staleness
10. Empty objective is rejected

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/context_capsule.rs` (~600 lines):
  types + `ContextCompiler` + `CapsuleInputs` + `InMemoryCapsuleInputs`
- New traits re-exported in `lib.rs`
- `crates/sddk-engine/tests/context_capsule_tests.rs` (10 scenarios)
- Target version: **v1.96.0** (next minor after v1.95.0)

## Toxicology check (decision sanity)

- No deviation from SPEC-021 §Schema (canonical).
- No deviation from ADR-028 rules (encoded in compiler contract).
- P10-a follows the same architectural pattern as AGENT-HOST-001/002
  (struct + trait + plain-data policy).
- Token budget enforcement is fail-closed (no partial capsules).
- Negative knowledge preservation is the default; drop is policy-driven.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ContextCapsuleCompiler.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-081-CONTEXT-CAPSULE-COMPILER.md`
- Spine: CTX-COMPILER-001 PROPOSED (unchanged from baseline)
- Local workspace: clean
- Binary: `sddk 1.95.0`
