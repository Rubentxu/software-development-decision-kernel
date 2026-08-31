# INC-DEBT-012: Cycle-37 follow-up — dead-code cleanup

**status**: closed
**severity**: low
**priority**: P3
**created_at**: 2026-08-25
**closed_at**: 2026-08-25
**cycle**: 38

## Problem

Cycle-37 left 2 dead-code findings that were identified but not resolved:

- **W1**: `resolve_alias_for` was extracted in T2 (reconcile.rs:258-281) but was never wired to the 3 adapter call sites. The function existed as dead code, generating a clippy `function is never used` warning.
- **F1**: `ParsedAgentForTest` (mod.rs:280-285) had 3 fields never read in tests: `description`, `tools`, `body`. Only `aliases` was ever used by the cycle-37 parser tests.

## Resolution

Cycle-38 T1–T3 wired the helper, trimmed dead fields, and added an anti-tautology direct test:

1. **T1 (refactor)**: Replaced inline `or_else` alias-lookup pattern in 3 adapters (json/claude/codex) with calls to `resolve_alias_for(ctx.renames, &agent.name, |n| read_*(..., n)).map(|(e, _)| e)`. The helper now has 3 callers and is no longer dead code (W1 closed).
2. **T2 (dead-field trim)**: Trimmed `ParsedAgentForTest` from 4 fields to 1 field (`aliases` only). The `name` is read from the filename stem externally at `load_agent_sources` (mod.rs:221), not stored in the parsed wrapper. F1 closed.
3. **T3 (anti-tautology test)**: Added `resolve_alias_for_first_match_wins` direct unit test exercising 3 cases (no-match, canonical-only, alias-match) independently of adapter call sites. Confirms helper logic is correct and non-tautological.

## Evidence

- 1 branch: `feat/kernel-cycle-38-inc-debt-012-cycle-37-followup-cleanup`
- 4 commits: T1 refactor / T2 dead-fields / T3 direct test / T4 closeout
- Test count delta: 316 → 317 (+1 direct test)
- Clippy delta: -2 in-scope warnings
  - `resolve_alias_for is never used` (W1) → GONE after T1
  - `fields 'description', 'tools', and 'body' are never read` (F1) → GONE after T2
  - 2 out-of-scope warnings remain (`field 'client' is never read`, `method 'get_client' is never used`)

## Cycle-32 Invariants Preserved

- **INV-8** (engine untouched): preserved — no changes to `sddk-engine`
- **INV-9** (no thread leaks): preserved
- **INV-10** (no Mutex on workflow state): preserved — `resolve_alias_for` consumes `&BTreeMap` by reference; no `Mutex`/`RwLock` introduced
- **INV-11** (deterministic output): preserved — `resolve_alias_for` iterates `renames.iter()` over `BTreeMap<String, String>` (alphabetical by definition); first-loaded alphabetical collision policy preserved by `renames_builder` at reconcile.rs:234-251

## Spec Correction Applied

| Field | Old (spec) | New (tasks/apply) | Rationale |
|-------|-----------:|------------------:|-----------|
| `ParsedAgentForTest` post-trim shape | "2 fields: `name`, `aliases`" | "1 field: `aliases`" | Spec hallucinated a `name` field. The actual struct has 4 fields (`description`, `tools`, `aliases`, `body`), none of which is `name`. `name` is read from the filename stem externally at `mod.rs:221`. Post-trim is 1 field. |

## Lifecycle

- **created**: 2026-08-25 (cycle-38 explore)
- **discovered**: cycle-38 explore (W1 + F1 surfaced as dead code from cycle-37)
- **remediated**: cycle-38 apply T1-T3
- **verified**: pending (cycle-38 sddk-verify)
- **archived**: pending (cycle-38 sddk-archive)

## Files Changed

| File | Change |
|------|--------|
| `crates/sddk-cli/src/dev/editor_adapters/json.rs` | Import + call `resolve_alias_for` (T1) |
| `crates/sddk-cli/src/dev/editor_adapters/claude.rs` | Import + call `resolve_alias_for` (T1) |
| `crates/sddk-cli/src/dev/editor_adapters/codex.rs` | Import + call `resolve_alias_for` (T1) |
| `crates/sddk-cli/src/dev/editor_adapters/mod.rs` | Trim `ParsedAgentForTest` to 1 field (T2) |
| `crates/sddk-cli/src/dev/tests/reconcile_tests.rs` | Add `resolve_alias_for_first_match_wins` direct test (T3) |
