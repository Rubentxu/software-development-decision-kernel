# INC-DEBT-011: Rename-detection mechanism (per-file frontmatter aliases)

**status**: closed
**severity**: medium
**priority**: P2
**created_at**: 2026-08-13
**closed_at**: 2026-08-25
**cycle**: 37

## Problem

When a bundle agent has an alias (e.g., `sddk-a` with `aliases: [a]`), but the user's IDE config uses the alias entry (e.g., opencode.json has `"a": {...}` instead of `"sddk-a": {...}`), the reconcile system would not detect this as a name diff. The config entry would be treated as a new agent or cause incorrect behavior, because the lookup by canonical name `"sddk-a"` would return `None`.

This is the dormant-handler problem from cycle-36: the apply handlers (`apply_rename_in_agents_map`, `apply_rename_claude_file`, `apply_rename_codex_file`) existed but were never triggered because alias-driven name diffs were never detected.

## Resolution

Cycle-37 implemented per-file frontmatter `aliases:` parsing + `renames_builder` + alias-aware lookup in all 3 reconcile adapters:

1. **`aliases:` frontmatter parsing** (T1): `load_agent_sources` now parses `aliases: [a, b]` or `aliases: a` from agent markdown frontmatter, populating `AgentSource.aliases: Option<Vec<String>>`.

2. **`renames_builder()`** (T2): Builds a `BTreeMap<String, String>` from alias → canonical name, scope-filtered to `is_framework_namespaced` agents. First-loaded alphabetical wins on collision (INV-11).

3. **`ReconcileContext.renames`** (T2): The renames map is passed to all adapters via `ReconcileContext.renames: &BTreeMap<String, String>`.

4. **Alias-aware lookup** (T3): Each adapter's reconcile loop now does:
   - Try canonical name lookup → if found, use it
   - Else, check `ctx.renames` for alias mapping to this canonical → if found, use alias entry
   - Else, no existing entry

5. **Entry name update fix** (T3): `apply_rename_in_agents_map` now updates the entry's internal `name` field to match the new map key after rename. This prevents subsequent reconcile runs from re-detecting the name as different.

6. **Apply handler activation** (T3): When a name diff is detected (existing entry has different name than target), the cycle-36 apply handlers are triggered, performing the actual rename.

## Evidence

- 1 branch: `feat/kernel-cycle-37-inc-debt-011-rename-detection-mechanism`
- 5 commits: T1 (parser), T2 (renames builder + scope filter), T3 (apply activation), T4 (integration), T5 (closeout)
- Test count delta: +8 (308 → 316)
  - T1: 1 test
  - T2: 3 tests
  - T3: 3 tests
  - T4: 1 test

## Cycle-32 Invariants Preserved

- **INV-8** (engine untouched): preserved ✅ — no changes to `sddk-engine`
- **INV-9** (no thread leaks): preserved ✅
- **INV-10** (no Mutex on workflow state): preserved ✅
- **INV-11** (deterministic replay via alphabetical sort): preserved ✅ — `BTreeMap` gives deterministic iteration; first-loaded alphabetical wins on collision

## Lifecycle

- **created**: 2026-08-13 (cycle-37 explore)
- **discovered**: cycle-37 explore (alias-driven diffs dormant without detection mechanism)
- **remediated**: cycle-37 apply T1-T5
- **verified**: pending (cycle-37 sddk-verify — confirm in this PR)
- **archived**: pending (cycle-37 sddk-archive)

## Technical Notes

### First-Loaded Alphabetical Wins on Collision (INV-11)
`renames_builder` uses `BTreeMap::or_insert_with` which keeps the first-inserted value on collision. Since `load_agent_sources` sorts agents alphabetically before iterating, the first-loaded (alphabetically earliest) agent wins. This is deterministic.

### Scope Filter: is_framework_namespaced
Only agents where `is_framework_namespaced(name)` contribute aliases. Currently: prefixes `sddk-`, `sdd-`, `gentle-`. This prevents user agents' aliases from being considered.

### Entry Name Update Fix
The fix in `apply_rename_in_agents_map` ensures the entry's internal `name` field is updated to match the new map key after rename. Without this fix, subsequent reconcile runs would detect the same rename again (entry at new key has old name in `name` field).

## Files Changed

| File | Change |
|------|--------|
| `crates/sddk-cli/src/dev/editor_adapters/mod.rs` | Export `renames_builder` |
| `crates/sddk-cli/src/dev/editor_adapters/reconcile.rs` | `renames_builder()`, `resolve_alias_for()`, `ReconcileContext.renames` |
| `crates/sddk-cli/src/dev/reconcile.rs` | Wire `renames_builder` in `run_dev_reconcile` |
| `crates/sddk-cli/src/dev/editor_adapters/json.rs` | Alias-aware lookup + entry name update in `apply_rename_in_agents_map` |
| `crates/sddk-cli/src/dev/editor_adapters/claude.rs` | Alias-aware lookup in `reconcile_claude` |
| `crates/sddk-cli/src/dev/editor_adapters/codex.rs` | Alias-aware lookup in `reconcile_codex` |
| `crates/sddk-cli/src/dev/tests/reconcile_tests.rs` | 8 new tests |
| `crates/sddk-cli/src/dev/tests/claude_adapter_tests.rs` | Add `renames` field to test contexts |
| `crates/sddk-cli/src/dev/tests/codex_adapter_tests.rs` | Add `renames` field to test contexts |
| `CHANGELOG.md` | Cycle-37 entry |
