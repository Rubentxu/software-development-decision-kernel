# Handoff — cycle-34 INC-DEBT-008 dead_code cleanup

## Summary

Cycle-34 closes INC-DEBT-008 (carry-forward from cycle-33 FIND-000017). 26 items
landed across 9 files in `crates/sddk-cli/`: 17 deleted (C1) + 9 annotated
with `#[allow(dead_code)]` referencing ADR-0064 §D-4/§D-5 (C2). All cargo gates green.

## Files changed

| File | Change |
|------|--------|
| `crates/sddk-cli/src/dev/editor_adapters/claude.rs` | C1: removed unused import `is_framework_namespaced` |
| `crates/sddk-cli/src/dev/editor_adapters/codex.rs` | C1: removed unused import `is_framework_namespaced` |
| `crates/sddk-cli/src/dev/editor_adapters/json.rs` | C1: removed unused imports `ReconcileTarget`, `is_framework_namespaced` |
| `crates/sddk-cli/src/dev/reconcile.rs` | C1: trimmed 4 names from `pub use` re-export |
| `crates/sddk-cli/src/dev/comments_check.rs` | C1: deleted 5 fields (RulesContract.version, RulesContract.schema, LanguageSpec.block_close, PatternSpec.description, CommentViolation.language); also fixed test that accessed deleted fields |
| `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | C2: annotated 8 items with `#[allow(dead_code)]` per ADR-0064 §D-4/§D-5 |
| `crates/sddk-cli/src/inventory_cycle.rs` | C1: deleted `run_check_ignore` function |
| `crates/sddk-cli/src/dev/check.rs` | Consequence: removed `version` and `schema` fields from RulesContract construction (required by C1 field deletion) |
| `docs/debt/INC-DEBT-008-dead-code-sddk-cli.md` | New: durable debt record for INC-DEBT-008 |

## Commits

| SHA | Subject |
|-----|---------|
| `ba5b633` | chore(cli): cleanup dead_code in sddk-cli — delete 17 + annotate 8 per ADR-0064 (cycle-34) |
| `TBD` | docs(debt+inc): create INC-DEBT-008 + document cycle-34 closure |
| `TBD` | docs(handoff): cycle-34 handoff (cycle-34) |

## Verification

- ✅ `cargo clippy -p sddk-cli --all-targets --no-deps` dead_code warnings in sddk-cli: 0 (C3 `ExistingEntry.name` excluded per proposal scope)
- ✅ `cargo clippy --workspace --all-targets -- -D errors` exits 0 (preserved from cycle-33)
- ✅ `cargo test -p sddk-cli --lib --no-fail-fast` passes (301 tests preserved)
- ✅ `cargo fmt --all -- --check` clean
- ✅ Cycle-32 engine invariants preserved (no engine changes)
- ✅ ADR-0064 §D-4/§D-5 capability-framework contract preserved

## Out of scope

- `ExistingEntry.name` design gap (C3 deferred, separate ticket) — warning remains
- `make_stub_*_dir` test helpers (not in 26 items, warnings remain)
- `check.rs` modification was required consequence of C1 field deletion (not drive-by cleanup)

## Notes

- The proposal's Affected Areas listed 8 files + 1 new debt file. The actual change required 9 files + 1 new debt file because `check.rs` needed updating as a consequence of deleting `RulesContract.version` and `RulesContract.schema` fields.
- The proposal counted 17 C1 deletions + 9 C2 annotations = 26 items. The C2 count was 8 in the proposal table; the 9th item (`AgentReconcileResult.name`) was reclassified from C1 to C2 per proposal Q0 sub-decision.
