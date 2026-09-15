# HANDOFF — A3-S7 / AC5 closure + v1.169.29 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/a3-7-ac5-debverify-audit` (A-lite)
- **Status:** **CLOSED** (sequence 14)
- **Release:** `v1.169.29` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.29
- **HEAD:** `e8bb81d` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.29`

## What shipped

AC5 of the Architecture Conformance track (DebVerify global audit):

> Global challenge pass that looks for duplicate/shadow authorities, missing
> owners, bypasses, stale compatibility and contradictions even when no recent
> delta points at them.
> **Exit:** no conflation with `verify --full`.

New module `crates/sddk-engine/src/architecture_debverify/` (~720 LoC, 5 files,
28 tests) + `tests/ac5_not_verify_full.rs` (2 integration tests).

| File | Purpose |
|---|---|
| `types.rs` | `DebVerifyFindingKind` (5-closed), `FindingSeverity` (3-closed, total mapping), `DebVerifyFinding`, `DebVerifyAudit` |
| `detectors.rs` | the five global detectors |
| `audit.rs` | `run_debverify_audit(overlay, contracts, now)` + digest |
| `tests.rs` | 28 tests (incl. AC-UAT-009) |

### The five findings

| Kind | Severity | Detection |
|---|---|---|
| `ShadowAuthority` | Critical | ≥2 `SingleAuthority` over one component, or ≥2 `UniqueOwner` over one entity |
| `AuthorityBypass` | Critical | a `ForbiddenDependency` whose edge exists in the graph |
| `MissingOwner` | High | an owner absent from graph node locators |
| `Contradiction` | High | a subject declared both authority-owned and `ProjectionOnly` |
| `StaleCompatibility` | Medium | a compatibility window elapsed with no replacement |

### Exit criterion made structural (AC-034-002)

Four independent pins:
1. `run_debverify_audit(overlay, contracts, now)` — **three** arguments, no change basis.
2. Source-grep: production code never references `changed_units` / `ConformanceInputs` / `compute_conformance_delta` / `ArchitectureConformanceDelta` / `DeltaContractStatus`.
3. Distinct output type with no conversion to/from AC4's delta.
4. **Behavioural** (`tests/ac5_not_verify_full.rs`): same overlay, same contracts, same `now`:
   - AC4 `compute_conformance_delta(..., changed_units = [])` → `affected = {}`
   - AC5 `run_debverify_audit(...)` → 1 **Critical** `ShadowAuthority` over `comp:auth`

That is exactly AC-UAT-009.

## New ADR

`ADR-0117-DEBVERIFY-GLOBAL-ARCHITECTURE-AUDIT` (accepted, mirrored).

Triggered by a real gate the session had not yet exercised:
`crates/sddk-cli/tests/context_fitness.rs::no_new_root_level_context_module_without_adr`.
A new root-level engine module requires an ADR rather than a baseline entry.
`architecture_conformance` (AC4) and `architecture_mutation` (AC6) already
satisified it via ADR-0112/ADR-0116 mentions; DebVerify did not, so it got a
proper ADR with rejected alternatives (`--full` flag, consume-the-delta, fold
into mutation probes).

Notably, that same fitness file contains guards
(`no_workbook_canonical_write`, `no_knowledge_to_provider_sdk`,
`no_domain_to_rpc_or_provider_or_host_sdk`,
`no_alignment_to_governance_authority_or_instruction_compiler`)
that are the real-repo counterparts of AC6's critical mutations.

## Gates (all green)

explore · specify · design · build · verify (4) · release (2) · archive (2) = 12.

## Evidence

```
cargo test --workspace                                  -> 197 blocks, 4106 passed, 0 failed
cargo test -p sddk-engine --lib architecture_debverify:: -> 28 passed
cargo test -p sddk-engine --test ac5_not_verify_full      -> 2 passed
cargo test -p sddk-engine --lib                          -> 1024 passed (was 996)
cargo test -p sddk-cli --test context_fitness            -> 7 passed
cargo fmt --check / clippy -p sddk-engine --lib --tests -- -D warnings -> clean
shellcheck tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh -> exit 0
sddk dev doctor -> c4.authority_single_admission: present
bash scripts/release.sh --skip-tests                     -> exit 0 (14 steps, 199s)
```

## Commits

| SHA | Subject |
|---|---|
| `a468089` | docs(spec): cycle-bounded spec for A3-S7 (AC5 DebVerify architecture audit) |
| `709dcdb` | feat(engine): AC5 DebVerify global architecture audit (A3-S7) |
| `c530f8e` | docs(adr): ADR-0117 DebVerify is a global audit, not a Verify mode |
| `e8bb81d` | chore(release): bump version 1.169.28 -> 1.169.29 |

## Carry-over debt

None.

## Next roadmap items

Per `docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/09-ROADMAP.md`:

1. **AC7 — OO/FP/ADT/DSL lens assessments.** Feeds AC3's `LensAssessment` and
   AC4's `paradigm_alignment` vector dimension (currently always
   `NotEvaluated`).
2. **AC8 — SDDK self-audit + `ArchitectureConformanceReceipt`.** The Base
   production gate; consumes AC1–AC7 and reproduces a representative subset of
   A0/A1 findings (AC-UAT-016).

## Deferred (unchanged)

- CLI surfaces for AC4's delta, AC5's audit and AC6's mutation suites.
- AC13 counterfactual stages (ADRs 0116/0117 record this).
- AC10 typed-edge refinement of AC5's identifier heuristic.
