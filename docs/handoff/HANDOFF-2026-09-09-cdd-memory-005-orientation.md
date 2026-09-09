# Handoff — CDD-MEMORY-005 orientation

**Cycle:** `p-63676b11dc0ef88f-cdd-memory-005-mutation-extensions`
**Target release:** v1.151.0
**Started:** 2026-09-09
**Status:** ORIENTATION (this handoff)
**Predecessor:** CDD-MEMORY-004 (v1.150.0, SHIPPED 2026-09-09)

---

## 1. Why this cycle

CDD-MEMORY-004 (v1.150.0) shipped the git-like mutation surface for
the Decision Memory substrate (`cherry_pick`, `revert`, `amend`,
`reset` Hard-only, `delete_ref`) plus persistent reflog history and
two new error variants (`NotImplemented`, `RefNotEmpty`). Three
items were explicitly deferred to v1.151.0 because they were
either semantically deeper (true inverse patches) or required
careful integration (tombstone GC of unreferenced commits).

This orientation handoff surveys each deferred item, estimates its
scope, and proposes a 4-WU breakdown that fits inside a normal
1,500-LoC budget cap (CDD-MEMORY-004 used 1,400 of 1,500; this
cycle should comfortably fit ~1,200 LoC).

## 2. Deferred items inventory

### 2.1 Soft / Mixed `reset` (R-M7 deferred portion)

**Spec source:** `REQ-DecisionMemoryMutation.md` §R-M7 clause 3:

> If `mode == ResetMode::Soft` or `ResetMode::Mixed`: return
> `DecisionMemoryError::NotImplemented { op: "reset", message: "..." }`.

**What it actually means (semantics):**

- **Soft reset** = move the ref AND keep the working tree
  untouched. In git terms: `git reset --soft <target>` moves HEAD
  but leaves the index and working tree alone (the dropped commits
  become staged-but-not-committed).
- **Mixed reset** = move the ref AND reset the index but leave
  the working tree. In git terms: `git reset --mixed <target>`
  moves HEAD, resets the staging area, keeps the working tree.

**Mapping to Decision Memory semantics:**

- There's no `index` or `working tree` in Decision Memory — the
  substrate is content-addressed and immutable. The closest analog
  is the "staged-but-not-replayed" set of dropped commits.
- Soft = move the ref + record the dropped-commit set in the
  reflog history so callers can re-apply it (new `reflog_at` query
  already supports this).
- Mixed = move the ref + record the dropped-commit set (same as
  Soft) but also clear any pending projection-cache entries
  keyed on the dropped commits (no such cache exists in v1.150.0).

**Implementation proposal:**

- Add two new ops on the reset path that don't error on Soft/Mixed.
- Soft: same as Hard + append a new `ReflogEntry { reason: "reset:soft" }`
  with `new_target = old_target` (so the history records what was
  dropped). No DAG mutation, no extra bookkeeping.
- Mixed: identical to Soft for v1.151.0 (no index/working-tree
  concept to distinguish). Mark in the trait doc that Mixed is
  currently a Soft alias pending semantic differentiation.

**Estimated LoC:** ~30 in `decision_memory.rs` + 2-3 tests (~60
LoC tests). Total: ~90 LoC.

### 2.2 Tombstone GC for `delete_ref` (R-M8 deferred portion)

**Spec source:** `REQ-DecisionMemoryMutation.md` §R-M8 prose after
clause 3:

> For v1.150.0 the tombstone is just an empty marker; full GC of
> unreferenced commits is deferred to v1.151.0.

**What it means:**

After `delete_ref` removes a branch, the commits unique to that
branch stay in the `commits` BTreeMap even though they're now
unreachable from any ref. For an in-memory store this is benign
(it'll be GC'd when the store drops). But:

- The reflog history still records the deleted branch's path,
  so the commits are still "reachable" through the history alone
  (intentional — the history is part of the substrate).
- The current code refuses delete via `RefNotEmpty` if unique
  commits would be lost. To honor the spec, after delete we
  should be able to physically drop those unique commits from
  `commits` and the corresponding `trees` and `blobs`.

**Implementation proposal:**

- After `g.remove(&path)` in `delete_ref`, scan the now-unique
  commits and remove their entries from `commits`, `trees`,
  `blobs` BTreeMaps.
