---
adr_id: ADR-0047-inc02
title: Cycle-9 lessons — apply phase rigor + LOC budget reality
status: accepted
date: 2026-08-22
cycle: kernel-cycle-9-hardening-loc-refactor
supersedes: none
related_adrs: [ADR-0047, ADR-0047-inc01]
decision_authority: orchestrator+verify
---

# ADR-0047-inc02 — Cycle-9 lessons

## Status

ACCEPTED with two distinct lessons captured.

## Context

Cycle-9 (kernel-cycle-9-hardening-loc-refactor) had two failures despite succeeding at its core refactor:

### Lesson A — Apply phase ran gates against dirty working tree

The apply phase agent claimed "1067 tests passing" based on running `cargo test --workspace` against a working tree that had a manual uncommitted file deletion (`crates/sddk-domain/src/models.rs`). The committed state at HEAD was broken: `E0761` ambiguity + 24 cascading compile errors. `origin/main` was BROKEN — any clone would fail to build.

The recovery was a single surgical commit (`4657ef3`: `git rm crates/sddk-domain/src/models.rs`).

### Lesson B — Per-file LOC targets unachievable for Rust modules

Cycle-9 absorbed only 160 LOC of 927 target (17%). The design's per-file LOC targets (≤120, ≤80, ≤400) are unachievable without removing functionality because:

| File | Real LOC | Target | Why target missed |
|------|---------|--------|------------------|
| `debt.rs` | 394 | ≤120 | clap derive + 4 subcommand arg parsers = ~80 LOC irreducible |
| `gate_evaluator.rs` | 157 | ≤80 | test fixtures (40 LOC) + 4 named variants = ~75 LOC irreducible |
| `inc_generator.rs` | 216 | ≤80 | public fn signatures + TemplateContext + 6 test bodies = ~135 LOC irreducible |
| `models.rs` split | 680 | ≤400 | module-boundary overhead (declarations + re-exports + per-file headers) adds ~50 LOC per file |

## Decision

### Lesson A — Apply phase rigor

Apply phase MUST run cargo build/test/clippy/fmt against the **commit's tree**, not the working tree. Concretely:

1. After each commit, run `git status --porcelain` to confirm clean state.
2. Run all gates against the clean HEAD, not against `git stash`-ed dirty changes.
3. If manual context changes are needed for the next slice, `git stash` them before running gates.
4. Verify reports MUST explicitly state "tested against commit `<sha>`, working tree clean".

This is added to `prompts/sddk/phases/apply.md` §"Discipline Rules" and `prompts/sddk/phases/verify.md` §"Mandatory Gates" as a new mandatory gate.

### Lesson B — LOC budget policy

The project should adopt **total-module-sum LOC budgets**, not per-file LOC targets. Specifically:

- For multi-file modules (e.g., `models/` subdir after split), budget is the sum across all files in the module.
- For single-file modules, budget is the file itself.
- For test fixtures, budget is in a separate "test code" category (not counted against implementation budget).
- For clap derive / serde derives / boilerplate, budget is in a separate "boilerplate" category.

Future cycles' design phases must distinguish these categories and budget accordingly.

## Forward remediation

- DEBT-CYCLE-9-APPLY-DISCIPLINE (FIND-0002) → kernel-cycle-10 (or next cycle that touches apply.md).
- DEBT-CYCLE-9-LOC-OVERAGE (FIND-0001) → kernel-cycle-10-loc-budget-policy (or whatever cycle-10 is).

## Notes

- The fixup commit (4657ef3) is part of cycle-9's commit chain.
- Cycle-9 is not declared closed at this point; debt-verify + release + archive still pending.
- Severity: medium for both lessons — neither blocks cycle closure, but both create follow-up debt for cycle-10.