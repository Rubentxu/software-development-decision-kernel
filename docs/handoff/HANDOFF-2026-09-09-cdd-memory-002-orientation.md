# CDD-MEMORY-002 — Proposal-ready orientation (handoff)

> **Status:** proposal-ready (orientation). Cycle work itself lives
> in the next session / phase that picks this handoff up.
> **Triggered by:** v1.147.1 patch closure of CDD-MEMORY-001
> substrate (HEAD `a8defda` on `origin/main`).
> **Spine slot:** order 268, H4 — CDD Handoff.
> **Depends on:** CDD-MEMORY-001 (SHIPPED at v1.147.0 + patched
> at v1.147.1).
> **Next-dependency:** CDD-CONTINUE-001 (order 269, SHIPPED at
> v1.100.0) and the broader Cycle-Resume narrative.
> **References:**
>   - `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml:345-356` (spine row)
>   - `docs/sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md:220` (objectives)
>   - `docs/sddk-decision-kernel-architecture/02-roadmap/DECISION-MEMORY-GIT-MODEL.md` (canonical design)
>   - `~/.sddk-knowledge/sddk-framework/adrs/drafts/ADR-087-DECISION-MEMORY-OBJECT-REF-MODEL.md` (parent ADR)
>   - `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionMemory.md` (substrate contract)

## 1. Context and scope

CDD-MEMORY-001 closed the **substrate half** of the Decision Memory
Git-like model: blobs, trees, commits, refs, canonical HEAD, tags,
append-only reflog, advisory-branch invariant, and the
`HashMismatch`/`AuthorityRejected` invariants. The implementation is
`crates/sddk-engine/src/decision_memory.rs` (~870 lines + 24 RED->GREEN
tests in `crates/sddk-engine/tests/decision_memory_tests.rs`).

CDD-MEMORY-002 closes the **traversal + projection half**. The substrate
already answers "what content exists" but does not yet answer "how do
agents/delegators/users navigate that content". Without traversal the
substrate is a tomb of immutable bytes; with it, the DAG becomes
session-resumable, why-explainable, and forkable.

## 2. Exit gate (verbatim from `EXECUTION-SPINE.yaml:352`)

> Agent/user can `log`/`show`/`tree`/`diff` ancestry, resolve
> historical session checkpoints, compute `merge-base`/`fork-point`
> and inspect `decision/`/`delegation` branches back to original
> evidence.

Eight acceptance criteria from `DECISION-MEMORY-GIT-MODEL.md:243-258`
expand this gate:

1. resolve canonical HEAD
2. render a bounded resume tree
3. diff it with an earlier session/checkpoint
4. traverse why/decision branches to original evidence
5. inspect rejected alternatives/revisit triggers
6. recover delegated contributions and synthesis receipts
7. propose next candidates with pros/cons
8. fork a what-if branch without mutating canonical state

## 3. Existing reusable artefacts (do not rewrite)

| Surface | Location | Notes |
|---------|----------|-------|
| `ActiveGraphProjection` | `crates/sddk-engine/src/active_graph.rs` (588 lines) | typed node/edge DAG view; already understands Decision Memory refs + Lab promotions. Splendid substrate for the cross-substrate view. |
| `WhyQueryEngine` (`decision_why`, `debt_why`, `why`) | `crates/sddk-engine/src/why_queries.rs` | parent-chain traversal w/ deterministic replay. Covers S-1..S-4 of REQ-WhyQueries. |
| `AgentContributionEnvelope` + `ContextLease` | `agent_contribution_envelope.rs` | envelopes already carry `decision_memory_head`; this surface is what CDD-MEMORY-002 anchors on. |
| `OrchestrationSynthesisReceipt` | `orchestration_synthesis.rs` | anchors the **delegation projection** (git-model §7 §9). |
| `ContinuationCandidate` + `ResumeView` | `continuation_candidate.rs`, cockpit | already SHIPPED; CDD-MEMORY-002 must serve these views from memory refs. |
| Substrate objects | `decision_memory.rs` | `DecisionMemoryBlob`, `DecisionMemoryTree`, `DecisionMemoryCommit`, `MemoryStore`, `MemoryRef`/`RefKind`, `Reflog`, `MemoryId`, `DecisionMemoryError` — all stable at v1.147.1. |

