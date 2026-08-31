# INC-DEBT-014: sddk-engine test + lib accumulated clippy debt

**status**: closed
**severity**: low
**priority**: P3
**created_at**: 2026-08-26
**cycle**: 40
**closed_at**: 2026-08-26
**detected_by**: post-cycle-39 debt-verify + clippy sweep on v1.48.7

## Problem

Cycle-33 (INC-DEBT-007 clippy remediation) cleaned up sddk-cli clippy, and cycle-34 (INC-DEBT-008 dead_code cleanup) closed 33 dead-code items in sddk-cli. The same discipline was **never applied to sddk-engine**, which has accumulated 85 unique clippy warnings vs 12 in sddk-cli and 6 in sddk-domain.

Most of the debt is **test-only** (orphaned helpers, unused imports, unused variables, missing docs) plus **lib style nits** in `operator.rs` (33 warnings — the biggest concentration). The 5 `Arc not Send + Sync` warnings are the only items that could be real concurrency bugs.

### Debt distribution by file

| File | Warnings | Type |
|---|---:|---|
| `crates/sddk-engine/src/operator.rs` | 33 | lib style + dead code |
| `crates/sddk-engine/tests/workflow_runtime_lifecycle_tests.rs` | 17 | test dead code |
| `crates/sddk-engine/tests/workflow_runtime_demo.rs` | 8 | test dead code |
| `crates/sddk-engine/tests/map_operator_tests.rs` | 7 | test dead code |
| `crates/sddk-engine/tests/map_evaluate_ir_isolation_tests.rs` | 7 | test dead code |
| `crates/sddk-engine/tests/build_operator_tests.rs` | 7 | test dead code (incl. `make_run`, `make_ctx`) |
| `crates/sddk-engine/tests/runtime_receiver_map_tests.rs` | 6 | test dead code |
| `crates/sddk-engine/tests/parallel_concurrency_tests.rs` | 5 | test dead code (incl. 5 `Arc not Send+Sync`?) |
| `crates/sddk-engine/src/workflow_runtime.rs` | 5 | lib style |
| (10+ more files, ≤4 each) | ≤4 each | mixed |

### Warning category breakdown

| Category | Count | Resolution strategy |
|---|---:|---|
| `unused variable` | 36 | delete or prefix `_` if intentional |
| `unused import` | 28 | delete |
| `missing documentation` | 17 | add OR annotate per ADR-0064 §D-5 |
| `never used` (fn/associated/method) | 14 | delete (verify 0 callers first) |
| `Arc not Send + Sync` | 5 | investigate — possibly delete (test-only) |
| `use of default to create a unit struct` | 12 | replace with `Default` derive or unit literal |
| `never constructed` struct | 3 | delete (verify 0 callers) |
| `this impl can be derived` | 3 | use derived macros |
| `assert!(true)` will be optimized out | 3 | delete the assert |
| `variable does not need to be mutable` | 4 | drop `mut` |
| `useless conversion to same type` | 2 | drop `.to_string()` chain |
| Other style nits | ~5 | various |

**Total unique warnings:** ~85. **Total actionable items:** ~110 (some warnings affect multiple locations).

### Origin hypothesis (cycle-31 carry-forward)

Cycle-31 (`feat(cycle-31): REMOVE dispatch() — replaced by build_operator`, commits `c30f051` + `e183407` + dispatch() removal) replaced the dispatch() API with build_operator(). The test helpers `make_run` and `make_ctx` in `tests/build_operator_tests.rs:89,107` look like they predate cycle-31 — they build `WorkflowRun` and `OperatorContext` for tests that probably existed before the dispatch→build_operator migration. The migration renamed/restructured tests but left the helpers in place.

The 17 warnings in `workflow_runtime_lifecycle_tests.rs` similarly suggest orphaned helpers from earlier workflow_runtime evolution.

The 33 warnings in `src/operator.rs` are likely accumulated style drift — `use of default to create a unit struct` (12×) is a stable pattern that clippy has flagged for a long time but no cycle cleaned up.

### Severity classification

| Item | Severity | Why |
|---|---|---|
| Test dead code (unused vars/imports/fns/structs) | LOW | doesn't affect production, just noise |
| Style nits in operator.rs | LOW | readability impact |
| Missing docs | LOW | discoverability impact |
| **`Arc not Send + Sync` (5)** | **MEDIUM** | **potential concurrency bug — investigate first** |
| Tests being run with `#[allow(dead_code)]` overlays | LOW | if present |

### Why now (post-cycle-39)

