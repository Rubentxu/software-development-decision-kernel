# Handoff — CDD-MEMORY-004 orientation

**Date:** 2026-09-09
**Author:** orchestrator (auto-directive continuation)
**Cycle slug:** `p-63676b11dc0ef88f-cdd-memory-004-reflog-history`
**Status:** ORIENTATION (proposal-ready)
**Prior cycle:** CDD-MEMORY-003 (v1.149.0, SHIPPED `791377b`)

---

## 1. Context

CDD-MEMORY-001..003 built the Decision Memory spine:

- **001 (v1.147.0):** substrate — `DecisionMemoryBlob`/`Tree`/`Commit` +
  `MemoryStore` trait + `InMemoryMemoryStore` + `MemoryRef`/`RefKind` +
  `Reflog` (append-only per single write) + `DecisionMemoryError` (7
  variants). Read-only API.

- **002 (v1.148.0):** traversal — `log`/`show`/`tree`/`diff`/
  `merge_base`/`ancestors`/`why`/`reflog`/`branch`/`fork` + 6 projection
  types. Operational caps (log ≤1024, ancestors ≤1024, tree fan-out
  ≤64/kind). 11 new tests, 14 → 35 cases.

- **003 (v1.149.0):** projection specialization — `ProjectionScope`
  (All / ProjectScoped / CycleScoped), `DecisionProjection`,
  `DelegationProjection`, typed views
  (`SessionCheckpointView` / `SessionDeltaView`). 2 new `MemoryStore`
  ops + `InMemoryMemoryStore` impls. 12 new tests, 35 → 47 cases. WU-4
  (WhyQueryEngine bridge) deferred to v1.149.1 per 800-LoC cap.

All three shipped on `origin/main` (`d6f5581` → `2e7f303`/`ddc8a4d` →
`7574bef`/`bd80275`/`8c93266`/`791377b`). The Decision Memory
substrate is now production-ready for read-only operations and
branch-aware inspection.

---

## 2. What's still missing

Two concrete gaps surface from the audit on 2026-09-09:

### 2.1 Reflog history is single-write only

`MemoryStore::write_ref_with_reflog` appends one `ReflogEntry` per
call. The current `Reflog` value type holds only the latest entry; the
trait surface offers no `Reflog::history` / `reflog_at(seq)` query, and
`InMemoryMemoryStore` does not persist older entries.

**Consequence:** the agent cannot answer questions like "what did
`refs/heads/main` point at 5 commits ago?" or "did anyone rewrite HEAD
between T1 and T2?". This blocks the **R-T11** scenario and weakens
the audit trail promised by 001's append-only invariant.

### 2.2 No mutation surface beyond `branch` and `fork`

002 added `branch` (create) and `fork` (replicate HEAD under
`what-if/`). What's absent:

- `cherry_pick(commit, onto_branch)` — replay a single decision onto a
  different base.
- `revert(commit)` — emit an inverse commit that undoes the named
  commit's tree delta.
- `amend(commit, new_tree)` — replace tree at the most recent commit
  (rewrites HEAD with new content but same parent lineage).
- `reset(ref, target, mode)` — move a ref to `target` with three modes
  (`soft` keeps index/work, `mixed` keeps work, `hard` discards).
- `delete_ref(ref)` — soft-delete a branch ref (preserves its commits).

**Consequence:** the agent cannot repair a mis-staged commit, undo an
exploratory fork, or merge sibling branches without rebuilding the
entire DAG externally. The skeleton `DecisionMemoryCommit::merge`
constructor exists (R-2.7) but is unwired.

---

## 3. Candidate CDD-MEMORY-004

### 3.1 Title

**CDD-MEMORY-004 — Reflog History + Branch Mutation**

### 3.2 Objective

Extend `MemoryStore` with two coherent capability groups:

**A. Reflog history persistence (gap 2.1).**

- Persist the full `Reflog` append history per ref in
  `InMemoryMemoryStore` (in-memory `BTreeMap<String, Vec<ReflogEntry>>`
  with cap = 1024 entries per ref).
- Add `MemoryStore::reflog_history(ref_kind) -> Result<Vec<ReflogEntry>,
  DecisionMemoryError>`.
- Add `MemoryStore::reflog_at(ref_kind, seq) -> Result<ReflogEntry,
  DecisionMemoryError>` (1-indexed into the history).
- Existing `reflog(ref_kind, scope)` semantics MUST keep returning
  the latest entry (backward-compatible).

**B. Mutation ops (gap 2.2).**

- `MemoryStore::cherry_pick(commit_id, onto_ref, message) ->
  Result<MemoryId, DecisionMemoryError>` — replay `commit_id`'s tree
  onto `onto_ref`'s tip; new commit has 2 parents
  (`onto_ref` + `commit_id`).
- `MemoryStore::revert(commit_id) -> Result<MemoryId,
  DecisionMemoryError>` — emit inverse tree under current HEAD; new
  commit has 1 parent.
