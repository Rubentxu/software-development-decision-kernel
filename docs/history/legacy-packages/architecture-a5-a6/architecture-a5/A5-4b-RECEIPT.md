# A5-4b-RECEIPT — Bounded Compatibility, Lints & Operator UX Closure

| field | value |
|---|---|
| cycle | `p-63676b11dc0ef88f/a5-4b-bounded-compat-lints-operator-ux` |
| slice | A5-4b (single-risk budget) |
| scope | FU-A3-CO-1, FU-A3-CO-3, FU-A3-S15-4, ASC-MA-1, 3 advisory `allow` lints, `EvidenceAttachmentV1` carry-forward |
| status | CLOSED (4 MUST_CLOSE items + 3 advisory pins); 1 carry-forward |
| baseline | `v1.169.82` (`48115a9`) |
| target | `v1.169.83` |
| ADR | `ADR-0136-A5-4B-BOUNDED-COMPAT-LINTS-OPERATOR-UX-CLOSURE` |
| pin test | `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs` |

## Disposition (single decision per item)

```text
FU-A3-CO-1 (relation payload encoding)        KEEP_WITH_REASON
  evidence: KnowledgePayload::Relation exists in
            crates/sddk-knowledge/src/... with zero production
            writers; A4-4 already closed the lens-shape concern
            independently. No alternate encoding to close. Recorded
            so a future cycle does not re-open it as orphaned work.

FU-A3-CO-3 (rename/shape cleanup)             CLOSED_BY_PRIOR_WORK
  evidence: SpecifiedBy is the one canonical relation post-A4-S15R
            (v1.169.65); VerifiedBy was repointed to EvidenceRef in
            the same cycle. No residual rename/shape to perform.

FU-A3-S15-4 (fitness rule / CLI lint /       CLOSED_BY_PRIOR_WORK
            doctor conversion)
  evidence: TOML-driven lint at
            docs/architecture/lints/deprecated_patterns.toml +
            implementation in
            crates/sddk-cli/src/dev/lint/deprecated_patterns.rs.
            Zero duplicate implementations across lint / doctor /
            release (one source, many presentations, per ADR-0134).

ASC-MA-1 (sddk --help UX pass)                CLOSE_BY_HELP
  evidence: crates/sddk-cli/src/lib.rs:189 about-line changed
            from
              "Deterministic SDDK workflow tooling — First-class
               commands: status, plan, run, ship, recover, memory"
            to
              "Deterministic SDDK workflow tooling — uses
               `sddk agent-help` for the operator-facing surface".
            5 of 6 commands in the old substring were M6.1 legacy
            facades, not first-class. Pin tests assert BOTH absence
            of the legacy substring AND presence of the new
            `sddk agent-help` substring:
              - first_class_commands::help_drops_first_class_substring
              - cli_first_class_help::help_drops_first_class_substring
              - cli_golden::sddk_help_matches_snapshot (54 → 58 lines)
              - cli_compatibility::top_level_help_matches_snapshot
                (byte-comparison stderr fixture, 4148 bytes)
              - cli_compatibility::uat_help_matches_snapshot

execution_outcome_as_synthesis (allow, 0)     KEEP_ALLOW_WITH_REASON
  evidence: audit-cleared; corpus expansion organic; registry
            carries explanation = """ ... """ body > 10 tokens
            (registry-pinned, machine-enforced by
            a5_4b_lint_disposition_pin::a5_4b_allow_lints_have_
            default_allow_and_non_empty_explanation).

transition_outcome_used (allow, 24)           KEEP_ALLOW_WITH_REASON
  evidence: state-machine regression guard, M9.2 closed; 24
            legitimate hits are intentional guards. Registry
            explanation body > 10 tokens (machine-pinned).

asset_unregistered_cli_example (allow, 0)     KEEP_ALLOW_WITH_REASON
  evidence: regex unsafe-by-design (CLI examples do not always
            carry asset registry IDs); AX-S1 pin; explanation
            body > 10 tokens (machine-pinned).

EvidenceAttachmentV1 + compat decoder         STOP_NEEDS_SEPARATE_SLICE
  evidence: anti-encroachment for A5-4b prohibits
            EvidencePosture / Verify / DebVerify / Alignment /
            IntelligenceLoop / AdvisoryWhy / UniversalConcern
            semantics changes. The compat decoder + ratchet
            exclude + storage migration are C2.5 risk class and
            cannot be absorbed into a single-risk A5-4b cycle.
            See A5-DEFERRED-POST-BASE.md for the follow-up.
```

## Files touched

```text
M  crates/sddk-cli/src/lib.rs                                  (about-line)
M  crates/sddk-cli/tests/first_class_commands.rs               (pin test rewrite)
M  crates/sddk-cli/tests/cli_first_class_help.rs               (pin test rewrite)
M  docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-help.txt
                                                              (54 → 58 lines)
M  docs/architecture/tests/fixtures/cli_golden/README.md       (line count 54 → 58)
M  crates/sddk-cli/tests/fixtures/cli/help-top-level.txt       (4148 bytes,
                                                              stderr snapshot)
A  crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs         (3 pin tests,
                                                              new file)
M  docs/architecture/a5/A5-DEBT-DISPOSITION.md                 (added §3.7)
A  docs/architecture/a5/A5-4b-RECEIPT.md                       (this file)
A  docs/architecture/adrs/ADR-0136-A5-4B-BOUNDED-COMPAT-LINTS-OPERATOR-UX-CLOSURE.md
A  docs/handoff/HANDOFF-2026-09-18-a5-4b-session-close.md
M  Cargo.toml                                                  (version bump 1.169.82 → 1.169.83)
```