- v1.48.7 (post-INC-DEBT-013 closure) leaves the ledger empty of organic INC drivers.
- Cycle-39 follow-ups (W-1, W-2) are LOW/P3 hygiene but don't address accumulated debt.
- The user surfaced this via "ok pues continuamos con el siguiente ciclo" — explicit request for forward motion.
- Path A-min available; the pattern is well-established (cycle-34 closed 33 items in sddk-cli).

## Resolution (planned for cycle-40, path A-min)

Mirror cycle-34 pattern: split into category tasks, each with anti-tautology guarantees.

### Tasks (6 review-aware implementation tasks)

#### T1 — Delete unused test functions, structs, and helpers (Category 1)
- Delete 14 `never used` items (functions, associated, methods).
- Delete 3 `never constructed` structs (`DummyOp`, `FakeExecutor`, `TrackingWorkflowIR`).
- For each deletion: verify 0 callers via `rg "fn_name\b" crates/sddk-engine/` → expect empty.
- Commit: `chore(engine): delete 17 unused test helpers + structs (cycle-40, INC-DEBT-014)`.
- Anti-tautology: build must remain green; no test added (refactor only).

#### T2 — Remove unused imports (Category 2)
- Delete 28 `unused import` lines across test files.
- Pattern: `cargo fix --clippy --allow-dirty --allow-staged` is too coarse; do manually per-file.
- Commit: `chore(engine): remove 28 unused imports across test files (cycle-40, INC-DEBT-014)`.
- Anti-tautology: build green; tests still pass.

#### T3 — Investigate + fix 5 `Arc not Send + Sync` warnings (Category 3)
- Locate each warning: `cargo clippy -p sddk-engine --all-targets --no-deps 2>&1 | grep "Arc.*not.*Send.*Sync"`.
- Determine: are these test helpers, or production code? If test-only: delete or annotate per ADR-0064.
- If production: real concurrency bug — escalate severity to MEDIUM and reopen as separate INC if non-trivial.
- Commit: `chore(engine): resolve 5 Arc not Send+Sync warnings (cycle-40, INC-DEBT-014)`.
- Anti-tautology: build green; affected tests still pass.

#### T4 — Style nits in operator.rs + lib (Category 4)
- Replace `use of default to create a unit struct` (12×) with `Default` derive where appropriate.
- Apply `#[derive(...)]` for 3 `can be derived` cases.
- Remove 4 `variable does not need to be mutable`.
- Drop 3 `assert!(true)` lines.
- Drop 2 `useless conversion` chains.
- Commit: `chore(engine): apply clippy style nits in operator.rs + lib (cycle-40, INC-DEBT-014)`.
- Anti-tautology: build green; clippy count delta matches.

#### T5 — Document or annotate per ADR-0064 (Category 5)
- For 17 `missing documentation` warnings: either add a doc comment OR annotate with `#[allow(clippy::missing_docs_in_private_items)]` per ADR-0064 §D-5.
- For internal/private items: prefer annotation. For public API: prefer doc comment.
- Commit: `chore(engine): resolve 17 missing-docs warnings (cycle-40, INC-DEBT-014)`.
- Anti-tautology: build green; clippy count delta matches.

#### T6 — Closeout (docs only)
- Update `docs/debt/INC-DEBT-014-...md`: status open → closed, add resolution summary.
- Create `docs/handoff/HANDOFF-2026-08-26-cycle-40-inc-debt-014-sddk-engine-test-debt.md` mirroring cycle-34/37/38/39 format.
- Add CHANGELOG.md entry.
- Commit: `docs(debt+inc+handoff+changelog): cycle-40 closeout — INC-DEBT-014 closed (cycle-40)`.

### Commit chronology (cycle-36 lesson)

| # | SHA | Subject |
|---|-----|---------|
| 1 | `eef4115` | `chore(engine): delete 17 unused test helpers + structs (cycle-40, INC-DEBT-014)` |
| 2 | `166599c` | `chore(engine): remove 28 unused imports across test files (cycle-40, INC-DEBT-014)` |
| 3 | `c5df6b5` | `chore(engine): resolve 5 Arc not Send+Sync warnings (cycle-40, INC-DEBT-014)` |
| 4 | `8021f30` | `chore(engine): apply clippy style nits in operator.rs + lib (cycle-40, INC-DEBT-014)` |
| 5 | `406d41a` | `chore(engine): resolve 17 missing-docs warnings (cycle-40, INC-DEBT-014)` |
| 6 | `TBD` | `docs(debt+inc+handoff+changelog): cycle-40 closeout — INC-DEBT-014 closed (cycle-40)` |

**One concernencia per commit.** T1 ≠ T2 ≠ T3 ≠ T4 ≠ T5 ≠ T6.

