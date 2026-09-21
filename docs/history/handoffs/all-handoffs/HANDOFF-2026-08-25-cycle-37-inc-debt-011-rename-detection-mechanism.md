# HANDOFF — cycle-37 — INC-DEBT-011 Rename Detection Mechanism

**Cycle**: kernel-cycle-37-inc-debt-011-rename-detection-mechanism
**Date**: 2026-08-25
**Status**: ✅ Apply complete (T1-T4 done, T5 in progress)
**Branch**: `feat/kernel-cycle-37-inc-debt-011-rename-detection-mechanism`
**Last SHA**: `76d3431`

## What Was Built

INC-DEBT-011: Rename-detection mechanism per-file frontmatter aliases.

### T1 — DONE (previous agent)
- `feat(cli): parse aliases frontmatter (cycle-37, INC-DEBT-011)` (`adaec72`)
- Extended `ParsedAgent` + `parse_agent_file` to populate `aliases: Vec<String>` from frontmatter

### T2 — DONE (this agent)
- `feat(cli): wire ReconcileContext.renames + scope filter (cycle-37, INC-DEBT-011)` (`e7d2e37`)
- `renames_builder()` function in `reconcile.rs` builds `BTreeMap<String, String>` from alias → canonical
- `ReconcileContext.renames` field added and wired in `run_dev_reconcile`
- `is_framework_namespaced` scope filter applied (S5)
- 3 RED tests: `renames_builder_populates_from_aliases`, `renames_builder_collision_picks_first_alphabetical_with_warning`, `renames_builder_skips_non_sddk_agents`

### T3 — DONE (this agent)
- `feat(cli): activate cycle-36 apply handlers on alias-driven name diffs (cycle-37, INC-DEBT-011)` (`8bee12d`)
- Alias-aware lookup in `reconcile_json`: canonical not found → check aliases → find alias → detect name diff
- Same pattern applied to `reconcile_claude` and `reconcile_codex`
- `apply_rename_in_agents_map` now updates entry's internal `name` field after rename
- 3 RED tests: `apply_rename_json_key_on_alias`, `apply_rename_claude_file_on_alias`, `apply_rename_codex_file_on_alias`

### T4 — DONE (this agent)
- `test(cli): integration — full reconcile renames on disk for alias-driven diff (cycle-37, INC-DEBT-011)` (`76d3431`)
- Integration test: bundle with alias → renames_builder → alias-aware lookup → apply rename → on-disk verification

### T5 — IN PROGRESS (this commit)
- Handoff document (this file)
- INC-DEBT-011 closure
- CHANGELOG entry

## What Remains

### sddk-verify
- Run `cargo test --workspace --locked` (expected: 316 sddk-cli, 128 sddk-engine)
- Run `cargo clippy --workspace --all-targets -- -D errors` (expected: 0 errors)
- Verify 8 new tests pass (316 - 308 = 8)
- Adversarial revert verification for T3 (anti-tautology)

### sddk-archive
- Archive manifest generation
- Release receipt capture
- INC-DEBT-011 debt ledger closure

## Key Technical Decisions

### First-Loaded Alphabetical Wins on Collision (INV-11)
`renames_builder` uses `BTreeMap::or_insert_with` which keeps the first-inserted value on collision. Since `load_agent_sources` sorts agents alphabetically (by name), the first-loaded agent wins. This is deterministic.

### Scope Filter: is_framework_namespaced
Only agents where `is_framework_namespaced(name)` (prefixes: `sddk-`, `sdd-`, `gentle-`) contribute aliases. Non-framework agents' aliases are ignored.

### apply_rename_in_agents_map Entry Name Update
After moving an entry from `old_key` to `new_key`, the entry's internal `name` field is now updated to match `new_key`. This ensures subsequent reconcile runs don't re-detect the name as different.

### Alias-Aware Lookup Pattern
```
1. read_json_existing(config_path, canonical_name) → if found, use it
2. else: for (alias, canonical) in renames where canonical == agent.name:
      read_json_existing(config_path, alias) → if found, use it
3. else: no existing entry
```