- Walk each unique commit's tree (BFS over `TreeEntry.id`) and
  collect blob ids to remove; walk each tree's sub-entries
  recursively to collect deeper tree ids.
- Skip any id that appears in another commit, tree, or blob
  (defensive — should be impossible after a correct `delete_ref`,
  but defensive check is cheap).
- Tombstone record: keep the `delete` reflog history entry, but
  add a new field `tombstone_for: Vec<MemoryId>` (the dropped
  commit ids) so the history alone is sufficient to reconstruct
  what was lost.

**Estimated LoC:** ~80 in `decision_memory.rs` (recursive tree
walker + BTreeMap mutations) + 2-3 tests (~70 LoC). Total: ~150
LoC.

### 2.3 True inverse-patch `revert` (R-M5 deferred portion)

**Spec source:** `REQ-DecisionMemoryMutation.md` §R-M5 prose
after clause 3:

> The revert is a **single-commit inverse tree**, not a true
> inverse patch. Callers needing byte-level reverts must re-apply
> the inverse tree on top of the current HEAD themselves.

**What it means:**

The v1.150.0 `revert` builds a new commit with `tree = parent_tree`
(parent of the reverted commit's tree). That's a coarse-grained
snapshot rollback — it loses any later changes on the parent line.
A true revert preserves subsequent work.

**True revert semantics in Decision Memory:**

A 3-way merge of:
- base = merge_base(current_HEAD, commit_id) — the LCA commit
- ours = current_HEAD tree
- theirs = commit_id tree

The merged tree preserves changes in `ours` that don't conflict
with `commit_id`, and reverts the change `commit_id` introduced
relative to `base`. Result: a new commit with 2 parents
`[current_HEAD, commit_id]` and `tree = merged`.

The substrate doesn't have a generic 3-way merge primitive yet.
The minimal correct approach: per-tree-entry 3-way merge where
each entry is a `(name, blob_id)` triple. If `ours` and `theirs`
agree on a name, keep it. If they disagree, pick ours (caller
can re-apply theirs manually). If `base` had it and neither ours
nor theirs does, drop it. If neither base nor ours has it but
theirs does, take theirs.

This is a heuristic 3-way merge, not a true content-merge. It's
good enough for the Decision Memory use case because each tree
entry is itself content-addressed (changing the entry means a
new blob_id), so conflicts are crisp: same name with different
ids = conflict.

**Implementation proposal:**

- Add a helper `merge_trees(base, ours, theirs) -> MemoryId` on
  `InMemoryMemoryStore` (or as a free function in the same module).
- Update `revert` to call `merge_trees(merge_base(...), HEAD_tree, commit_id_tree)`
  and use the result as the new commit's tree.
- merge_base already exists; HEAD resolution needs a new
  `resolve_head()` helper (or pass HEAD via `target_ref`).
- The new revert is `cherry_pick`-shaped: 2 parents
  `[target_ref_tip, commit_id]`, but the tree is the 3-way merge
  result, not commit_id.tree verbatim.

**Estimated LoC:** ~120 in `decision_memory.rs` (merge helper +
revert update) + 3-4 tests (~90 LoC). Total: ~210 LoC.

### 2.4 ADR-093 acceptance

**Current status:** `~/.sddk-knowledge/sddk-framework/adrs/drafts/ADR-093-DECISION-MEMORY-REFLOG-HISTORY-AND-MUTATION.md`
(draft, 229 lines, 6 sections).

**What acceptance means:**

Per the SDDK ADR lifecycle, a draft ADR gets:
1. Renamed to drop the `drafts/` path → `adrs/ADR-093-...md`.
2. `## Status` field updated: `Draft` → `Accepted (v1.150.0)`.
3. New `## Consequences` section listing what shipped, what was
   deferred, and where the deferred items live (this cycle).
4. Cross-link to `REQ-DecisionMemoryMutation.md` (the spec it
   embodies) and to the archive manifest.

**Estimated LoC:** ~50 in the ADR markdown only. No code change.

## 3. Total scope estimate

| WU | Concern | LoC |
|----|---------|-----|
| WU-1 | Soft / Mixed reset (~90 LoC) + tests | ~150 |
| WU-2 | Tombstone GC (~150 LoC) + tests | ~220 |
| WU-3 | True inverse-patch revert (~210 LoC) + tests | ~300 |
| WU-4 | ADR-093 acceptance (markdown only) | ~50 |
| WU-5 | Pre-release gate: bump v1.151.0 + push + tag + archive | (no code) |

Total: ~720 LoC of code, ~270 LoC of tests, ~50 LoC of ADR
markdown. Well under the 1,500-LoC budget cap.

## 4. Risks and preconditions

- **3-way merge semantics:** the heuristic above is a simplification.
  If a downstream consumer expects true content-level merge (e.g.
  merging two divergent decision branches), they'll be surprised.
  Mitigation: document the merge strategy in the trait docstring
  and in the REQ. Provide a follow-up cycle if needed.
- **Tombstone GC correctness:** removing commits from `commits`/
  `trees`/`blobs` must be transactional with the ref removal.
  Risk: if delete_ref errors partway through, we leave dangling
  data. Mitigation: keep the current two-phase (compute unique set,
  remove refs + commits + trees + blobs) but verify with a
  defensive "skip if any other ref sees it" check at each id.
- **HEAD resolution:** trait has no `Head` alias. The true-invert
  revert needs to know the current HEAD tip. Resolution: caller
  passes `target_ref: RefKind` (already the v1.150.0 sig) AND the
  trait resolves the tip via `resolve_ref`. No trait change.

## 5. Acceptance gates

Per AGENTS.md §3 (verify profile) and §8 (release flow):

- All v1.147.0..v1.150.0 tests still pass (R-M10 backward compat).
- New DMT-63..DMT-68 cover the three deferred items.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo build --release -p sddk-cli`.
- `scripts/release.sh` runs end-to-end without `--force` (because
  we'll add the `chore(release)` bump as part of WU-5).
- GH Release `v1.151.0` published; install round-trip verified.

## 6. Recommended cycle ordering

1. **WU-1 Soft/Mixed reset** (smallest, lowest risk).
2. **WU-2 Tombstone GC** (mid-size, no semantic change).
3. **WU-3 True inverse-patch revert** (largest, semantic change;
   do last so the merge helper can be reasoned about in isolation).
4. **WU-4 ADR-093 acceptance** (no code; can be done anytime after
   WU-3 lands).
5. **WU-5 Release bump** (only after WU-1..WU-4 pass).

The 4 WUs naturally split into 3 impl+test commits + 1 docs
commit + 1 release bump commit, matching the AGENTS.md §2.1
one-concern-per-commit discipline. Estimated ~10-12 commits total.

## 7. Open questions for the user

1. Should `Mixed` be a Soft alias for v1.151.0 or should we add
   a no-op distinguishing behavior (e.g. logging)? Recommendation:
   alias for now, defer distinction to v1.152.0 when there's a
   real index/working-tree concept to differentiate.
2. Should tombstone GC be opt-in (e.g. `delete_ref` gains a
   `gc: bool` parameter) or always-on? Recommendation: always-on
   for v1.151.0 — the spec already commits to it. Add the flag
   in v1.152.0 if users want to preserve unreachable commits for
   forensics.
3. Should ADR-093 acceptance also include a backfill of any
   missed design rationale (e.g. why `reflog_history` cap is
   1024 and not 256 or 4096)? Recommendation: yes — the current
   draft omits this. Add a "Capacity choices" subsection.

## 8. Files of record (this cycle)

- Cycle dir: `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-cdd-memory-005-mutation-extensions/`
  - `HANDOFF-2026-09-09-cdd-memory-005-orientation.md` (this file)
  - `propose-manifest.md` (next phase)
  - `REQ-DecisionMemoryMutationPlus.md` (or three separate REQ files;
    one per item)
  - `tasks.md` (WU breakdown)
  - `archive-manifest.md` (after release)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-093-...md` (after acceptance)
- Engine impl: `crates/sddk-engine/src/decision_memory.rs`
- Tests: `crates/sddk-engine/tests/decision_memory_tests.rs`
- Mirror of this handoff: `docs/handoff/HANDOFF-2026-09-09-cdd-memory-005-orientation.md`
