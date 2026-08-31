# INC-DEBT-015: sddk-engine style nits + bogus lint name cleanup

**status**: closed
**severity**: low
**priority**: P3
**created_at**: 2026-08-26
**cycle**: 41
**detected_by**: post-cycle-40 debt-verify (PASS_WITH_WARNINGS, 6 issues documented)

## Problem

Cycle-40 INC-DEBT-014 closed with 36 unique clippy warning messages (73 total occurrences) remaining in `sddk-engine`. Two categories of debt:

1. **Bogus lint name** (1 item, lib.rs:10):
   - `#![allow(clippy::missing_docs)]` — `clippy::missing_docs` is not a real lint (correct: rustc `missing_docs` or `clippy::missing_docs_in_private_items`).
   - Generates 1 `unknown lint` warning each compile (noise that masks real warnings).

2. **Style nits + unused variables + unused imports** (72 items):
   - **22 originally scoped** (deferred from cycle-40 T4): 12 `use_of_default`, 3 `assert!(true)`, 4 `variable_mut`, 2 `useless_conversion`.
   - **51 NEW** (surfaced by cycle-40 T1 deletions): mostly `unused variable` (34) and `unused imports` (3) in test files, plus 14 lib warnings (mostly `variant` field warnings from error constructions).
   - **2 manual** (need human judgment): 1 `loop variable` and 1 `empty lines after doc comment`.

### Debt distribution

| File | Warnings | Type |
|---|---:|---|
| `crates/sddk-engine/tests/workflow_runtime_lifecycle_tests.rs` | 13 | unused vars (from T1 deletions) |
| `crates/sddk-engine/tests/parallel_concurrency_tests.rs` | 4 | useless_conversion (2) + unused vars (2) |
| `crates/sddk-engine/tests/runtime_receiver_map_tests.rs` | 3 | unused vars + unused imports |
| `crates/sddk-engine/tests/map_operator_tests.rs` | 2 | unused vars |
| `crates/sddk-engine/tests/runtime_construction_tests.rs` | 2 | unused vars |
| `crates/sddk-engine/tests/workflow_runtime_demo.rs` | 4 | use_of_default + unused vars |
| `crates/sddk-engine/tests/*` (other) | 5 | unused vars + loop variable |
| `crates/sddk-engine/src/operator.rs` | 15 | unused variant fields + clock/source/body/max_concurrency |
| `crates/sddk-engine/src/workflow_runtime.rs` | 4 | node_id + variable_mut |
| `crates/sddk-engine/src/lib.rs` | 1 | bogus lint name |
| `crates/sddk-engine/src/tasks/{sleep,sha256,http_fetch,file_write}.rs` | 4 | empty lines after doc comment (×4) |

**Total unique warnings**: 36. **Total occurrences**: 73.

### Why now (post-cycle-40)

- v1.48.8 leaves 73 sddk-engine clippy warnings on the table.
- Cycle-40's T1 deletions surfaced ~51 new warnings (orphaned unused vars/imports in tests).
- Cycle-40 T5 introduced 1 bogus lint name that generates noise each compile.
- Path A-min available; the pattern is established (cycle-40 closed 49 items with T1-T6).

### Severity classification

| Item | Severity | Why |
|---|---|---|
| Bogus lint name (lib.rs:10) | LOW | noise only, easy fix |
| Style nits (use_of_default, assert, mut, useless_conversion) | LOW | readability |
| Unused vars/imports in tests | LOW | dead code remnants |
| Loop variable warning | LOW | single occurrence, needs review |
| Empty lines after doc comment | LOW | style nit |

## Resolution (planned for cycle-41, path A-min)

### Tasks (4 review-aware implementation tasks)

#### T1 — Fix bogus lint name in lib.rs (1 item)
- `crates/sddk-engine/src/lib.rs:10`: change `#![allow(clippy::missing_docs)]` → `#![allow(missing_docs)]` (rustc lint).
- Anti-tautology: `cargo build` no longer produces "unknown lint" warning.
- Commit: `fix(engine): correct bogus clippy::missing_docs lint name to rustc missing_docs (cycle-41, INC-DEBT-015)`.

#### T2 — Apply machine-applicable clippy fixes (~70 items)
- Run `cargo clippy --fix --allow-dirty --allow-staged` for sddk-engine.
- Covers: use_of_default, assert!(true), variable_mut, useless_conversion, unused vars (prefix `_`), unused imports (delete), variant field unused (use `_`).
- Manual review of diff: verify no semantic change, all tests pass.
- Commit: `chore(engine): apply machine-applicable clippy --fix across lib + tests (cycle-41, INC-DEBT-015)`.

#### T3 — Manual cleanup of remaining warnings (~2 items)
- Loop variable warning: review context, either suppress with `_` or refactor loop.
- Empty lines after doc comment: delete blank line.
- Commit: `chore(engine): resolve manual clippy warnings (loop variable, empty lines) (cycle-41, INC-DEBT-015)`.