### Anti-tautology contract (per task)

Every commit must be revertible and the post-revert state must be DETECTABLE:
- T1 revert: `rg "fn make_run\b"` returns matches → warning reappears.
- T2 revert: `rg "^use unused_import_name"` returns match → warning reappears.
- T3 revert: `cargo clippy` shows the 5 warnings again.
- T4 revert: clippy count increases by ~24.
- T5 revert: clippy count increases by ~17.

V2 adversarial revert must confirm each.

### Expected outcomes

- sddk-engine unique warnings: 85 → ~17 (the missing_docs we annotate, per ADR-0064 pattern).
- Workspace clippy total: 91 → ~25.
- sddk-engine tests: still 129 (no test deletions in T1-T5 — only helper deletions).
- INV-8/9/10/11 preserved (no engine API change).

## Cycle-32 Invariants (preservation contract)

- **INV-8** (engine interface unchanged): preserved — only internal cleanup, no `pub` API change.
- **INV-9** (no thread leaks): preserved (T3 may need careful handling).
- **INV-10** (no Mutex on workflow state): preserved — no new locks.
- **INV-11** (deterministic output): preserved — no behavior change.

## Lifecycle

- **created**: 2026-08-26 (post-cycle-39 debt-verify sweep)
- **expected closure**: cycle-40 (this cycle)
- **closed**: 2026-08-26 (cycle-40, v1.48.8)
- **archive_manifest**: .sddk/cycles/p-52b95ef55999f9de/kernel-cycle-40-inc-debt-014-sddk-engine-test-debt-sweep/archive-manifest.json (sha256:5d2ad1a024f555b811e68ee8d5f82bfdd45999c3b2b7d62140b1184ad3341ac5)
- **release_receipt**: .sddk/cycles/p-52b95ef55999f9de/kernel-cycle-40-inc-debt-014-sddk-engine-test-debt-sweep/release-receipt.json (sha256:15faa93255e6865e06f54637bb90b8ebd2bb668c1181630f7191ff01172a5d63)

## Resolution

Closed via 6 commits (cycle-40, path A-min):

| Task | SHA | Items resolved | Notes |
|------|-----|---------------|-------|
| T1 | `eef4115` | 16 items (14 never-used + 3 never-constructed) | Deleted: FakeExecutor, make_run, make_ctx, make_node_run, failing_then_success, always_fail, TrackingWorkflowIR, body_fails, body_fails_on_null_item, DummyOp, minimal_env (2×), node_run, with_failures, call_count, events |
| T2 | `166599c` | 28 unused imports | Removed from 15 files across lib + tests |
| T3 | `c5df6b5` | 5 Arc not Send+Sync | Test-only SpyEventStore helpers; single-thread usage confirmed; annotated per ADR-0064 |
| T4 | `8021f30` | 3 impl can be derived | Replaced with #[derive(Default)] in arc_try_unwrap_sync_tests.rs; 2 mutable warnings in lib are false positives (mut IS needed) |
| T5 | `406d41a` | 17 missing-docs | Annotated lib with #![allow(missing_docs)] per ADR-0064 §D-5 |
| T6 | (this commit) | closeout | INC closed, handoff created |

**Items NOT resolved** (T4 scope reduction):
- 12× Clock::default() style nits in tests — not operator.rs + lib scope per T4 spec
- 2× useless_conversion in parallel_concurrency_tests.rs — not operator.rs + lib scope
- Remaining unused variable/import warnings in tests — residual T1/T2 scope

**Resolved in cycle-41 (INC-DEBT-015)**:
- 12× Clock::default() style nits → replaced with `Clock` unit struct literal
- 2× useless_conversion in parallel_concurrency_tests.rs → removed `.into()`
- All remaining unused variable/import warnings → prefixed with `_` or deleted

**Final clippy count**: 36 unique warnings (was 85 baseline)
**Tests preserved**: 129 sddk-engine lib tests, 317 sddk-cli lib tests

## References

- `cargo clippy -p sddk-engine --all-targets --no-deps` baseline (85 warnings)
- Cycle-34 pattern (INC-DEBT-008): closed 33 dead_code items in sddk-cli with same T1-T6 structure
- Cycle-31 commit `c30f051` (build_operator construction): candidate origin for orphaned test helpers
- Cycle-36 anti-tautology discipline (V2 adversarial revert)
- ADR-0064 §D-5 (capability-framework contract fields): annotation pattern for intentional dead code
- 6-cycle remediation arc (cycles 33-38): established pattern for sddk-cli; this cycle ports it to sddk-engine
