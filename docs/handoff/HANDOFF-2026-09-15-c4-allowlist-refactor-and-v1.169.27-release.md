# HANDOFF — INC-A3-S1-C4-LINE-SHIFT closure + v1.169.27 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/inc-a3-s1-c4-content-allowlist` (A-min)
- **Status:** **CLOSED** (sequence 12)
- **Release:** `v1.169.27` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.27
- **HEAD:** `1244085` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.27`

## What shipped

Closure of `INC-A3-S1-C4-LINE-SHIFT` (the C4 legacy-authority freeze allowlist
used raw line numbers and broke on every `pub mod` insertion in `lib.rs`;
5 instances recorded, blocking threshold reached at instance 5).

`crates/sddk-cli/src/dev/arch_lint.rs`:

```rust
pub struct LegacyAllowance {
    pub path: &'static str,
    pub api: LegacyApi,           // ForCli | Validate
    pub line: &'static str,        // whitespace-normalized source line
    pub max_occurrences: usize,    // multiplicity cap within the file
}
```

`c4_find_new_legacy_authority_deps` keys each detected site on
`(path, api, normalize(line))` and flags only occurrences beyond the allowed
multiplicity. Physical line numbers are reported but never keyed.

Baseline regenerated from the current sources: 21 `path:line` entries →
**17 distinct content keys**. Detection patterns unchanged (frozen site set
identical).

## Proof of closure (real repo, real binary)

| Scenario | Result |
|---|---|
| baseline | `c4.authority_single_admission: present` |
| +2 fake `pub mod` lines in `lib.rs` (GateReceipts site 1170 → 1172) | `present` |
| inject a new legacy call into a scanned file | `missing` + `new legacy authority dependency crates/sddk-engine/src/cycle_pause.rs:1 (validate)` |

Row 2 is the exact failure mode that recurred five times; it now passes.
Row 3 proves the freeze is still enforced.

## Gates (all green)

explore `exploration-sufficient` · specify `requirements-testable` ·
build `implementation-complete` · verify `tests-pass`, `policy-compliant`,
`debt-severity-assigned`, `debt-priority-assigned` · release
`no-pending-effects`, `release-uat-approved` · archive `ledger-valid`,
`vault-index-current`.

## Evidence

```
cargo test --workspace                                -> 195 blocks, 4046 passed, 0 failed
cargo test -p sddk-cli arch_lint                      -> 83 passed (was 75)
cargo test -p sddk-cli --test dev_lint_e2e            -> 5 passed
cargo fmt --check                                     -> clean
cargo clippy -p sddk-cli --all-targets -- -D warnings -> clean
bash scripts/release.sh --skip-tests                  -> exit 0 (14 steps, 187s)
```

## Commits

| SHA | Subject |
|---|---|
| `8f9db48` | docs(spec): cycle-bounded spec for A3-S6 (C4 content-addressed allowlist) |
| `890a54c` | fix(cli): content-addressed C4 allowlist (closes INC-A3-S1-C4-LINE-SHIFT) |
| `16843bd` | docs(debt): close INC-A3-S1-C4-LINE-SHIFT (content-addressed allowlist) |
| `1244085` | chore(release): bump version 1.169.26 -> 1.169.27 |

## Carry-over debt

None. `open` debt in `docs/debt/` after this cycle: `INC-DEBT-023`
(lints advisory, low/P3) and `INC-DEBT-007`-class residuals — unrelated.

## Next roadmap items

Per `docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/09-ROADMAP.md`:

1. **AC6 — Critical mutation probes.** Natural next step: it drives AC4's
   `contradiction_witnesses` seam. Start with provider type leak,
   Alignment→Governance, workbook write, second canonical writer (AC-UAT-010).
2. **AC5 — DebVerify architecture audit.** Global challenge pass; must not be
   conflated with `verify --full` (AC-UAT-009).
3. **AC7 — OO/FP/ADT/DSL lens assessments.** Plugs into AC3's `LensAssessment`
   and AC4's `paradigm_alignment` vector dimension.

## Available seam for AC6

AC4 (`architecture_conformance`) accepts `contradiction_witnesses: &[ContractId]`
in its `ConformanceInputs`. AC6 can produce those witnesses by executing
deterministic mutation probes in a sandbox and feeding them into
`compute_conformance_delta`, closing the loop AC4 left open.