## 4. Scope of CDD-MEMORY-002 (the actual delta)

### 4.1. Nine semantic operations on `MemoryStore`

Add to the `MemoryStore` trait (extend, do not break v1.147.1):

```rust
fn log(&self, from: MemoryId, max: usize) -> Result<Vec<MemoryId>>;
fn show(&self, id: MemoryId) -> Result<MemoryShow>;
fn tree(&self, id: MemoryId) -> Result<TreeProjection>;
fn diff(&self, a: MemoryId, b: MemoryId) -> Result<MemoryDiff>;
fn merge_base(&self, a: MemoryId, b: MemoryId) -> Result<MemoryId>;
fn ancestors(&self, id: MemoryId, max_depth: usize) -> Result<Vec<Vec<MemoryId>>>;
fn why(&self, ref_id: RefKind) -> Result<WhyProjection>;          // delegated to WhyQueryEngine
fn reflog(&self, scope: ReflogScope, max: usize) -> Result<Vec<ReflogEntry>>;
fn branch(&self, name: &str, target: MemoryId) -> Result<()>;
fn fork(&self, as_name: &str) -> Result<()>;
```

`MemoryShow`, `TreeProjection`, `MemoryDiff`, `WhyProjection` are
POD-value projection structs that callers (CLI, Context Compiler,
ResumeView) consume. All operations are pure: no mutation of the
content-addressed object graph; only ref + reflog mutations.

### 4.2. SessionCheckpoint + SessionDelta

A new `SessionCheckpoint` IS a ref (not a new object class). It is a
named tag binding a canonical ref + a recorded set of subject /
planning / workflow / knowledge revisions at that moment in time.
This lets `diff(session_A, session_B)` work even when the lineage
graph between the two spans hundreds of plain commits.

```rust
struct SessionCheckpoint {
    tag: RefKind::Tag,
    at_commit: MemoryId,
    captured_subjects: RevisionSet,  // { subject, planning, workflow, knowledge }
    captured_at_ms: i64,
    author: DecisionMemoryAuthor,
}
struct SessionDelta {
    added: Vec<DecisionMemoryCommit>,
    mutated_refs: Vec<RefMovement>,
    subject_drift: Vec<RevisionDrift>,
    knowledge_drift: Vec<KnowledgeDrift>,
}
```

Store them as blobs addressed by SHA-256, like any other content.

### 4.3. Decision projection (`decision/<decision-id>/<option>`)

Mirrors git-model §7 Decision projection: returns a typed view of
Question / Options / Evidence / Outcome / Rejected-reason /
Revisit-trigger. Build on top of `MemoryStore::tree(commit_id)` +
the typed blob indices the substrate already has.

### 4.4. Delegation projection (`delegation/<run-id>`)

Mirrors git-model §7 Delegation projection: orchestrator tree with
contributions + synthesis + consumed/omitted/conflicts/dissent.
Build on top of `OrchestrationSynthesisReceipt` + the EnvelopeStore
cross-substrate seam we now have.

### 4.5. CLI + Context Compiler binding

- Expose the 12 logical operations as a stable Rust surface
  (`sddk_memory_*`) usable from `sddk-cli` and from the Context
  Compiler. CLI flags TBD per session; canonical surface is the
  Rust API.
- Wire `ResumeView` / `ContinuationCandidate` to consume
  `MemoryStore::tree(HEAD)` instead of the ad-hoc
  `ContinuationCandidateFrontier` they currently use. This is the
  H4-closing integration that CDD-CONTINUE-001 (already SHIPPED)
  was waiting for.

### 4.6. Determinism

Every traversal must be **deterministic**: same input state + same
`at_ms` ⇒ byte-identical output. Recorded-at timestamps are
caller-supplied (per `REQ-ActiveGraphProjection S-6`). `merge_base`
returns the lowest common ancestor in topological order; ties broken
deterministically by commit hash.

