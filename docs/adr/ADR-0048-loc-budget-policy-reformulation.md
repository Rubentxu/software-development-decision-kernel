---
adr_id: ADR-0048
title: LOC budget policy — total-module-sum instead of per-file
status: accepted
date: 2026-08-22
cycle: kernel-cycle-10-apply-discipline-loc-policy
supersedes: none
related_adrs: [ADR-0047, ADR-0047-inc01, ADR-0047-inc02]
decision_authority: orchestrator+spec
---

# ADR-0048 — LOC budget policy reformulation

## Status

ACCEPTED in cycle-10.

## Context

Cycle-8 (kernel-cycle-8-debt-runtime-implementation) released v1.35.0 with 1.290 LOC absorbed vs 450 target. ADR-0047-inc01 documented the exception.

Cycle-9 (kernel-cycle-9-hardening-loc-refactor) absorbed 160 LOC vs 927 target = 17%. Per-file LOC targets were unachievable without removing functionality. ADR-0047-inc02 documented the lesson.

## Concrete numbers from cycle-9

| File | Real LOC | Target | Why target missed |
|------|---------|--------|------------------|
| `crates/sddk-cli/src/debt.rs` | 394 | <=120 | clap derive + 4 subcommand arg parsers = ~80 LOC irreducible |
| `crates/sddk-engine/src/gate_evaluator.rs` | 157 | <=80 | test fixtures (40 LOC) + 4 named variants = ~75 LOC irreducible |
| `crates/sddk-engine/src/inc_generator.rs` | 216 | <=80 | public fn signatures + TemplateContext + 6 test bodies = ~135 LOC irreducible |
| `crates/sddk-domain/src/models.rs` (split) | 680 | <=400 | module-boundary overhead (declarations + re-exports + per-file headers) adds ~50 LOC per file |

Total irreducible: ~360 LOC. Module-boundary overhead: ~50 LOC.

## Decision

Project LOC budget policy shifts from per-file targets to **total-module-sum budgets**:

1. **Implementation LOC**: code that does the work (function bodies, type definitions, match arms).
2. **Boilerplate LOC**: derive macros (`#[derive(...)]`), trait impls that just delegate, clap/serde annotations.
3. **Test fixture LOC**: setup helpers, mock data, test-only types.

These categories are budgeted separately. Default budget per cycle:
- Implementation: <= 200 LOC.
- Boilerplate: <= 100 LOC.
- Test fixtures: <= 200 LOC.

A single file's LOC target is the sum of its implementation + boilerplate + test fixtures. Per-file targets are NOT enforced.

## Why

Per-file LOC targets force unnatural splits (functions into multiple files just to satisfy targets). They also make cycle planning brittle (every cycle has to re-justify why its targets are missed).

Total-module-sum budgets are:
- Aligned with how Rust actually organizes code (modules, not files).
- Tolerant of boilerplate that derive macros legitimately add.
- Easier to budget (one number per module, not per file).

## Consequences

- Future cycles' design phases must distinguish implementation / boilerplate / test fixture LOC.
- Debt findings that miss per-file targets should be filed against implementation LOC only.
- ADR-0047-inc01 (cycle-8) is amended by reference: the per-file <= 80/<= 120 targets it cited are deprecated.

## Notes

- The reformulation is orthogonal to apply discipline (ADR-0047-inc02 §Lesson A). Two separate lessons.
