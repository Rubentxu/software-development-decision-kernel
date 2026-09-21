# HANDOFF — cycle-38 — INC-DEBT-012 Cycle-37 Follow-up Cleanup

**Cycle**: kernel-cycle-38-inc-debt-012-cycle-37-followup-cleanup
**Date**: 2026-08-25
**Status**: Apply complete (T1-T4 done)
**Branch**: `feat/kernel-cycle-38-inc-debt-012-cycle-37-followup-cleanup`
**Last SHA**: `aeab3de` (T3)
**Next**: sddk-verify

## What Was Built

INC-DEBT-012: Cycle-37 dead-code cleanup (W1 + F1).

### T1 — DONE
- `refactor(cli): wire 3 adapters to resolve_alias_for helper (cycle-38, INC-DEBT-012)` (`2afdfe0`)
- Replaced inline `or_else` in json.rs:230-235, claude.rs:260-265, codex.rs:253-258 with `resolve_alias_for` helper call
- Added `resolve_alias_for` import to all 3 adapters
- 0 tests added (316 preserved — refactor only, no behavior change)
- Clippy W1 (`resolve_alias_for is never used`) → GONE

### T2 — DONE
- `refactor(cli): trim ParsedAgentForTest to aliases-only (cycle-38, INC-DEBT-012)` (`e39418a`)
- Trimmed `ParsedAgentForTest` from 4 fields to 1 field (`aliases`)
- Spec correction applied: post-trim is 1 field, not 2. `name` is read from filename stem externally.
- 0 tests added (316 preserved — parser tests only read `.aliases`)
- Clippy F1 (`fields 'description', 'tools', and 'body' are never read`) → GONE

### T3 — DONE
- `test(cli): add direct RED test for resolve_alias_for helper (cycle-38, INC-DEBT-012)` (`aeab3de`)
- Added `resolve_alias_for_first_match_wins` direct unit test with 3 sub-cases:
  - Case 1: no match → None
  - Case 2: canonical present → Some(canonical_entry, "canonical")
  - Case 3: alias match → Some(alias_entry, "alias")
- Anti-tautology verified: test calls helper directly; removing helper makes test fail to compile (E0432)
- Test count: 316 → 317 (+1)

### T4 — DONE (this commit)
- INC-DEBT-012 closure document
- Handoff document (this file)
- CHANGELOG entry

## What Remains

### sddk-verify
- Run `cargo test --workspace --locked` (expected: 317 sddk-cli, 128 sddk-engine)
- Run `cargo clippy --workspace --all-targets -- -D errors` (expected: 0 errors)
- Run `cargo fmt --all -- --check` (expected: 0 diffs)
- Verify clippy count: `cargo clippy -p sddk-cli --all-targets --no-deps | grep -E "warning:.*never (used|read)"` → only 2 out-of-scope warnings remain
- `git diff --stat 3442215..HEAD -- crates/sddk-engine/` → empty (INV-8)

### sddk-archive
- Archive manifest generation
- Release receipt capture
- INC-DEBT-012 debt ledger closure

## Key Technical Decisions

### Spec Correction — ParsedAgentForTest
The spec described a 2-field post-trim shape (`name` + `aliases`). The apply agent discovered the actual struct has 4 fields (`description`, `tools`, `aliases`, `body`) and `name` is NEVER a field — it is read from the filename stem externally at `load_agent_sources` (mod.rs:221). Post-trim is truthfully 1 field (`aliases`).

### resolve_alias_for First-Match-Wins Preserved
The helper's first-match-wins semantics (canonical first, then alias lookup) are preserved exactly from cycle-37. No behavioral change — only refactoring to eliminate dead code.

### Anti-Tautology Design
The direct test exercises `resolve_alias_for` with an explicit `BTreeMap` + closure — completely independent of the adapter call sites. This means:
- If helper is removed → test fails to compile (E0432)
- If adapter calls are removed (T1 reverted) → test still passes (proves both axes are exercised independently)

## Files Changed

| File | T | Change |
|------|---|--------|
| `crates/sddk-cli/src/dev/editor_adapters/json.rs` | T1 | Import `resolve_alias_for`; replace inline `or_else` with helper call |
| `crates/sddk-cli/src/dev/editor_adapters/claude.rs` | T1 | Import `resolve_alias_for`; replace inline `or_else` with helper call |
| `crates/sddk-cli/src/dev/editor_adapters/codex.rs` | T1 | Import `resolve_alias_for`; replace inline `or_else` with helper call |
| `crates/sddk-cli/src/dev/editor_adapters/mod.rs` | T2 | Trim `ParsedAgentForTest` to 1 field (`aliases`); trim `parse_agent_file_for_test` to 1-field initializer |
| `crates/sddk-cli/src/dev/tests/reconcile_tests.rs` | T3 | Add `resolve_alias_for_first_match_wins` direct unit test (3 sub-cases) |
| `docs/debt/INC-DEBT-012-cycle-37-followup-cleanup.md` | T4 | INC closure document |
| `docs/handoff/HANDOFF-2026-08-25-cycle-38-inc-debt-012-cycle-37-followup-cleanup.md` | T4 | This handoff |
| `CHANGELOG.md` | T4 | Cycle-38 entry |

## Test Count

- Before cycle-38: 316 tests (sddk-cli)
- After cycle-38: 317 tests (+1 direct test)
  - T1: 0 (refactor only, 316 preserved)
  - T2: 0 (dead-field trim, 316 preserved)
  - T3: 1 (`resolve_alias_for_first_match_wins`)
  - T4: 0 (docs)
- `sddk-engine`: 128 unchanged (INV-8 preserved)

## Clippy Baseline

| Warning | Before | After | Delta |
|---------|-------:|------:|------:|
| `resolve_alias_for is never used` (W1) | present | gone | -1 |
| `fields 'description', 'tools', and 'body' are never read` (F1) | present | gone | -1 |
| `field 'client' is never read` | present | present | 0 (out of scope) |
| `method 'get_client' is never used` | present | present | 0 (out of scope) |
| **Total in-scope** | **2** | **0** | **-2** |

## Invariants Preserved (cycle-32)

- INV-8 (engine untouched): preserved — no changes to `sddk-engine`
- INV-9 (no thread leaks): preserved — single-threaded refactor only
- INV-10 (no Mutex on workflow state): preserved — `resolve_alias_for` consumes `&BTreeMap` by reference
- INV-11 (deterministic output): preserved — `BTreeMap` gives deterministic iteration; first-loaded alphabetical wins on collision

## Open Items

- **Q-γ adjacent (`name` field hallucination)**: RESOLVED in T2. The spec described a 2-field post-trim shape that didn't match reality. Trimmed to truthful 1-field shape (`aliases`). Documented in INC-DEBT-012 §Spec Correction.