## 5. Out of scope (explicitly)

Per git-model §10 and the parent ADR's deferred sections:

- LLM counterfactual search (Pareto / beam / MCTS / LATS).
- Anything that mutates canonical HEAD (workflow lab work — separate
  cycle).
- Auto-promotion of `what-if/*` to `canonical` (policy work).
- Cross-project ref sync (each project has its own memory store by
  design).

## 6. Risks and seams

- **Linear time** on `ancestors`/`diff` if the DAG grows past
  ~10 000 commits. Mitigate with `max_depth` + paginated log,
  and document the cap.
- **`merge_base` on cycles** is impossible — the substrate is
  acyclic by construction (§11 invariants) but defensive error
  message required.
- **reflog contention** with concurrent writes: keep the lock
  ordering consistent (object graph → ref namespace → reflog) to
  avoid the join/deadlock class.
- **Cross-substrate synthesis traversal** (REQ-ActiveGraphProjection
  + REQ-DecisionMemory combination) must remain a pure read — no
  Coordinator writes back into memory during a traversal pass.

## 7. Proposed fan-out (A-lite, smaller than A-full)

1. **`sddk-spec`** (MiniMax-M3) → write
   `REQ-DecisionMemoryTraversal.md` with RFC 2119 scenarios
   DMT-T-01..T-12 covering all 9 operations + projections +
   Session/Decision/Delegation projections + 8-point acceptance.
2. **`sddk-design`** (MiniMax-M3) → design the projection struct
   hierarchy, the stored-blob shape for SessionCheckpoint, and the
   Context Compiler integration seam. Sketched architecture decision:
   store SessionCheckpoint as a Blob (not a new object kind) so
   storage/code stays uniform.
3. **`sddk-tasks`** → decompose into apply work units.
4. **`sddk-apply`** (MiniMax-M2.7-highspeed) → implement in
   `crates/sddk-engine/src/decision_memory_traversal.rs` + tests
   in `crates/sddk-engine/tests/decision_memory_traversal_tests.rs`.
5. **`sddk-verify`** + **`sddk-debt-verify`** → full profile (per
   AGENTS.md §3) and parallel debt lens fan-out.
6. **`sddk-release`** → bump version (target v1.148.0 since this
   is feature-additive) and push.
7. **`sddk-archive`** → write
   `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-cdd-memory-002-traversal/`

## 8. Success criteria for the orientation phase

- [x] Spine row + backlog + crosswalk + parent ADR confirmed.
- [x] Existing reusable artefacts (active_graph, why_queries,
      orchestration_synthesis, continuation_candidate) inventoried.
- [x] DECISION-MEMORY-GIT-MODEL.md sections 6 (operations),
      7 (projections), 8 (session continuity), 11 (invariants)
      and 12 (acceptance) read end-to-end and inlined as a
      contract reference.
- [x] Substrate (v1.147.1) confirmed as a stable foundation
      (canonical_payload JSON-escape bug fix already shipped,
      verify_id on Tree/Commit already shipped).

## 9. Open questions for the propose phase

1. Do we need an exact GIT-style porcelain vs plumbing split, or a
   single trait surface? Suggest single surface + thin CLI wrappers.
2. How do we paginate `log` and `reflog` for memory stores past
   10 000 entries — cursor / token / window?
3. Does `decision/<decision-id>/<option>` projection need write
   capability (Record of Decision), or read-only? Git-model is
   silent — propose read-only for v1.148.0; write tracked in
   CDD-MEMORY-003 (next).
4. Do we expose `merge_base` for cycles (returning the join-point
   in the past), or fail closed on discovery of a cycle?
5. Should `Branch` projection include a `cycle_progress:` summary
   (intake / deliberation / decision / synthesis) or leave cycle
   semantics to the higher-level `CycleNarrative` module?

## 10. Next action

Pick this handoff up in the next session/phase. Suggested first
command: launch `sddk-spec` with `cdd-memory-002` as the cycle id
and the spec landscape mapping in §3 plus the git-model excerpts
in §4 as anchor content.