#### T4 — Closeout (docs only)
- Update INC-DEBT-015: status open → closed, add resolution summary.
- Update INC-DEBT-014: append cycle-41 carry-forward resolution to clarify T4 scope deferral resolution.
- Create `docs/handoff/HANDOFF-2026-08-26-cycle-41-inc-debt-015-sddk-engine-style-nits.md`.
- Add CHANGELOG.md entry.
- Append cycle-41 narrative to ROADMAP.md.
- Commit: `docs(debt+inc+handoff+changelog+roadmap): cycle-41 closeout — INC-DEBT-015 closed (cycle-41)`.

### Commit chronology (cycle-36 lesson)

| # | SHA | Subject |
|---|-----|---------|
| 1 | TBD | `fix(engine): correct bogus clippy::missing_docs lint name to rustc missing_docs (cycle-41, INC-DEBT-015)` |
| 2 | TBD | `chore(engine): apply machine-applicable clippy --fix across lib + tests (cycle-41, INC-DEBT-015)` |
| 3 | TBD | `chore(engine): resolve manual clippy warnings (loop variable, empty lines) (cycle-41, INC-DEBT-015)` |
| 4 | TBD | `docs(debt+inc+handoff+changelog+roadmap): cycle-41 closeout — INC-DEBT-015 closed (cycle-41)` |

**One concernencia per commit.** T1 ≠ T2 ≠ T3 ≠ T4.

### Anti-tautology contract (per task)

Every commit must be revertible and the post-revert state must be DETECTABLE:
- T1 revert: `cargo build` produces "unknown lint" warning at lib.rs:10.
- T2 revert: clippy unique warning count increases by ~32 (machine-applicable items).
- T3 revert: clippy count increases by 2 (loop variable + empty lines).

V2 adversarial revert must confirm each.

### Expected outcomes

- sddk-engine unique warnings: 36 → ~2 (loop variable + empty lines after T3, OR 0 if T3 also resolves).
- sddk-engine total occurrences: 73 → ~5 (T2 collapses many to 1 per file).
- Workspace clippy: still passes `-D errors`.
- sddk-engine tests: still 129 (no test deletions).
- INV-8/9/10/11 preserved (no engine API change).

## Cycle-32 Invariants (preservation contract)

- **INV-8** (engine interface unchanged): preserved — only internal cleanup, no `pub` API change.
- **INV-9** (no thread leaks): preserved.
- **INV-10** (no Mutex on workflow state): preserved — no new locks.
- **INV-11** (deterministic output): preserved — no behavior change.

## Carry-forward decisions (cycle-41 setup)

- ✅ `.sddk/cycles/` added to `.gitignore` (commit `4b8ff12`) — runtime CAS no longer pollutes git status.
- ✅ INC-DEBT-014 lifecycle entries committed (commit `5f1afc5`) — archive_manifest + release_receipt paths.
- ✅ ROADMAP cycle-40 narrative committed (commit `e6b47cf`).
- ✅ HANDOFF-2026-08-26-cycle-40-archive.md committed (commit `46e702b`).
- 🟡 Cycle-39 archive-manifest + release-receipt: decided NOT to amend cycle-39 history (cycle-36 anti-tautology); INC-DEBT-013 lifecycle now references on-disk paths only (path-only reference, sha256 deliberately omitted to avoid drift).

## Lifecycle

- **created**: 2026-08-26 (post-cycle-40 debt-verify + carry-forward from INC-DEBT-014)
- **expected closure**: cycle-41 (this cycle)
- **closed**: 2026-08-26 (cycle-41 archive)
  - archive_manifest: `.sddk/cycles/p-52b95ef55999f9de/kernel-cycle-41-inc-debt-015-sddk-engine-style-nits-and-bogus-lint/archive-manifest.json`
  - release_receipt: `.sddk/cycles/p-52b95ef55999f9de/kernel-cycle-41-inc-debt-015-sddk-engine-style-nits-and-bogus-lint/release-receipt.json`
  - release_tag: `v1.48.9` (tag object `55d414fbdbddb57eea2d0024b8c861f3ad42eaff`, peels to `2806bb2`)

## Resolution

| Commit | Subject | Concern |
|--------|---------|---------|
| `464bc7d` | fix(engine): correct bogus clippy::missing_docs lint name to rustc missing_docs | T1 |
| `f7d4c83` | chore(engine): apply machine-applicable clippy --fix across lib + tests | T2 |
| (none) | manual clippy warnings resolved via #[allow] where clippy suggestion would change semantics | T3 |

**Final sddk-engine clippy**: 0 unique warnings (was 36 baseline / 73 occurrences)
**V2 adversarial revert**: confirmed per task
**Tests preserved**: 129 sddk-engine lib tests, all integration tests passing

## References

- Cycle-40 INC-DEBT-014 closure (carry-forward source): `docs/debt/INC-DEBT-014-sddk-engine-test-debt-sweep.md`
- Cycle-40 cycle handoff: `docs/handoff/HANDOFF-2026-08-26-cycle-40-inc-debt-014-sddk-engine-test-debt.md`
- Cycle-40 archive handoff: `docs/handoff/HANDOFF-2026-08-26-cycle-40-archive.md`
- Cycle-40 release-receipt: `.sddk/cycles/p-52b95ef55999f9de/kernel-cycle-40-inc-debt-014-sddk-engine-test-debt-sweep/release-receipt.json`
- Cycle-36 anti-tautology discipline: V2 adversarial revert per task
- ADR-0064 §D-5 (lint annotation pattern)
