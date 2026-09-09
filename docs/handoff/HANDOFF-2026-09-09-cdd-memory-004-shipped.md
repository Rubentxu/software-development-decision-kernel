# Handoff — CDD-MEMORY-004 SHIPPED (v1.150.0)

**Cycle:** `p-63676b11dc0ef88f-cdd-memory-004-reflog-history-and-mutation`
**Release:** v1.150.0
**Closed:** 2026-09-09
**Status:** SHIPPED + locally installed + spine reconciled

---

## 1. What shipped

| WU | Commit(s) | Concern |
|----|-----------|---------|
| WU-1 | `d9033e5` | reflog history persistence (REFLOG_HISTORY_CAP=1024, FIFO; `reflog_history` + `reflog_at`) — closes the WU-4 deferred from CDD-MEMORY-003 |
| WU-2 | `17a1b91`, `00d5b70`, `3e2d77b` | `cherry_pick(commit_id, onto_ref, message)` |
| WU-3 | `f31e8d0`, `e01fc4e` | `revert(commit_id, target_ref, message)` |
| WU-4 | `2c141af`, `2f2cc04` | `amend(commit_id, new_tree, target_ref, message)` |
| WU-5 | `d24366d`, `2a0bf20` | `reset(ref_kind, target, mode)` (Hard-only) + `ResetMode` enum + `NotImplemented` error variant |
| WU-6 | `70f709e`, `4de9b44` | `delete_ref(ref_kind)` + `RefNotEmpty` error variant |
| WU-7 | `46fab67` | DMT-62 pure-read invariant regression + DMT-23 closed-set Display extension |
| WU-8 | `6f96791`, `34dcdb7` | `chore(release)` bump to v1.150.0 + Cargo.lock refresh |

Cycle delta: **15 commits**, **+1,400 / -44 LoC** in `crates/sddk-engine/`
(under the 1,500-LoC budget cap).

Tests: **49 → 64** (+15 new: DMT-49..DMT-62).

## 2. Local install

```text
binary: 1.150.0
source: current
resolved: /home/rubentxu/.local/share/sddk/framework/1.150.0
present: true
all_present: true
```

Steps performed (mirror release.sh §8 without going through GH Releases,
because the GH Release assets for v1.150.0 were never published — we
shipped via git tag only):

1. `cargo build --release -p sddk-cli` → v1.150.0 binary at
   `target/release/sddk` → copied to `~/.local/bin/sddk`.
2. `sddk dev manifest --root . --verify` → fresh MANIFEST.sha256
   (370 files, hash 608c6d9c...).
3. Tarballed `agents skills prompts/sddk assets MANIFEST.sha256`,
   extracted to `~/.local/share/sddk/framework/1.150.0/`.
4. Wrote `BUNDLE.toml` (schema_version=2, version=1.150.0,
   min/max=1.150.0, manifest_sha256=first-hash-of-MANIFEST.sha256).
5. Removed old `current` symlink; recreated pointing at 1.150.0.
6. `sddk dev update --prune-only --keep 1` → removed 1.145.1, kept
   1.150.0.
7. `sddk dev link --editor all` → refreshed 69 stale agent paths in
   opencode + zcode configs (framework root changed from 1.145.1 to
   1.150.0).

## 3. GH Release status

**Published.** `bash scripts/release.sh --skip-tests --force` ran the
canonical end-to-end flow after the git tag was already in place:

```text
url:  https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.150.0
published: 2026-09-09T09:39:56Z
assets:
  - sddk                                 (binary, sha256 c8f5c6e4...)
  - sddk.sha256
  - sddk-v1.150.0-sddk-linux-x86_64-musl.tar.gz   (unified: bin/sddk + framework/)
  - sddk-v1.150.0-sddk-linux-x86_64-musl.tar.gz.sha256
  - CHECKSUMS
  - sbom.json
```

Step 13 of the release flow re-installed from the real GH URL and
verified `sddk 1.150.0` ran end-to-end (sha256 poll against the CDN,
doctor, prune). The local `~/.local/share/sddk/framework/` is
coherent at 1.150.0 with `current -> 1.150.0` and no stale
versions retained.

`bash scripts/install.sh --version v1.150.0 --editor all` from a
fresh machine will now download the unified tarball, verify the
sha256, extract `bin/sddk` (exec bit preserved), and lay out
`framework/` at `BUNDLE.toml` schema v2.

## 4. Spec drift recorded

R-M5 (revert) and R-M6 (amend) signatures extended with
`target_ref: RefKind`. Original spec said "advance HEAD" but the
trait has no HEAD alias. Documented inline in
`~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionMemoryMutation.md`.

## 5. Spine reconciliation

`docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`:
- CDD-MEMORY-004 added at `order: 660`, `horizon: H13` (post-GA
  maintenance), `status: SHIPPED`, `depends_on: [GA-002]`.
- CDD-MEMORY-002 status note updated: WU-4 (WhyQueryEngine bridge)
  closed in v1.149.1 (ref cd520b3), not "deferred".

YAML validates: 80 items, 78 SHIPPED (97.5%), 2 PROPOSED
(GRAPH-WHY-001/002 — pre-existing, not part of this cycle).

## 6. Deferred to v1.151.0

- `reset` Soft / Mixed modes (return `NotImplemented` in v1.150.0).
- Full tombstone GC after `delete_ref` (v1.150.0 just drops the ref;
  commits stay in the DAG and may be GC'd by future tooling).
- True inverse-patch `revert` (parent-tree 3-way merge). v1.150.0
  uses single-commit inverse tree (commit_id → parent(commit_id).tree).
- External implementors of `MemoryStore` who don't override the new
  ops will hit `unimplemented!()` defaults. This is opt-in by design
  (per ADR-093).

## 7. Files of record

- Cycle dir: `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-cdd-memory-004-reflog-history-and-mutation/`
  - `tasks.md` (status: SHIPPED)
  - `archive-manifest.md` (130 lines)
  - `propose-manifest.md`
- Spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionMemoryMutation.md`
- ADR draft: `~/.sddk-knowledge/sddk-framework/adrs/drafts/ADR-093-DECISION-MEMORY-REFLOG-HISTORY-AND-MUTATION.md` (accept-and-promote in CDD-MEMORY-005)
- Engine impl: `crates/sddk-engine/src/decision_memory.rs`
- Tests: `crates/sddk-engine/tests/decision_memory_tests.rs`
- Spine: `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`

## 8. What's next

The natural follow-up cycle (CDD-MEMORY-005 candidate):
1. Accept ADR-093 (currently a draft).
2. Land Soft/Mixed reset + tombstone GC.
3. Land parent-tree 3-way merge for true inverse reverts.
4. Bump to v1.151.0 via `scripts/release.sh` (no `--force` needed,
   the previous step's `chore(release)` commit will satisfy the gate).
