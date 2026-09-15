---
id: INC-A3-S1-C4-LINE-SHIFT
title: "C4 legacy-authority allowlist uses raw line numbers; any insertion in lib.rs invalidates it"
status: open
severity: medium
priority: P2
fingerprint: "c4_allowlist_line_shift_v1"
fingerprint_aliases: ["c4_allowlist_line_shift_v1"]
cluster_id: CL-09
created: 2026-09-14
created_by: orchestrator (cycle p-63676b11dc0ef88f/a3-1-kmt-foundation)
owner: orchestrator
cycle_source: p-63676b11dc0ef88f/a3-1-kmt-foundation
finding_ref: OBSERVATION-A3-S1
---

# INC-A3-S1-C4-LINE-SHIFT — C4 legacy-authority allowlist uses raw line numbers

> Durable cross-cycle record. Registered from observation during A3-S1
> (cycle `p-63676b11dc0ef88f/a3-1-kmt-foundation`).
> See ADR-0047 §3.2.

## Context

The M1 legacy-authority freeze baseline is stored as a `&[&str]` constant
`C4_LEGACY_ALLOWLIST_M1` in `crates/sddk-cli/src/dev/arch_lint.rs`. Each
entry is a literal `"path:line"` string. The scanner
`c4_find_new_legacy_authority_deps` compares `format!("{path}:{lineno}")`
against this list.

When A3-S1 inserted `pub mod knowledge;` in `crates/sddk-engine/src/lib.rs`
between `pub mod join_guard;` and `pub mod lab_promotion;` (one new line,
alphabetical ordering), the three pre-existing `auth.validate(...)` call
sites at lines 1165 / 1292 / 1352 shifted to 1166 / 1293 / 1353. The
allowlist still contained the old numbers, so the doctor falsely flagged
the three calls as "new legacy authority dependencies" and
`dev_doctor_c4_authority_single_admission_green_on_current_workspace`
failed.

## Instance log

| # | Cycle | Insertion | Sites | Priority |
|---|-------|-----------|-------|----------|
| 1 | `a3-1-kmt-foundation` | `pub mod knowledge;` | 1165/1292/1352 → 1166/1293/1353 | P3 |
| 2 | `a3-2-architectural-contract` | `pub mod architectural_contract;` | → 1167/1294/1354 | P3 |
| 3 | `a3-3-architecture-graph-overlay` | `pub mod architecture_graph;` | → 1168/1295/1355 | P3 |
| 4 | `a3-4-paradigm-lens-profile` | `pub mod paradigm_profile;` | → 1169/1296/1356 | P3 |
| 5 | `a3-5-ac4-verify-contracts` | `pub mod architecture_conformance;` | → 1170/1297/1357 | **P2** |

**Threshold reached at instance 5.** Every subsequent `pub mod` insertion in
`crates/sddk-engine/src/lib.rs` breaks the allowlist. Priority raised P3 → P2
(later-cycle planning commitment) and the content-based refactor is scheduled
as the immediate follow-up cycle.

## Mitigation applied in this cycle

The three allowlist entries were bumped by `+1` and a comment block was
added referencing this INC. Workspace tests now pass.

```rust
// A3-S1 (2026-09-14): line numbers shifted by +1 after alphabetical
// re-sort of `pub mod knowledge;` insertion in lib.rs. Code at the
// three sites is unchanged. See INC-A3-S1-C4-LINE-SHIFT.
"crates/sddk-engine/src/lib.rs:1166",
"crates/sddk-engine/src/lib.rs:1293",
"crates/sddk-engine/src/lib.rs:1353",
```

## Underlying fragility

The allowlist format couples the baseline to physical line numbers. Any
re-ordering or insertion in the listed files invalidates the baseline
even when the *behaviour* at the call sites is unchanged. This is
especially fragile for `crates/sddk-engine/src/lib.rs`, which is the
canonical place where new engine modules are declared.

## Recommended follow-up (not in A3-S1 scope)

1. Replace the `path:line` format with `path:fn` (or `path:symbol`)
   anchored to function/item identity (e.g. `crates/sddk-engine/src/lib.rs:Engine::evaluate_gate`
   resolved via rustdoc JSON or `syn::ItemFn`).
2. Add a `cargo xtask` regenerator that runs the scanner in "compute
   allowlist" mode and PR-updates the constant when the diff is purely
   numeric.
3. Consider excluding `lib.rs` from the M1 baseline entirely (route every
   `auth.validate` through a single facade in `admission.rs`, which is
   already the M9 plan) so reordering modules no longer shifts
   authoritative line numbers.

## Reproduction

1. `git checkout d842265 && cargo test -p sddk-cli --test dev_lint_e2e dev_doctor_c4_authority_single_admission`
   → passes (baseline).
2. Add `pub mod new_mod;` anywhere above line 1166 in
   `crates/sddk-engine/src/lib.rs`.
3. `cargo test -p sddk-cli --test dev_lint_e2e dev_doctor_c4_authority_single_admission`
   → fails; `sddk dev doctor --format json` reports
   `"c4.authority_single_admission": "new legacy authority dependency ..."`
   for the same three call sites, simply shifted by N lines.

## Related

- `docs/debt/INC-DEBT-007-preexisting-clippy-sddk-cli.md` (similar baseline
  fragility class for clippy lints).
- AC evolution package §7 (mutation probes): the allowlist would benefit
  from a mutation test that confirms shifting lines without changing
  behaviour does NOT trip the check.