- `MemoryStore::amend(commit_id, new_tree, message) ->
  Result<MemoryId, DecisionMemoryError>` — produce a new commit with
  same parents as `commit_id` but `new_tree`; new commit becomes
  HEAD.
- `MemoryStore::reset(ref_kind, target, mode) -> Result<()>` —
  move `ref_kind` to `target`. Mode is `Soft`/`Mixed`/`Hard`. Only
  `Hard` is implemented in v1.150.0; `Soft`/`Mixed` return
  `DecisionMemoryError::NotImplemented`.
- `MemoryStore::delete_ref(ref_kind) -> Result<()>` — soft-delete
  branch ref (writes `refs/deleted/<name>` tombstone, removes
  `refs/heads/<name>`).

### 3.3 Exit gate

The agent can:

- Reconstruct a ref's full movement history (`reflog_history`).
- Resolve a ref's state at an arbitrary sequence number (`reflog_at`).
- Cherry-pick a decision onto a sibling branch.
- Revert a previous decision (one-shot inverse tree).
- Amend the most recent commit with a corrected tree.
- Reset a branch ref to an older target (hard mode).
- Delete a branch ref (soft-delete with tombstone).

### 3.4 Scope budget

- ~600 LoC engine (5 new ops + 1 query op + helper state).
- ~400 LoC tests (8 RED cases + integration with 002/003 fixtures).
- 1 ADR (~150 LoC).
- 1 REQ spec (~300 LoC).

Target version: **v1.150.0** (minor bump — adds a new capability
group).

### 3.5 Dependencies / risks

- **Pure-read invariant (R-T1 from 002)** is partially relaxed:
  mutation ops are explicit, named, and atomic. The new invariants
  (R-T22..R-T26) replace R-T1 for the mutation ops only; pure
  reads keep R-T1.
- **`MemoryStore` trait surface** grows from 18 ops to 24 (+33%).
  Default impls stay `unimplemented!()` so external implementors
  opt in.
- **`DecisionMemoryError`** gains two variants: `NotImplemented`
  (for Soft/Mixed reset) and `RefNotEmpty` (for
  `delete_ref` on a branch with unmerged commits — guards
  accidental data loss).

### 3.6 Out of scope (deferred to v1.151.0+)

- `Soft` / `Mixed` reset modes.
- True `merge(commit_a, commit_b)` (3-way merge with conflict
  resolution — needs conflict surface).
- Rebase (replay of a chain onto a new base).
- Garbage collection of unreferenced objects.

---

## 4. Why this is the next spine target

- **Completes the audit story:** without persistent reflog, the
  spine has no way to answer "what did HEAD point at 3 hours ago?".
  This blocks CDD-CONTINUE-001 (already SHIPPED at v1.100.0) from
  recovering an in-flight session that has been amended/reset.
- **Natural next layer:** 001 = substrate, 002 = traversal,
  003 = projection. 004 = mutation. The spine advances by capability
  axis, not by partial re-coverage.
- **Bounded scope:** the 5 mutation ops + 1 history query fit the
  same ~600-LoC budget the prior cycles used. No architectural
  decisions needed.
- **Risk-low:** all 5 ops are well-defined (git analogues), the
  reflog persistence is append-only (extends existing R-2.6), and the
  new error variants are additive (back-compat preserved).

---

## 5. Spine reconciliation needs

`docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`
still shows `CDD-MEMORY-002` as `PROPOSED` even though v1.148.0 + v1.149.0
shipped its exit gate. Should be reconciled to `SHIPPED` before the next
spine cycle starts.

Independent work: add a `CDD-MEMORY-003` and a planned `CDD-MEMORY-004`
entry to the spine.

---

## 6. Next steps

1. Open a propose phase cycle (`p-63676b11dc0ef88f-cdd-memory-004-...`)
   that drafts:
   - `~/.sddk-knowledge/sddk-framework/adrs/drafts/ADR-093-DECISION-MEMORY-REFLOG-HISTORY-AND-MUTATION.md`
   - `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionMemoryMutation.md`
2. Reconcile `EXECUTION-SPINE.yaml` (CDD-MEMORY-002 status, add 003).
3. Land v1.149.1 patch for the deferred WU-4 of CDD-MEMORY-003 if
   budget allows in parallel.
4. Decide: ship CDD-MEMORY-004 as v1.150.0 in one go, or split
   reflog-history (v1.150.0) and mutation-ops (v1.151.0)?

---

## 7. Reference

- v1.147.0 (001): `ddc8a4d`
- v1.148.0 (002): `ddc8a4d`
- v1.149.0 (003): `791377b`
- Archive manifests: `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-cdd-memory-00{1,2,3}-*/archive-manifest.md`
- Specs: `REQ-DecisionMemory.md`, `REQ-DecisionMemoryTraversal.md`,
  `REQ-DecisionProjectionSpecialization.md`
- ADRs: `ADR-087`, `ADR-088`, `ADR-092`
