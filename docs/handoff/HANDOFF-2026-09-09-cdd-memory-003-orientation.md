# CDD-MEMORY-003 — Projection Specialization & Session Plumbing

> Orientation handoff. Proposes the cycle scope (what to ship) and
> inventory of the v1.148.0 surface we can build on. Not a plan.

**Cycle ID:** `p-63676b11dc0ef88f-cdd-memory-003-projection-specialization` (candidate)
**Spine order:** candidate — natural continuation of CDD-MEMORY-002 (spine 268)
**Trigger:** WU-7 deferred items in `archive-manifest.md` of CDD-MEMORY-002
**Date:** 2026-09-09
**Author:** SDDK orchestrator (cycle CDD-MEMORY-002 closure)

---

## 1. Context — what's already shipped (v1.148.0)

`crates/sddk-engine/src/decision_memory.rs` (after `ddc8a4d`):

| Layer | Symbol | Status |
|-------|--------|--------|
| Object graph | `DecisionMemoryBlob`, `DecisionMemoryTree`, `DecisionMemoryCommit` | shipped (v1.147.0) |
| Refs + authority | `RefKind`, `MemoryRef`, `RefAuthority`, `classify_ref`, `assert_canonical_authority` | shipped (v1.147.0) |
| Reflog | `Reflog`, `ReflogEntry`, `MemoryShow` | shipped (v1.147.0 + 1.148.0) |
| Store trait | `MemoryStore` (18 ops: 8 substrate + 10 traversal/projection) | shipped (v1.147.0 + 1.148.0) |
| In-mem impl | `InMemoryMemoryStore` overrides all 18 ops | shipped (v1.147.0 + 1.148.0) |
| Errors | `DecisionMemoryError` (9 variants: 6 + 3 traversal) | shipped (v1.147.0 + 1.148.0) |
| Projections | `MemoryShow`, `TreeProjection`, `MemoryDiff`, `RefMovement`, `WhyProjection`, `ReflogScope` | shipped (v1.148.0) |
| Writes (mutation) | `branch`, `fork` (only mutations on the substrate) | shipped (v1.148.0) |

**Constraint:** `DecisionMemoryBlob.object_kind: String` (free-form). This is the
lever we'll pull for `SessionCheckpoint` and `decision/*` filtering.

**Constraint:** `MemoryStore::reflog` returns `Ok(Vec::new())` for the in-mem
impl (gap documented in v1.148.0). Real reflog materialization needs a
per-ref aggregator.

**Constraint:** `MemoryStore::why` walks parents directly. ADR-088
documented the intent to delegate to `WhyQueryEngine::decision_why`,
but the integration is not wired — v1.148.0 ships the parent-walking
minimum.

---

## 2. Scope proposal — what this cycle ships

Three concerns, in priority order:

### A. SessionCheckpoint + SessionDelta (typed Blobs, no fourth object class)

Per ADR-088 §2: `SessionCheckpoint` is a `DecisionMemoryBlob` with
`object_kind = "session/checkpoint"`. `SessionDelta` is a `DecisionMemoryBlob`
with `object_kind = "session/delta"`. These ARE NOT new types in the
substrate — they're typed wrappers around the existing `DecisionMemoryBlob`.

Deliverables:
- `pub fn DecisionMemoryBlob::as_session_checkpoint() -> Option<SessionCheckpoint>`
- `pub fn DecisionMemoryBlob::as_session_delta() -> Option<SessionDelta>`
- `pub struct SessionCheckpoint { ... }` — typed view (decoded payload_ref if it parses)
- `pub struct SessionDelta { ... }` — typed view
- Constructor helpers on `DecisionMemoryBlob`:
  `with_session_checkpoint(...)`, `with_session_delta(...)`
- 3-4 tests verifying:
  - Round-trip preserves object_kind and id
  - `as_session_checkpoint()` returns Some only for the right object_kind
  - id is stable across new/from-typed-wrapper
  - payload_ref format is documented

Why this matters: gives downstream consumers (orchestration_synthesis,
continuation_candidate, human_resume_view) a typed handle on session
state without growing the substrate.

### B. `decision/*` and `delegation/*` projections

Per ADR-088 §3: projections over `DecisionMemoryBlob.object_kind` for the
two domains that need them most. Read-only.

Deliverables:
- `pub struct DecisionProjection { /* filter-by-object-kind view */ }`
- `pub struct DelegationProjection { /* filter-by-object-kind view */ }`
- New ops on `MemoryStore`:
  - `fn decision_projection(&self, scope: ProjectionScope) -> Result<DecisionProjection, ...>`
  - `fn delegation_projection(&self, scope: ProjectionScope) -> Result<DelegationProjection, ...>`
