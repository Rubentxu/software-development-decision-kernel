# HANDOFF: INC-M7-9 Option 3 fix — pre-push semantic check

**Date**: 2026-09-11
**Audit**: `p-63676b11dc0ef88f/inc-hygiene-2026-09-11` (extension #5)
**Path**: A-min scope (real code + test changes)
**Outcome**: INC-M7-9-PRE-PUSH-HOOK-CEREMONIAL-COMMIT closed via Option 3; pre-push hook now accepts semantic version bumps

## Summary

Closed INC-M7-9-PRE-PUSH-HOOK-CEREMONIAL-COMMIT (P3 low) by implementing Option 3 from the INC's remediation list (semantic check vs regex check). The `githooks/pre-push` hook now accepts EITHER:

1. **A commit with subject matching** `^chore\(release\): bump version` (existing flow, preserved), OR
2. **A commit that bumps `[workspace.package] version` in Cargo.toml** (new semantic check).

The semantic check uses awk to extract the workspace.package.version key from each commit's Cargo.toml content and compares across the commit range. If any commit changes the version, the push is accepted without requiring a separate `chore(release)` marker.

## What changed

### `githooks/pre-push`

- Added `INC_CEREMONIAL` reference and `cargo_workspace_version()` helper that parses `[workspace.package] version = "..."` from a Cargo.toml blob via awk.
- Added a second pass through the commit range that compares the workspace.package.version key across each commit's Cargo.toml. If the version differs between any pair (parent, commit), the push is accepted.
- The error message now names both remediation paths and references `INC-M7-9-PRE-PUSH-HOOK-CEREMONIAL-COMMIT` for traceability.

### `tests/test_push_prevention_hook.sh`

- Added Scenario (d): push to main WITH a Cargo.toml version bump but WITHOUT a marker commit. Expected: accepted via the semantic check.
- Original scenarios (a)/(b)/(c) preserved.

## Test results

```
Scenario (a): push to main WITHOUT release commit AND WITHOUT version bump → rejected ✓
Scenario (b): push to main WITH chore(release): bump version commit → accepted ✓
Scenario (c): push to non-main branch → accepted ✓
Scenario (d): push to main WITH Cargo.toml version bump (no marker) → accepted ✓
All 4 scenarios passed.
```

## Impact

### Forward-looking

- **Future release flow is simpler**: the typical release (feature work + version bump + GH Release) can land as a single commit containing the actual version bump, with no separate `chore(release)` marker required.
- **The ceremonial commit smell is resolved**: no more empty `chore(release): bump version to v1.167.8 (already published)` marker commits cluttering history.

### Backward-compatibility preserved

- The regex check still passes releases that use the traditional `chore(release): bump version` subject (e.g., from `bash scripts/release.sh`).
- Post-release doc handoffs without a version bump still require the marker (preserves the safety guarantee that every push to main is authorized by either a release or a documented marker).

### Self-application: the chicken-and-egg

This commit (`e214177`) is the first push that uses the new semantic check. The commit subject is `fix(githooks):` (not `chore(release):`) and the only "release marker" is the genuine Cargo.toml version bump from 1.167.8 → 1.167.9. The hook accepted the push via the semantic path. The version bump is real (closes an INC + adds new behavior + adds test), not ceremonial.

## Files changed

- `githooks/pre-push` — semantic check added (62 lines → 124 lines, +62)
- `tests/test_push_prevention_hook.sh` — Scenario (d) added (178 lines → 225 lines, +47)
- `Cargo.toml` — version bump 1.167.8 → 1.167.9 (release-level change)
- `~/.sddk-knowledge/sddk-framework/incs/INC-M7-9-PRE-PUSH-HOOK-CEREMONIAL-COMMIT.md` — vault frontmatter updated (closed)

## Vault counts after this commit

| Status | Before | After |
|--------|--------|-------|
| open   | 24 | 19 |
| closed | 14 | 19 |
| resolved | 7 | 7 |
| tracked | 1 | 1 |

## INC-M7-9 Action checklist (closed)

- [x] Identify or implement remediation (Option 3 chosen)
- [x] Update hook to support the new check
- [x] Add test scenario for the new path
- [x] Verify backward compatibility (regex path preserved)
- [x] Document in INC frontmatter
- [x] Commit + push to main (passes via semantic check on this commit)
