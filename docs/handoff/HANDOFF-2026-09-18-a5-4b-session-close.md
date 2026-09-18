# HANDOFF-2026-09-18-a5-4b-session-close

> Cycle: `p-63676b11dc0ef88f/a5-4b-bounded-compat-lints-operator-ux`
> Status: **CLOSED / RELEASED v1.169.83**
> Prior handoff: `HANDOFF-2026-09-15-session-close.md` (cycle-46 install coherence)
> Companion: `HANDOFF-2026-08-26-sddk-framework.md` (long-running state)

## What this cycle did

A5-4b is a **bounded-compatibility** cycle. It disposes 4
`MUST_CLOSE_A5` items, pins 3 advisory `allow` lints as
`KEEP_ALLOW_WITH_REASON` with machine-readable registry-pinned
explanations, and carries `EvidenceAttachmentV1` forward to a
separate slice.

| Item | Disposition |
|---|---|
| `FU-A3-CO-1` (relation payload encoding) | KEEP_WITH_REASON |
| `FU-A3-CO-3` (rename/shape cleanup) | CLOSED_BY_PRIOR_WORK |
| `FU-A3-S15-4` (fitness/CLI lint/doctor) | CLOSED_BY_PRIOR_WORK |
| `ASC-MA-1` (`sddk --help` UX pass) | CLOSE_BY_HELP |
| `execution_outcome_as_synthesis` (allow) | KEEP_ALLOW_WITH_REASON |
| `transition_outcome_used` (allow) | KEEP_ALLOW_WITH_REASON |
| `asset_unregistered_cli_example` (allow) | KEEP_ALLOW_WITH_REASON |
| `EvidenceAttachmentV1` + compat decoder | STOP_NEEDS_SEPARATE_SLICE |

The about-line at `crates/sddk-cli/src/lib.rs:189` was changed
from "First-class commands: status, plan, run, ship, recover,
memory" (5/6 M6.1 legacy facades) to "uses `sddk agent-help`
for the operator-facing surface".

Two pin tests verify BOTH absence of the legacy substring AND
presence of the new `sddk agent-help` substring:

- `crates/sddk-cli/tests/first_class_commands.rs::help_drops_first_class_substring`
- `crates/sddk-cli/tests/cli_first_class_help.rs::help_drops_first_class_substring`

Two golden fixtures were regenerated:

- `docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-help.txt`
  (54 → 58 lines; README updated).
- `crates/sddk-cli/tests/fixtures/cli/help-top-level.txt`
  (4148-byte stderr snapshot for `cli_compatibility::top_level_help_matches_snapshot`).

A new test file
`crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs` (3 tests)
machine-pins the disposition of the three `allow` lints:

1. `a5_4b_allow_lints_have_default_allow_and_non_empty_explanation`
   — each allow entry has `default = "allow"` plus a registry-
   pinned explanation body of > 10 tokens.
2. `a5_4b_no_new_allow_lints_added_silently` — enforces
   `allow_blocks_in_registry == ALLOW_LINTS.len()` (3).
3. `a5_4b_denied_lints_have_zero_illegitimate_hits_at_v1_169_83`
   — freezes the deny-lint corridor (6 lints at 0 hits each).

## What did NOT happen (honor bound)

- No semantic-freeze change. No `EvidencePosture` / `Verify` /
  `DebVerify` / `Alignment` / `IntelligenceLoop` / `AdvisoryWhy`
  / `UniversalConcern` semantics touched.
- No Authority / providers / M0–M9 restructuring.
- No security / secrets change.
- No resurrection of deleted symbols. `evaluate_lens` is gone;
  the pin tests assert both absence of the legacy substring AND
  presence of the new `sddk agent-help` substring.

## Verification

```text
cargo build --tests                             exit 0
cargo fmt --all -- --check                      exit 0
cargo clippy --workspace --all-targets -- -D warnings
                                               exit 0
cargo test --workspace
   passed=4732 failed=0 ignored=13
   (pre-A5-4b: passed=4729 failed=0 ignored=13)
   delta=+3 passed: a5_4b_lint_disposition_pin (3 new pin tests)

bash tests/test_adr_promotion_format.sh         exit 0 (44 ADRs, 0 violations)
sddk dev lint deprecated-patterns --format json exit 0 (9 lints, 0 deny hits)
```

## Files touched

```text
M  crates/sddk-cli/src/lib.rs                                  (about-line)
M  crates/sddk-cli/tests/first_class_commands.rs               (pin test rewrite)
M  crates/sddk-cli/tests/cli_first_class_help.rs               (pin test rewrite)
M  docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-help.txt
                                                              (54 → 58 lines)
M  docs/architecture/tests/fixtures/cli_golden/README.md       (line count)
M  crates/sddk-cli/tests/fixtures/cli/help-top-level.txt       (4148 bytes,
                                                              stderr snapshot)
A  crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs         (3 pin tests)
M  docs/architecture/a5/A5-DEBT-DISPOSITION.md                 (added §3.7)
A  docs/architecture/a5/A5-4b-RECEIPT.md                       (new receipt)
A  docs/architecture/adrs/ADR-0136-A5-4B-BOUNDED-COMPAT-LINTS-OPERATOR-UX-CLOSURE.md
A  docs/handoff/HANDOFF-2026-09-18-a5-4b-session-close.md      (this file)
M  Cargo.toml                                                  (1.169.82 → 1.169.83)
```

## Release

- Version: `v1.169.83`
- ADR: `ADR-0136`
- Receipt: `docs/architecture/a5/A5-4b-RECEIPT.md`
- Disposition: `docs/architecture/a5/A5-DEBT-DISPOSITION.md` §3.7
- Mirror: `python3 scripts/mirror_adrs_to_vault.py` (canonical, not raw cp)

## Anti-encroachment confirmation

A5-4b is bounded to one risk class. It did not touch:

| Class | Status |
|---|---|
| `EvidencePosture` semantics | NOT TOUCHED |
| `Verify` semantics | NOT TOUCHED |
| `DebVerify` semantics | NOT TOUCHED |
| `Alignment` semantics | NOT TOUCHED |
| `IntelligenceLoop` semantics | NOT TOUCHED |
| `AdvisoryWhy` semantics | NOT TOUCHED |
| `UniversalConcern` semantics | NOT TOUCHED |
| Authority (a6_) | NOT TOUCHED |
| Providers | NOT TOUCHED |
| M0–M9 roadmap | NOT TOUCHED |
| Security / secrets | NOT TOUCHED |
| Semantic freeze | NOT TOUCHED |
| `EvidenceAttachmentV1` (compat decoder) | NOT TOUCHED — STOP_NEEDS_SEPARATE_SLICE |

## Carry-forward

- `EvidenceAttachmentV1` + compat decoder → separate slice.
  Tracked under `A5-DEFERRED-POST-BASE.md`.

## Next step

A5 plan continues. A5-4c and A5-C are not auto-opened.
Awaiting explicit user green-light.

## Reference

- `docs/architecture/a5/A5-DEBT-DISPOSITION.md` §3.7
- `docs/architecture/a5/A5-4b-RECEIPT.md`
- `docs/architecture/adrs/ADR-0136-A5-4B-BOUNDED-COMPAT-LINTS-OPERATOR-UX-CLOSURE.md`
- `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs`
- `docs/handoff/HANDOFF-2026-09-15-session-close.md` (prior)
- `docs/handoff/HANDOFF-2026-08-26-sddk-framework.md` (long-running state)