## Files Changed

| File | T | Change |
|------|---|--------|
| `crates/sddk-cli/src/dev/editor_adapters/mod.rs` | T1 | Export `renames_builder` |
| `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | T2 | `renames_builder`, `resolve_alias_for`, `ReconcileContext.renames` |
| `crates/sddk-cli/src/dev/reconcile.rs` | T2 | Wire `renames_builder` in `run_dev_reconcile` |
| `crates/sddk-cli/src/dev/editor_adapters/json.rs` | T3 | Alias-aware lookup + entry name update in `apply_rename_in_agents_map` |
| `crates/sddk-cli/src/dev/editor_adapters/claude.rs` | T3 | Alias-aware lookup in `reconcile_claude` |
| `crates/sddk-cli/src/dev/editor_adapters/codex.rs` | T3 | Alias-aware lookup in `reconcile_codex` |
| `crates/sddk-cli/src/dev/tests/reconcile_tests.rs` | T1-T4 | Tests: 1 (T1) + 3 (T2) + 3 (T3) + 1 (T4) = 8 new tests |
| `crates/sddk-cli/src/dev/tests/claude_adapter_tests.rs` | T2 | Add `renames` field to test contexts |
| `crates/sddk-cli/src/dev/tests/codex_adapter_tests.rs` | T2 | Add `renames` field to test contexts |
| `CHANGELOG.md` | T5 | Cycle-37 entry |

## Test Count

- Before cycle-37: 308 tests
- After cycle-37: 316 tests (+8)
  - T1: 1 test (`parse_agent_file_populates_aliases`)
  - T2: 3 tests (`renames_builder_populates_from_aliases`, `renames_builder_collision_picks_first_alphabetical_with_warning`, `renames_builder_skips_non_sddk_agents`)
  - T3: 3 tests (`apply_rename_json_key_on_alias`, `apply_rename_claude_file_on_alias`, `apply_rename_codex_file_on_alias`)
  - T4: 1 test (`reconcile_full_renames_on_disk_for_alias`)

## Invariants Preserved (cycle-32)

- INV-8 (engine untouched): preserved ✅
- INV-9 (no thread leaks): preserved ✅
- INV-10 (no Mutex on workflow state): preserved ✅
- INV-11 (deterministic replay via alphabetical sort): preserved ✅

## Dormant Handlers Activated (cycle-36)

All cycle-36 apply handlers were dormant because alias-driven name diffs were never detected. Now with `ctx.renames` wired:
- `apply_rename_in_agents_map` — active (json)
- `apply_rename_claude_file` — active (claude)
- `apply_rename_codex_file` — active (codex)

## Risks

1. **Adversarial revert not verified in this session**: T3 anti-tautology verification should be done in sddk-verify phase.

2. **`resolve_alias_for` dead code**: The function is defined in `reconcile.rs` but the inline alias lookup pattern is used instead. Could be removed or kept for documentation.

## Notes for Next Agent

- The `resolve_alias_for` function in `reconcile.rs` is currently unused (warning). The inline alias lookup pattern in each adapter is used instead. Consider removing `resolve_alias_for` or using it consistently.

- The T3 tests use a separate config directory to avoid `load_agent_sources` loading alias files as separate agents. This is the correct approach for testing alias-driven rename.

- The `apply_rename_in_agents_map` entry name update fix ensures subsequent reconcile runs don't re-detect the rename. Without this fix, the entry at the new key would have the old name in its `name` field, causing repeated rename detection.

## Commands to Verify

```bash
# Build
cargo build --release -p sddk-cli

# Tests
cargo test -p sddk-cli --lib  # Expected: 316 passed

# Clippy
cargo clippy -p sddk-cli --all-targets -- -D errors  # Expected: 0 errors

# Anti-tautology (in sddk-verify)
git stash push crates/sddk-cli/src/dev/editor_adapters/json.rs
cargo test -p sddk-cli --lib apply_rename_json_key_on_alias 2>&1 | tail -5
# Expected: FAIL
git stash pop
```
