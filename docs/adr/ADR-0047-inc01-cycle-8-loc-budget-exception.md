---
adr_id: ADR-0047-inc01
title: Cycle-8 LOC budget exception — accepted with cycle-9 remediation
status: accepted
date: 2026-08-21
cycle: kernel-cycle-8-debt-runtime-implementation
supersedes: none
related_adrs: [ADR-0047]
decision_authority: user
---

# ADR-0047-inc01 — Cycle-8 LOC budget exception

## Status

ACCEPTED with documented forward remediation.

## Context

Cycle-8 (kernel-cycle-8-debt-runtime-implementation) delivered 1.290 LOC
non-doc Rust (1.174 implementation + 116 tests) against the budget of
≤450 LOC inherited from prior Rust-light cycles (cycle-5, cycle-6,
cycle-7a, cycle-7b, cycle-7c all delivered 25-200 LOC).

The path chosen was explicit A-full Rust implementation; the budget was
calibrated for cycles whose primary artifact was docs/JSON/YAML, not
Rust source.

The verify phase flagged this as AC-K8-X01-3 BLOCKER. The user accepted
the exception ("Aceptar excepción, registrar en retrospectiva") given:
  - Path A-full was explicitly chosen for this cycle.
  - Per-file LOC inflation is justified by scope per subsystem.
  - Closing the cycle is more valuable than refactoring mid-cycle.

## Decision

ACCEPT the exception. Do NOT consider this a precedent for future
cycles — the ≤450 budget remains in force for cycles whose primary
artifact is docs/JSON/YAML (per AGENTS.md §4).

Cycle-8 closes with verdict PASS_WITH_NOTE; AC-K8-X01-3 moves from
FAIL to PASS_WITH_NOTE.

## Forward remediation (cycle-9 candidate debt)

The 4 files with the highest overage will be candidates for a hardening
cycle (cycle-9 candidate):

| File | LOC delivered | LOC target | Overage | Rationale |
|------|---------------|------------|---------|-----------|
| crates/sddk-cli/src/debt.rs | 333 | ≤120 | 213 | CLI surface for 4 subcommands + clap derive + error types |
| crates/sddk-engine/src/gate_evaluator.rs | 242 | ≤80 | 162 | 2 gate definitions + 3 outcome variants + 4 unit tests |
| crates/sddk-engine/src/inc_generator.rs | 237 | ≤80 | 157 | include_str! template + frontmatter serde + 4 unit tests |
| crates/sddk-domain/src/models.rs | 149 | ≤30 | 119 | 3 types × full serde derives + 4 unit tests |

**Total bring-forward**: 651 LOC of refactor opportunity, but no
correctness or performance issue identified in verify phase.

## Debt entry (cycle-9 candidate)

| Field | Value |
|-------|-------|
| Finding ID | DEBT-CYCLE-8-LOC-OVERAGE |
| Severity | medium |
| Priority | P2 |
| Cluster | over-engineering |
| Status | open |
| Remediation cycle | cycle-9 (candidate) |
| Estimated LOC reduction | 651 (across 4 files, no functional change) |

## Notes

- The LOC budget in AGENTS.md §5 / ADR-0047 remains ≤450 for non-Rust-
  primary cycles. This ADR does not change that.
- If cycle-9 absorbs the refactor, it should be path A-min (Rust-only)
  with verify-only phase.
- If cycle-9 is taken by a different higher-priority need, the debt
  remains open and reappears in cycle-10's debt report.

## Cycle-9 reconciliation (2026-08-22)

After cycle-9 explore (kernel-cycle-9-hardening-loc-refactor), the
actual file sizes diverge from the original estimates in this ADR:

| File | ADR estimate | Actual (ground-truth) | Post-cycle-9 | Delta absorbed |
|------|-------------|------------------------|-------------|---------------|
| crates/sddk-cli/src/debt.rs | 333 | 399 | 394 | 5 |
| crates/sddk-engine/src/gate_evaluator.rs | 242 | 245 | 157 | 88 |
| crates/sddk-engine/src/inc_generator.rs | 237 | 237 | 216 | 21 |
| crates/sddk-domain/src/models.rs | 149 | **726** | 680 (sum of subdir) | 46 |

The `models.rs` discrepancy (5×) is the largest. The ADR originally
counted only the debt-domain slice of the file; the actual file
includes the full persistence layer (30 public types across 9 concerns).

Cycle-9 absorbs ~160 LOC out of 1.297 total overage (~12%).
Residual ~1.137 LOC stays filed as DEBT-CYCLE-9-LOC-OVERAGE.

**Key architectural decisions made in cycle-9:**
- `models.rs` split into 15-module subdir preserving pub use re-exports
- `gate_evaluator.rs` dead-code removal + generic predicate collapse
- `inc_generator.rs` TemplateContext extraction (12-replace chain)
- `debt.rs` VaultError introduction + backfill_report pure extraction

**Note on LOC targets**: The ≤120/≤80/≤400 targets from the original
ADR were aspirational. The realistic post-refactor LOC is higher due to:
- clap derive boilerplate (unavoidable without breaking CLI surface)
- Test fixtures that can't be further inlined without expanding test code
- Module boundary overhead from the models/ subdir split