- `pub enum ProjectionScope { /* ProjectScoped | CycleScoped | All */ }`
- Default impls on `MemoryStore` (using `unimplemented!()`) so external
  impls are forced to provide them.
- `InMemoryMemoryStore` impls that filter blobs by object_kind prefix.
- 4-6 tests verifying:
  - Projection contains only blobs with `object_kind` in the prefix
  - `ProjectionScope::ProjectScoped(s)` filters by project_id metadata
  - Empty projection returns Ok(empty), not error
  - Unknown commit id → NotFound
  - 64-cap per kind enforced (same as `tree()`)

Why this matters: turns the substrate into a queryable projection
layer for the two highest-volume domains.

### C. Wire `why()` to `WhyQueryEngine` (proper integration)

v1.148.0 `MemoryStore::why` walks parents directly. ADR-088 §1
indicated it should delegate to `WhyQueryEngine::decision_why`. The
delegation requires a bridging shim because `WhyQueryEngine` operates
on `ActiveGraphProjection` (a different namespace).

Deliverables:
- `DecisionMemoryStore::with_why_engine(...)` — configure the engine
- `MemoryStore::why` impl now uses the engine (replacing the parent walk)
- Backward compat: parent-walk output (path + evidence_refs +
  promotions) must remain a subset of the engine's output, so existing
  DMT-30 keeps passing
- 2-3 tests verifying:
  - why(HEAD) returns the same shape with the engine
  - why(HEAD) on a commit with no edges returns path only (engine
    returns empty causal_path)
  - The engine's causal_path is folded into `path` of `WhyProjection`

Why this matters: completes the v1.148.0 deferred item and unlocks
the full why-query semantics (debt-why, decision-why, summary text).

---

## 3. Out of scope (deferred again)

These were also in the WU-7 backlog but are explicitly OUT of this cycle:

1. **`MemoryStore::reflog` real materialization.** Needs a per-ref
   aggregator that captures every `write_ref_with_reflog` call. This
   is a state refactor — push to CDD-MEMORY-004.

2. **ResumeView / ResumeInfo wiring to MemoryStore.**
   `continuation_candidate::ResumeView` and `human_resume_view::ResumeInfo`
   are session snapshots that *could* consume `MemoryStore::why()` /
   `decision_projection()`. But making them do so changes their
   constructors — that's a downstream behavior change. Push to
   CDD-MEMORY-005 (resume plumbing).

3. **CLI surface.** No CLI commands for v1.148.0 ops and none planned
   for v1.149.0 either. CLI is a separate spine item.

---

## 4. Inventory — reusable substrates

| Substrate | Location | Use in cycle |
|-----------|----------|--------------|
| `DecisionMemoryBlob::object_kind` | `decision_memory.rs:41` | discriminator for typed wrappers and projections |
| `MemoryStore` trait | `decision_memory.rs:452` | extend with `decision_projection` / `delegation_projection` |
| `WhyProjection` | `decision_memory.rs:604` | extend `why()` impl |
| `WhyQueryEngine` trait + `DefaultWhyQueryEngine` | `why_queries.rs:106` | delegation target for `MemoryStore::why` |
| `WhyQueryResult` | `why_queries.rs:60-100` | bridge type — needs a conversion to `WhyProjection` |
| `DecisionMemoryTree` `entries: BTreeMap<String, Vec<TreeEntry>>` | `decision_memory.rs:132` | typed view already has 12 buckets; `decision_projection` can reuse the same shape with `object_kind` filter |
| `Reflog` + `ReflogEntry` | `decision_memory.rs:1177,1188` | deferred to CDD-MEMORY-004 |

---

## 5. Risks / open questions

