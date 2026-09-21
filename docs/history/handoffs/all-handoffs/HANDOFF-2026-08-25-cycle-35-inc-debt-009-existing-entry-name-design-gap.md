# Handoff — cycle-35 INC-DEBT-009 ExistingEntry.name design gap

## Summary

Cycle-35 closes INC-DEBT-009. Added name comparison in `diff_existing_target` + removed `#[allow(dead_code)]` from `ExistingEntry.name` + added RED test. 3 commits on feat branch.

## Files changed

- `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` (impl: name comparison + remove annotation)
- `crates/sddk-cli/src/dev/tests/reconcile_tests.rs` (RED test)
- `docs/debt/INC-DEBT-009-existing-entry-name-design-gap.md` (new, closed)
- `CHANGELOG.md` (cycle-35 entry)
- `docs/handoff/HANDOFF-2026-08-25-cycle-35-inc-debt-009-existing-entry-name-design-gap.md` (this)

## Commits

| SHA | Subject |
|-----|---------|
| `7454ba5` | test(cli): add RED test for diff_existing_target name comparison (cycle-35) |
| `69b6c4f` | feat(cli): wire name comparison in diff_existing_target (cycle-35) |
| `838c9fb` | docs(debt+inc): close INC-DEBT-009 + document cycle-35 (cycle-35) |

## Verification

- ✅ RED test added in commit 1 (initially fails; passes after commit 2)
- ✅ `cargo test -p sddk-cli --lib -- dev::reconcile::reconcile_tests` → 27 tests pass (26 existing + 1 new)
- ✅ `cargo clippy -p sddk-cli --all-targets --no-deps` → 0 dead_code warnings for ExistingEntry.name
- ✅ `cargo clippy --workspace --all-targets -- -D errors` → exit 0
- ✅ `cargo fmt --all -- --check` → clean
- ✅ Cycle-32 engine invariants preserved (no engine changes)
- ✅ INC-DEBT-009 closed

## Out of scope

- Renaming files on disk when name diff is detected (deferred to INC-DEBT-010)
