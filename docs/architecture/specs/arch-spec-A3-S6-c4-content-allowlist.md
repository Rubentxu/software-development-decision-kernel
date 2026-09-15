---
id: arch-spec-A3-S6-c4-content-allowlist
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/inc-a3-s1-c4-content-allowlist
source: docs/debt/INC-A3-S1-C4-LINE-SHIFT.md + ADR-0047-durable-debt-remediation
based_on: crates/sddk-cli/src/dev/arch_lint.rs
---

# arch-spec-A3-S6 — Content-addressed C4 legacy-authority allowlist

## Intent

Close `INC-A3-S1-C4-LINE-SHIFT` at its documented blocking threshold (5th
instance): replace the `"path:line"` freeze baseline with a content-addressed
allowance so module insertions in `crates/sddk-engine/src/lib.rs` no longer
invalidate the C4 deny-new-dependency guard.

## Scope (must)

- `LegacyApi` closed enum (`ForCli`, `Validate`) with `canonical_tag()`.
- `LegacyAllowance { path, api, line, max_occurrences }`.
- `normalize_legacy_line(&str) -> String` (whitespace-collapsed).
- `c4_find_new_legacy_authority_deps(files, &[LegacyAllowance])` keys on
  `(path, api, normalized line)` and flags occurrences beyond
  `max_occurrences`.
- `c4_legacy_allowlist_m1() -> &'static [LegacyAllowance]`.
- The baseline is regenerated from the current sources; the 21 `path:line`
  entries collapse to 17 distinct content keys.

## Scope (must NOT)

- No change to the detection patterns (the frozen site set is identical).
- No new crate dependency; no AST parser.
- No change to `sddk dev check-architecture` (different rule set).

## Requirements

- **REQ-A3S6-001** — `LegacyApi::canonical_tag()` returns `"for_cli"` /
  `"validate"`; `C4LegacyDependency.api` remains those strings.
- **REQ-A3S6-002** — `normalize_legacy_line` collapses runs of whitespace.
- **REQ-A3S6-003** — A site is allowed iff its `(path, api, normalized line)`
  key has a baseline entry and its occurrence count does not exceed that
  entry's `max_occurrences`.
- **REQ-A3S6-004** — Inserting lines above a frozen site does NOT produce a
  hit (shift-immunity). Regression pinned by test.
- **REQ-A3S6-005** — A genuinely new legacy call site (unallowlisted key) IS
  flagged with its physical `file:line`.
- **REQ-A3S6-006** — A duplicate of an allowed line beyond its
  `max_occurrences` IS flagged.
- **REQ-A3S6-007** — The shipped baseline covers every real site in the
  scanned file set of this checkout.
- **REQ-A3S6-008** — No two baseline entries share a key (multiplicity is
  folded into `max_occurrences`).
- **REQ-A3S6-009** — `docs/debt/INC-A3-S1-C4-LINE-SHIFT.md` is closed with
  resolution evidence.

## Acceptance tests

1. `c4_guard_flags_new_for_cli_site_outside_allowlist` (REQ-A3S6-005)
2. `c4_guard_allows_allowlisted_site_by_content` (REQ-A3S6-003)
3. `c4_guard_survives_line_shift` (REQ-A3S6-004)
4. `c4_guard_flags_duplicate_beyond_max_occurrences` (REQ-A3S6-006)
5. `c4_guard_max_occurrences_allows_exact_multiplicity` (REQ-A3S6-003)
6. `c4_guard_normalizes_whitespace` (REQ-A3S6-002)
7. `c4_guard_flags_same_content_in_different_file` (REQ-A3S6-003)
8. `c4_allowlist_baseline_is_internally_consistent` (REQ-A3S6-008)
9. `c4_guard_ignores_test_modules_and_non_surface_validate` (detection parity)
10. `c4_guard_flags_new_validate_site_naming_file_and_line` (REQ-A3S6-005)
11. `c4_baseline_covers_real_sources` (REQ-A3S6-007)
12. `c4_baseline_survives_real_lib_rs_module_insertion` (REQ-A3S6-004, real path)
13. `dev_doctor_c4_authority_single_admission_green_on_current_workspace` (e2e)

## Determinism

The guard is a pure function over `(files, allowlist)`. No IO inside the
matcher; the caller supplies file contents.
