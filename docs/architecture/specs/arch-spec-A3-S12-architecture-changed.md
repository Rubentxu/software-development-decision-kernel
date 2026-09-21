---
id: arch-spec-A3-S12-architecture-changed
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-12-architecture-changed
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/12-CLI-AGENT-UX.md
based_on: arch-spec-A3-S10-architecture-cli-surface + arch-spec-A3-S11-architecture-read-surfaces
---

# arch-spec-A3-S12 — `sddk architecture --changed`

## Intent

`12-CLI-AGENT-UX.md` names `sddk verify architecture [--changed]`. The receipt
verb ships but always with an **empty change basis**, so AC4's change-scoped
delta reports `affected_contracts: 0` on every run. This cycle gives it a real
basis: git diff → changed paths → declared units → affected contracts.

## Two blockers this cycle resolves

**B1 — path→unit mapping.** Units carry a `locator` (a file or directory path);
git reports changed paths. A unit is affected when a path overlaps its locator.

**B2 — contract→unit linkage.** `ArchitectureGraphOverlay::find_contracts_for_unit`
discovers contracts through `ArchitectureClaimedBy` relations, which only exist
if a *claim* was attached to the unit. The declaration links nothing today.

Resolution for B2: the CLI attaches a **genuine** AC1 claim for every
(contract, subject-unit) pair using `ContractEvaluation::evaluate` with the same
evidence the receipt uses (empty in v1 ⇒ truthfully `Unknown`). Only
`SingleAuthority` (component) and `UniqueOwner` (entity) are unit-scoped; the
other kinds remain global.

## Scope (must)

- `--changed` and `--base <rev>` on `architecture receipt`.
- `changed_units(root, base, decl)` — resolve the base, list changed paths from
  git (committed range ∪ worktree), map to unit refs.
- `declare_overlay` attaches the linkage claims for unit-scoped contracts.
- `ChangeBasis { base, changed_units }` on the receipt, `None` for a global run.
- The receipt renderer prints the change basis.
- Fail closed (exit 2) when the base cannot be resolved or git is unavailable.

## Scope (must NOT)

- MUST NOT mutate the repository; `git diff --name-only` is a read.
- MUST NOT fall back silently to an empty basis when `--changed` was requested.
- MUST NOT hand-build an `ArchitectureClaim`; use AC1's evaluator.
- MUST NOT change global-run behaviour (no `--changed` ⇒ exactly as A3-S11).
- MUST NOT add a new root module (no ADR required).

## Requirements (REQ-A3S12-NNN)

- **REQ-A3S12-001** — `path_overlaps(locator, path)` is true when either is a
  prefix of the other. Pin: `acceptance_path_overlap`.
- **REQ-A3S12-002** — A declared unit is affected iff a changed path overlaps its
  locator. Pin: `acceptance_changed_units_mapping`.
- **REQ-A3S12-003** — `--changed` without a resolvable base fails closed
  (exit 2) rather than reporting an empty basis. Pin: e2e
  `architecture_changed_requires_resolvable_base`.
- **REQ-A3S12-004** — With `--changed --base <rev>`, only contracts whose subject
  unit changed enter the delta scope. Pin: e2e
  `architecture_changed_scopes_to_touched_units`.
- **REQ-A3S12-005** — The receipt records `change_basis { base, changed_units }`
  when `--changed` was used, and omits it otherwise. Pin: e2e
  `architecture_changed_records_basis` + `acceptance_change_basis_optional`.
- **REQ-A3S12-006** — The text renderer prints the change basis. Pin: e2e
  `architecture_changed_text_names_base`.
- **REQ-A3S12-007** — A global run (no `--changed`) is unchanged: empty basis,
  `audited_contracts` from AC5. Pin: e2e `architecture_global_run_unchanged`.
  The text renderer prints `change_basis:      (global run; no --changed)` so a
  global run and an empty scoped run are never confused.
- **REQ-A3S12-008** — Untouched units' contracts do not enter the delta.
  Pin: e2e `architecture_changed_excludes_untouched`.
- **REQ-A3S12-009** — The linkage claims are produced by AC1's evaluator and are
  `Unknown` when no evidence is supplied. Pin: e2e
  `architecture_changed_reports_unknown_without_evidence`.
- **REQ-A3S12-010** — Path enumeration is byte-faithful: `git diff` is read
  NUL-separated (`-z`), so a path holding a byte outside ASCII or a space still
  matches its locator. Pin: e2e `architecture_changed_matches_non_ascii_paths`,
  `architecture_changed_matches_paths_with_spaces`. *Found in verify: without
  `-z`, git quotes `café/y.rs` as `"caf\303\251/y.rs"`, no locator matches, and
  `--changed` silently reports an empty basis for a file that did change.*
- **REQ-A3S12-011** — Renames are split (`--no-renames`), so a move scopes both
  the destination unit and the unit that lost its source. Pin: e2e
  `architecture_changed_scopes_both_sides_of_a_rename`,
  `detection_would_miss_the_renamed_away_side`. *Found in verify: with rename
  detection on, a move reports only the destination, so the unit whose entire
  source was deleted is never scoped.*

## Acceptance tests (shipped, 15)

CLI unit tests (`architecture_cmd.rs`):
1. `acceptance_path_overlap`
2. `acceptance_changed_units_mapping`
3. `acceptance_change_basis_optional`

Engine (`architecture_receipt/tests.rs`):
4. `acceptance_receipt_carries_change_basis`

CLI e2e (`tests/architecture_changed_cli_e2e.rs`):
5. `architecture_changed_requires_resolvable_base`
6. `architecture_changed_scopes_to_touched_units`
7. `architecture_changed_records_basis`
8. `architecture_changed_excludes_untouched`
9. `architecture_global_run_unchanged`
10. `architecture_changed_text_names_base` (REQ-006)
11. `architecture_changed_reports_unknown_without_evidence` (REQ-009)
12. `architecture_changed_matches_non_ascii_paths` (REQ-010, verify-found)
13. `architecture_changed_matches_paths_with_spaces` (REQ-010, verify-found)
14. `architecture_changed_scopes_both_sides_of_a_rename` (REQ-011, verify-found)
15. `detection_would_miss_the_renamed_away_side` (REQ-011, verify-found)

## Change-basis contract

| Situation | Behaviour |
|---|---|
| no `--changed` | global run; `change_basis: null`; empty AC4 basis |
| `--changed`, base resolves | `change_basis { base, changed_units }`; AC4 scoped to those units |
| `--changed`, base unresolvable / no git | exit 2 with an explanation |
| `--changed`, base resolves, nothing overlaps | `changed_units: []` and a recorded basis — a *real* zero |

## Determinism

Identical declaration + identical `(base, worktree, now-ms)` ⇒ identical output.
