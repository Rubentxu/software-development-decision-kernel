# CDD-MEMORY-001 — Decision Memory Object and Ref Model (CLOSED 2026-09-09)

- status: CLOSED (apply commit `d6f5581`; spine reconcile commit `TBD`; release v1.147.0)
- cycle: CDD-MEMORY-001 (order 267, horizon H4 — CDD Handoff)
- depends on: CDD-HANDOFF-002 (v1.146.0, SHIPPED at v1.146.0 on 2026-09-09)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/drafts/ADR-087-DECISION-MEMORY-OBJECT-REF-MODEL.md` (proposed, P14-a)
- Spec (proposed): `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionMemory.md`

## Executive

CDD-MEMORY-001 establishes the **substrate** half of SDDK's
Git-like Decision Memory model: three immutable content-addressed
types (`DecisionMemoryBlob`, `DecisionMemoryTree`,
`DecisionMemoryCommit`), a canonical sha256 hashing rule, a
ref/HEAD/tags namespace with append-only reflog, and the
advisory-branch authority invariant that prevents `what-if` /
`rejected` branches from ever masquerading as canonical state.

Traversals (`log`, `show`, `diff`, `branch`, `merge-base`,
`ancestors`, `why`, `reflog`), session-delta projections, and
`ContextLease.decision_memory_head` integration are deferred to
CDD-MEMORY-002 (order 268). This cycle draws the boundary at the
substrate: types, hash, refs, reflog, authority invariant.

## Objective

Implement Git-like content-addressed Decision Memory object and ref
model so that any current or historical state of the kernel has a
deterministic, immutable, traversable address.

## Exit gate (per EXECUTION-SPINE.yaml order 267)

Immutable `DecisionMemoryCommit` / `DecisionMemoryTree` /
`DecisionMemoryBlob` objects support parents, canonical
serialization + sha256 hash, refs, `HEAD`, tags and append-only
reflog while advisory branches have no runtime authority.

## Design context (already canonical)

- `docs/sddk-decision-kernel-architecture/02-roadmap/DECISION-MEMORY-GIT-MODEL.md`
  (P11 era — 12 sections covering objects, refs, merge semantics,
  traversal targets, projections, session continuity, invariants).
- `docs/evolutivo-continuidad-sesiones-delegacion-deliberacion.md`
  (research backing the model).
- Backlog section in
  `docs/sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md`
  (`CDD-MEMORY-001` and `CDD-MEMORY-002` split).

ADR-087 ratifies the model for the engine substrate and pins the
field set, hash function, ref namespace, and authority invariants.

## In-scope (this cycle)

- Three immutable types with `id = sha256(canonical_payload)`.
- Canonical JSON encoding (sorted keys, RFC3339-UTC, integers,
  no `id` in canonical payload).
- `MemoryStore` trait + `InMemoryMemoryStore` reference impl.
- `MemoryRef`, `RefKind`, `RefAuthority`, ref-resolution helpers.
- Append-only `Reflog` with monotonic `seq`.
- `assert_canonical_authority` guard that fails closed on
  advisory-only reachability.
- Merge-receipt skeleton (forward pointer only — no parsing).
- 12 RED→GREEN scenarios (`DMT-01..DMT-12`).

## Out-of-scope (deferred)

- Traversal operations (`log`, `show`, `diff`, `branch`,
  `merge-base`, `ancestors`, `why`, `reflog`).
- `SessionCheckpoint` / `SessionDelta` projections.
- Decision / delegation branch projections.
- `ContextLease.decision_memory_head` (integration point).
- `cold_start` / `context_capsule` HEAD integration.
- Persistent backends (in-memory only for tests; persistence is a
  follow-on cycle).

## Surface contract

```text
DecisionMemoryBlob
DecisionMemoryTree
DecisionMemoryCommit
MemoryId (= [u8;32] hex-lower)
MemoryStore (trait) + InMemoryMemoryStore
MemoryRef, RefKind (Head | Branch(String) | Tag(String))
RefAuthority (Canonical | Advisory)
classify_ref, resolve, canonical_head, advisory_branch_target,
assert_canonical_authority
Reflog, ReflogEntry
DecisionMemoryError (closed enum, #[non_exhaustive])
```

All errors typed. No panic on bad input. Authority invariant
enforced at API surface and re-checked by every guard helper.

## Path

1. Write RED tests in `decision_memory_tests.rs`
   (DMT-01..DMT-12).
2. Write GREEN impl in `crates/sddk-engine/src/decision_memory.rs`.
3. Re-export at engine root in `crates/sddk-engine/src/lib.rs`.
4. `cargo fmt --check && cargo clippy --workspace --all-targets
   -- -D warnings` clean.
5. `cargo test -p sddk-engine --test decision_memory_tests`
   12/12 green.
6. Spine reconcile: order 267 PROPOSED → SHIPPED with inline
   evidence at release commit.
7. Promote ADR-087 + `REQ-DecisionMemory` from `proposed` to
   `accepted` in `~/.sddk-knowledge/sddk-framework/`.
8. Bump workspace version `v1.146.0 → v1.147.0` and push.

## Status

- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/drafts/ADR-087-DECISION-MEMORY-OBJECT-REF-MODEL.md`
- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionMemory.md`
- Spine: CDD-MEMORY-001 PROPOSED (order 267, depends on CDD-HANDOFF-002
  which is now SHIPPED at v1.146.0)
- Local workspace: clean (commit `9a841f2` on `main`, in sync with
  `origin/main`)
- Binary: `sddk 1.146.0`

## Path forward (post-apply)

CDD-MEMORY-002 (order 268, depends on CDD-MEMORY-001) closes the
traversal / projection half: `log`, `tree`, `show`, `diff`,
`merge-base`, `branch`, `ancestors`, `why`, `reflog`,
`SessionCheckpoint` / `SessionDelta`, decision and delegation
branch projections, and `ContextLease.decision_memory_head`
integration. GRAPH-WHY-001/002 (`why_queries`) are deeper
search/ranking follow-ons that consume the traversable memory.

Each cycle that lands shrinks the remaining H4 territory; the
final CDD-CONTINUE-001 cold-start integration is already SHIPPED
at v1.100.0, so once CDD-MEMORY-002 lands the runtime will
already be able to bind to the new HEAD resolution helper.