1. **Bridge semantics for `WhyProjection`.** The current `WhyProjection`
   shape (`path`, `evidence_refs`, `promotions`, `admit_failures`)
   differs from `WhyQueryResult.causal_path` + `summary`. Need a
   clear mapping that preserves information. Proposal:
   - `path` = sorted unique MemoryIds from `causal_path` (after
     mapping NodeId → MemoryId where possible; if no mapping, drop
     the entry — but log a marker)
   - `evidence_refs` = NodeIds classified as EvidenceOf edge sources
   - `promotions` = NodeIds classified as Promotes edge sources
   - `summary` → not stored in `WhyProjection` (lossy drop; v1.149.0
     doesn't carry summary text)

   If preserving summary is required, extend `WhyProjection` with
   `pub summary: String`. Decide before the proposal phase.

2. **Projection over `object_kind` vs tree-kind.** The spec says
   `decision/*` and `delegation/*` projections. The substrate uses
   `object_kind` for blob classification. So `decision/foo` matches
   any blob with `object_kind.starts_with("decision/")`. Confirm
   this interpretation with the user (orchestrator) before locking.

3. **Typed wrapper or just constructors?** The proposal suggests
   `SessionCheckpoint`/`SessionDelta` as typed views. Alternative:
   just provide constructor helpers on `DecisionMemoryBlob` and let
   downstream code call `as_session_checkpoint()`. Same effect, less
   code. Decide in propose phase.

4. **Per-cycle size.** v1.148.0 added 1135 lines. A focused v1.149.0
   should stay bounded: ~400-600 lines of code + ~300 lines of tests.
   Three concerns (A, B, C above) push against this bound. If it
   balloons, drop C and ship A+B as v1.149.0, push C to v1.149.1.

---

## 6. Acceptance criteria (proposal-phase inputs)

The propose phase will turn these into formal `R-P-*` requirements
and `S-P-*` scenarios in `REQ-DecisionProjectionSpecialization.md`:

| ID (proposed) | Statement |
|---------------|-----------|
| R-P1 | `SessionCheckpoint` and `SessionDelta` are typed views over `DecisionMemoryBlob`; the substrate has no fourth object class. |
| R-P2 | `as_session_checkpoint()` returns `Some` iff `object_kind == "session/checkpoint"`; same for `SessionDelta` with `"session/delta"`. |
| R-P3 | `DecisionProjection` filters blobs by `object_kind.starts_with("decision/")`. |
| R-P4 | `DelegationProjection` filters blobs by `object_kind.starts_with("delegation/")`. |
| R-P5 | `ProjectionScope::ProjectScoped` narrows by `project_id` of the commit that points at the blob. |
| R-P6 | `why()` delegates to `WhyQueryEngine::decision_why`; the projection shape is preserved. |
| R-P7 | Existing v1.148.0 test suite (35 dmt + 20 syn) still passes. |
| R-P8 | Default `MemoryStore` impls for the new ops use `unimplemented!()`. |

---

## 7. Work-unit plan (tasks-phase input)

| WU | Concern | Approx LoC | Commits |
|----|---------|-----------:|---------|
| WU-1 | SessionCheckpoint / SessionDelta typed views + 4 tests | ~120 | 1 feat + 1 tests |
| WU-2 | DecisionProjection + DelegationProjection types + MemoryStore trait ops | ~150 | 1 feat |
| WU-3 | InMemoryMemoryStore impls for the 2 new ops + 4-6 tests | ~250 | 1 feat + 1 tests |
| WU-4 | WhyQueryEngine delegation in MemoryStore::why + bridge + 2-3 tests | ~150 | 1 feat + 1 tests |
| WU-5 | chore(release): bump to v1.149.0 | ~10 | 1 chore |

Total: 8-9 commits, ~700 lines of code + tests.

If size grows, drop WU-4 → v1.149.1 patch.

---

## 8. Spine ordering rationale

CDD-MEMORY-002 closed the substrate + traversal half of the
decision-memory model. CDD-MEMORY-003 closes the projection +
session half. Together they complete the "Git-like decision memory"
spine. Future spine items:

- **CDD-MEMORY-004** — reflog materialization (state refactor)
- **CDD-MEMORY-005** — resume plumbing (ResumeView / ResumeInfo wiring)
- **CDD-MEMORY-006** — CLI surface for the traversal+projection ops

This sequence keeps each cycle ≤500 LoC and bounded in scope.

---

## 9. References

- v1.148.0 archive: `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-cdd-memory-002-traversal/archive-manifest.md`
- ADR-088: `~/.sddk-knowledge/sddk-framework/adrs/.../ADR-088-DECISION-MEMORY-TRAVERSAL-AND-PROJECTION.md`
- REQ-DecisionMemoryTraversal: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionMemoryTraversal.md`
- WhyQueryEngine: `crates/sddk-engine/src/why_queries.rs`
- ResumeView: `crates/sddk-engine/src/continuation_candidate.rs:95`
- ResumeInfo: `crates/sddk-engine/src/human_resume_view.rs:39`
- ActiveGraphProjection: `crates/sddk-engine/src/active_graph.rs:125`

---

## 10. Conclusion

CDD-MEMORY-003 is the natural follow-up to v1.148.0. It picks up
the deferred items (WU-7) and keeps the projection layer moving
forward. Scope is bounded: 4 work units, ~700 LoC, version v1.149.0.

If user confirms, the next step is the **propose phase**: write
ADR-089, formalize REQ-DecisionProjectionSpecialization, and write
the propose-manifest.md.
