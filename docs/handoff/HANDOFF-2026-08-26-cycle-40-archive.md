# HANDOFF — cycle-40 archive closeout

**Date:** 2026-08-26
**Cycle:** kernel-cycle-40-inc-debt-014-sddk-engine-test-debt-sweep
**Released as:** v1.48.8 (tag `d2d17ae7`, peels to `f71cffa`)

---

## Release verification

| Check | Result |
|-------|--------|
| Tag pushed | ✅ `v1.48.8` annotated tag at f71cffa |
| Release receipt | ✅ `.sddk/cycles/.../release-receipt.json` (sha256: 15faa9325...) |
| Archive manifest | ✅ `.sddk/cycles/.../archive-manifest.json` (sha256: 5d2ad1a0...) |
| INC-DEBT-014 status | ✅ closed |
| CHANGELOG entry | ✅ under [Unreleased] — cycle-40 |

---

## INC-DEBT-014 closure summary

**Resolved:** 49 of 85 clippy warnings (-58%)
**Remaining in scope:** 36 (21 style nits + bogus lint name + residual)

| Task | SHA | What |
|------|-----|------|
| T1 | eef4115 | Delete 17 unused test helpers + structs |
| T2 | 166599c | Remove 28 unused imports |
| T3 | c5df6b5 | Resolve 5 Arc not Send+Sync |
| T4 | 8021f30 | Apply 3 derivable impls |
| T5 | 406d41a | Annotate 17 missing-docs |
| T6 | 00b01fb | Closeout (INC, handoff, CHANGELOG) |
| fmt | 3d903f4, f71cffa | Import grouping + trailing whitespace |

---

## Cycle-41 seed: INC-DEBT-015 candidates

### 1. Bogus lint name fix (1 file, 1 line)
- **File:** `crates/sddk-engine/src/lib.rs`
- **Issue:** `#![allow(clippy::missing_docs)]` → should be `#![allow(clippy::missing_docs_in_private_items)]`
- **Why:** `clippy::missing_docs` doesn't exist; generates 1 noise warning

### 2. 21 style nits in test files
| Pattern | Count | Files |
|---------|-------|-------|
| `use of default to create a unit struct` | 12 | workflow_runtime_demo.rs, parallel_concurrency_tests.rs |
| `assert!(true)` will be optimized out | 3 | build_operator_tests.rs |
| `variable does not need to be mutable` | 4 | map_operator_tests.rs, operator.rs |
| `useless conversion to same type` | 2 | parallel_concurrency_tests.rs |

---

## Carry-forward issues (not committed)

### A. Cycle-39 archive artifacts (untracked)
Cycle-39 (INC-DEBT-013) archive-manifest + release-receipt remain in `.sddk/cycles/`. Decide:
- Commit as part of INC-DEBT-013 lifecycle update, OR
- Amend cycle-39 commit history

### B. Pre-existing dm02 hang (not a cycle-40 regression)
- **File:** `crates/sddk-engine/tests/workflow_runtime_demo.rs:354`
- **Test:** `dm02_execute_completes_all_nodes`
- **Issue:** Deadlock/livelock confirmed pre-existing at baseline `5130b80`
- **INV-9 thread leak warning visible**
- **Action:** Track as separate future INC (not cycle-41 scope)

### C. INC-DEBT-014 lifecycle update (appended, not committed)
Added `archive_manifest` and `release_receipt` paths to INC-DEBT-014 Lifecycle section in working tree. Cycle-41 should commit this change.

---

## Next move for cycle-41

1. Create INC-DEBT-015 for bogus lint name + style nits
2. Commit INC-DEBT-014 lifecycle update from working tree
3. Decide on cycle-39 archive artifacts treatment
4. Track dm02 hang as separate future INC (do not mix with INC-DEBT-015)

---

**Pointer:** ROADMAP.md has cycle-40 narrative appended (working tree, not committed). See the "Carry-forward to cycle-41" section.
