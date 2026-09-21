# HANDOFF — A3-S6 / AC6 closure + v1.169.28 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/a3-6-ac6-mutation-probes` (A-lite)
- **Status:** **CLOSED** (sequence 14)
- **Release:** `v1.169.28` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.28
- **HEAD:** `317bf3b` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.28`

## What shipped

AC6 of the Architecture Conformance track (critical mutation probes):

> For critical invariants, prove the guard catches an injected violation in
> sandbox.

New module `crates/sddk-engine/src/architecture_mutation/` (~880 LoC, 5 files,
28 tests) + `tests/ac6_ac4_witness_bridge.rs` (2 integration tests).

| File | Purpose |
|---|---|
| `types.rs` | `MutationKind` (4-closed), `GuardCheck` (2-closed), `MutationInjection` (3-closed), `MutationGuard`, `MutationSpec`, `MutationProbe`, `MutationSuiteReceipt`, `GuardHit` |
| `sandbox.rs` | `MutationSandbox` (disposable in-memory `path -> content`), `apply_injection`, `evaluate_guard` |
| `run.rs` | `run_mutation_probe`, `run_mutation_suite`, `critical_mutations`, `critical_guards` |
| `tests.rs` | 28 tests (incl. AC-UAT-010) |

### The four critical mutations (AC-038-003)

| id | injection | guard |
|---|---|---|
| `mut-provider-type-leak` | `use tonic::transport::Channel;` into a domain file | `ForbiddenLines` (provider SDK prefixes) |
| `mut-alignment-to-governance` | `use crate::authority_engine::AuthorityEngine;` into an alignment file | `ForbiddenLines` (authority/capability) |
| `mut-workbook-canonical-write` | `write_canonical(...)` into a workbook file | `ForbiddenLines` (canonical-write markers) |
| `mut-second-canonical-writer` | a second `append_canonical` definition | `MaxOccurrences { max_allowed: 1 }` |

### Safety (AC-038-001)

The runner clones the sandbox, injects into the clone, evaluates the guard and
discards it. **No filesystem IO at all** — mutations never touch the working
tree. Pinned by `anti_encroachment_no_filesystem_writes` and
`acceptance_probe_does_not_mutate_input_sandbox`.

### AC4 loop closed

`tests/ac6_ac4_witness_bridge.rs::ac6_witnesses_drive_ac4_contradiction`:
AC6 detects the provider-type leak, its witness feeds AC4's
`ConformanceInputs.contradiction_witnesses`, and the contract resolves to
`DeltaContractStatus::Contradicted` — with a control showing `Verified` without
the witnesses. This is the seam AC4 deliberately left open.

## Gates (all green)

explore · specify · design · build · verify (4) · release (2) · archive (2) = 12.

## Evidence

```
cargo test --workspace                                 -> 196 blocks, 4076 passed, 0 failed
cargo test -p sddk-engine --lib architecture_mutation:: -> 28 passed
cargo test -p sddk-engine --test ac6_ac4_witness_bridge  -> 2 passed
cargo test -p sddk-engine --lib                         -> 996 passed (was 968)
cargo fmt --check / clippy -p sddk-engine --lib --tests -- -D warnings -> clean
shellcheck tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh -> exit 0
sddk dev doctor -> c4.authority_single_admission: present
bash scripts/release.sh --skip-tests                    -> exit 0 (14 steps, 196s)
```

## Notable: the A3-S6 refactor paid off

Adding `pub mod architecture_mutation;` to `lib.rs` required **zero** allowlist
maintenance. The previous five module insertions each needed a manual
line-number bump; the content-addressed baseline absorbed this one silently.

## Commits

| SHA | Subject |
|---|---|
| `ac700e8` | docs(spec): cycle-bounded spec for A3-S6 (AC6 mutation probes) |
| `dc99181` | feat(engine): AC6 critical mutation probes + AC4 witness bridge (A3-S6) |
| `317bf3b` | chore(release): bump version 1.169.27 -> 1.169.28 |

Also: `ADR-0116` promoted `proposed → accepted` with implementation evidence
(counterfactual stages noted as AC13).

## Carry-over debt

None.

## Next roadmap items

Per `docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/09-ROADMAP.md`:

1. **AC5 — DebVerify architecture audit.** Global challenge pass for duplicate/
   shadow authorities, missing owners, bypasses, stale compatibility even with
   no recent delta. Must **not** be conflated with `verify --full`
   (AC-UAT-009). Note: `sddk cycle supersede` already exists, unrelated.
2. **AC7 — OO/FP/ADT/DSL lens assessments.** Feeds AC3's `LensAssessment` and
   AC4's `paradigm_alignment` vector dimension (currently always
   `NotEvaluated`).
3. **AC8 — SDDK self-audit + `ArchitectureConformanceReceipt`.** The Base
   production gate; consumes AC1–AC7.

## Deferred (unchanged)

- CLI surfaces for AC4's delta and AC6's mutation suites.
- AC13 counterfactual stages (ADR-0116 decision 1b).