## Falsification matrix (10 pins)

```text
1. lib.rs:189 about-line no longer mentions status/plan/run/ship/
   recover/memory as "first-class commands"                      ✓ PASS
2. lib.rs:189 about-line points operators to `sddk agent-help`  ✓ PASS
3. first_class_commands::help_drops_first_class_substring green  ✓ PASS (1/1)
4. cli_first_class_help::help_drops_first_class_substring green  ✓ PASS (1/1)
5. cli_golden::sddk_help_matches_snapshot green (58 lines)       ✓ PASS (1/1)
6. cli_compatibility::top_level_help_matches_snapshot green
   (stderr fixture 4148 bytes)                                  ✓ PASS (1/1)
7. cli_compatibility::uat_help_matches_snapshot green            ✓ PASS (1/1)
8. a5_4b_lint_disposition_pin::a5_4b_allow_lints_have_
   default_allow_and_non_empty_explanation green (3/3 lints)    ✓ PASS
9. a5_4b_lint_disposition_pin::a5_4b_no_new_allow_lints_
   added_silently green (3 = ALLOW_LINTS.len())                  ✓ PASS
10. a5_4b_lint_disposition_pin::a5_4b_denied_lints_have_
    zero_illegitimate_hits_at_v1_169_83 green (corridor frozen) ✓ PASS
```

## Anti-encroachment (constraint audit)

```text
relation payload encoding                                  NOT TOUCHED
FU-A3-CO-1                                                 KEEP_WITH_REASON (recorded)
FU-A3-CO-3                                                 CLOSED_BY_PRIOR_WORK
FU-A3-S15-4                                                CLOSED_BY_PRIOR_WORK
ASC-MA-1                                                   CLOSE_BY_HELP
EvidenceAttachmentV1                                       STOP_NEEDS_SEPARATE_SLICE
R12 (sender-drop)                                          NOT TOUCHED
R1  (restart-survival)                                     NOT TOUCHED
EvidencePosture / Verify / DebVerify / Alignment /        NOT TOUCHED
  IntelligenceLoop / AdvisoryWhy / UniversalConcern
Authority (a6_)                                            NOT TOUCHED
providers                                                   NOT TOUCHED
M0–M9 roadmap restructuring                                NOT TOUCHED
security / secrets                                         NOT TOUCHED
semantic freeze                                            NOT TOUCHED
```

Confirmed by textual diff audit pre/post (no scope-creep signals).

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
   All prior tests remain green; no regression.
```

## Lint corpus snapshot (post A5-4b)

```text
agent_result_used                       deny     0
asset_authority_language                deny     0
asset_deprecated_namespace              deny     0
asset_raw_store_reference               deny     0
asset_unregistered_cli_example          allow    0   ← KEEP_ALLOW_WITH_REASON
evidence_kind_v1                        deny     0
execution_outcome_as_synthesis          allow    0   ← KEEP_ALLOW_WITH_REASON
orchestration_synthesis_no_dissent      deny     0
transition_outcome_used                 allow    24  ← KEEP_ALLOW_WITH_REASON
```

All three `allow` lints carry registry-pinned `explanation = """..."""`
bodies of >10 non-whitespace tokens. Future cycles cannot add a new
allow lint or remove the explanation without tripping
`a5_4b_no_new_allow_lints_added_silently`.

## What did NOT happen (honor bound)

- No new compatibility shim was introduced. The `KnowledgePayload::Relation`
  variant is left in place but flagged `KEEP_WITH_REASON` so the
  disposition is visible (not silently forgotten).
- The about-line is informational only. It points operators at
  `sddk agent-help`, the canonical operator-facing surface. The
  five legacy facade commands (`status`, `plan`, `run`, `ship`,
  `recover`, `memory`) remain available but no longer carry
  the "first-class" label.
- No semantic change to `EvidencePosture`, `Verify`, `DebVerify`,
  `Alignment`, `IntelligenceLoop`, `AdvisoryWhy`, `UniversalConcern`,
  or any other data-flow type.
- No PRD / roadmap delta: M0–M9 milestones remain untouched.
- `EvidenceAttachmentV1` compat decoder is left alone, not stripped.
  Migration goes to a separate slice (anti-encroachment).

## Debt ledger

```text
FU-A3-CO-1                                KEEP_WITH_REASON       (MUST_CLOSE_A5 → disposed)
FU-A3-CO-3                                CLOSED_BY_PRIOR_WORK   (MUST_CLOSE_A5 → disposed)
FU-A3-S15-4                               CLOSED_BY_PRIOR_WORK   (MUST_CLOSE_A5 → disposed)
ASC-MA-1                                  CLOSE_BY_HELP          (MUST_CLOSE_A5 → disposed)
execution_outcome_as_synthesis            KEEP_ALLOW_WITH_REASON (machine-pinned)
transition_outcome_used                   KEEP_ALLOW_WITH_REASON (machine-pinned)
asset_unregistered_cli_example            KEEP_ALLOW_WITH_REASON (machine-pinned)

DEFERRED (out of this slice):
  EvidenceAttachmentV1 + compat decoder   STOP_NEEDS_SEPARATE_SLICE (see A5-DEFERRED-POST-BASE.md)
```
