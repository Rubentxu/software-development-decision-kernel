# A5-4a-RECEIPT — Retire `paradigm_lens::evaluate_lens()` and `LensEvaluation`

| field | value |
|---|---|
| cycle | `p-63676b11dc0ef88f/a5-4a-paradigm-lens-facade-retirement` |
| slice | A5-4a (narrow, single budget) |
| scope | `paradigm_lens::evaluate_lens`, `paradigm_lens::LensEvaluation` |
| status | CLOSED / DELETED |
| baseline | `v1.169.81` (`c7e153fcc0f979550c4d6ccb183cd64bfb50da16`) |
| target | `v1.169.82` |
| ADR | `ADR-0135-PARADIGM-LENS-FACADE-RETIREMENT` |

## Disposition (single decision per item)

```text
paradigm_lens::evaluate_lens
  OBSOLETE → DELETE
  evidence: zero production call sites; consumer scope is 3 test
            files (paradigm_lens/tests.rs unit,
            ac7_lens_over_ac3_profile.rs, a4_4m_convergence_pins.rs
            m7_* + m9_* pins) and the A4-5b/A4-5c textual probes
            (comment-only references). Of those, the integration
            files were retired or migrated to kernel-friendly
            equivalents; the unit tests removed with the
            paradigm_lens/tests.rs module.

paradigm_lens::LensEvaluation
  OBSOLETE → DELETE
  evidence: the only production site referencing this type was
            architecture_receipt::compose::ReceiptInputs.lenses;
            the composer received `lenses: &[]` at runtime;
            its only field readers were the AC3-shaped projection
            (lens, anchor, status, basis, evidence_refs) which
            is now satisfied by `paradigm_profile::LensAssessment`.
            Type narrowed (info-preserving).

paradigm_lens::types     KEPT  (required by translation + probes)
paradigm_lens::probes    KEPT  (required by translation)
paradigm_lens::translation KEPT (required by alignment_lens::paradigm)
```

## Files touched

```text
D  crates/sddk-engine/src/paradigm_lens/lenses.rs        (facade deleted)
D  crates/sddk-engine/src/paradigm_lens/tests.rs          (16 legacy unit tests)
D  crates/sddk-engine/tests/ac7_lens_over_ac3_profile.rs  (3 legacy integration tests)
M  crates/sddk-engine/src/paradigm_lens/mod.rs            (re-exports removed; doc rewritten)
M  crates/sddk-engine/src/architecture_receipt/compose.rs (ReceiptInputs.lenses: &[LensAssessment])
M  crates/sddk-engine/src/architecture_receipt/tests.rs   (3 LensAssessment constructions)
M  crates/sddk-engine/tests/ac8_full_chain_receipt.rs     (1 LensAssessment construction)
M  crates/sddk-engine/tests/a4_4m_convergence_pins.rs     (narrowed to M3/M5/M10)
M  Cargo.toml                                             (version bump 1.169.81 → 1.169.82)
A  docs/architecture/adrs/ADR-0135-PARADIGM-LENS-FACADE-RETIREMENT.md
```

## Falsification matrix (10 pins)

```text
1. no production references evaluate_lens()                    ✓ PASS
2. no public runtime path reaches evaluate_lens()              ✓ PASS
3. production ParadigmLens still reads substrate (m10 pin)     ✓ PASS
4. LensInput.concern == LensContribution.concern               ✓ PASS (unchanged)
   (kernel pathway invariant, orthogonal to facade removal)
5. gaps NotEvaluated still preserved                           ✓ PASS (kernel pathway)
6. A4-5a composition still uses kernel productive              ✓ PASS (unchanged)
7. A4-5b advisory still works                                  ✓ PASS (unchanged)
8. historical receipts readable if compat needed               N/A (no historical
                                                              serializer writes
                                                              LensEvaluation to disk)
9. no new compatibility facade introduced                      ✓ PASS
10. no Alignment status/contribution/evidence semantics change ✓ PASS
```

## Anti-encroachment (constraint audit)

```text
relation payload encoding                                  NOT TOUCHED
FU-A3-CO-1                                                 NOT TOUCHED
FU-A3-CO-3                                                 NOT TOUCHED
FU-A3-S15-4                                                NOT TOUCHED
ASC-MA-1                                                   NOT TOUCHED
R12 (sender-drop)                                          NOT TOUCHED
R1  (restart-survival)                                     NOT TOUCHED
Authority (a6_)                                           NOT TOUCHED
providers                                                  NOT TOUCHED
M0–M9 roadmap restructuring                                NOT TOUCHED
```

Confirmed by textual diff audit pre/post (no scope-creep signals).

## Verification

```text
cargo build --tests                exit 0   (clean)
cargo fmt --all -- --check         exit 0
cargo clippy --workspace --all-targets -- -D warnings
                                  exit 0
cargo test --workspace
   passed=4729 failed=0 ignored=13
   (pre-A5-4a: passed=4763 failed=0 ignored=18)
   delta=-34 passed, -5 ignored: legacy m7_* pins (33) + unit
   tests in paradigm_lens/tests.rs (16 were a subset of those
   m7 pins across repetitions; ac7 integration test = 3; etc.).
   Coverage preserved by a4_4b_alignment_lens_kernel.rs,
   a4_4m_m0_migration_proof.rs, a4_5a_intelligence_loop_composition.rs.
```

## Canonical authority (post A5-4a)

```text
AlignmentLensRegistry
        ↓
AlignmentLensKernel
        ↓
KernelOutcome::Ok(LensEvaluation { contributions, gaps })   ← single evaluator

paradigm_lens::probe_*                                       ← evidence source
        ↓
paradigm_lens::translation                                  ← bridge to substrate
        ↓
SoftwareObservation (canonical observation substrate)
        ↓
AlignmentLensKernel.evaluate                                ← single motor
```

There is no second evaluator.

## What did NOT happen (honor bound)

- No new compatibility shim was introduced. The deleted file is
  gone, not replaced.
- The composer field was narrowed (info-preserving) to consume
  `LensAssessment`, not bridged via a `LensEvaluation → something`
  conversion function.
- No semantic change to `LensStatus`, `Alignment`, `LensContribution`,
  `EvidencePosture`, `SoftwareObservation`, or any other data-flow
  type. The kernel is the only producer of advisory lens outputs.
- No PRD / roadmap delta: M0–M9 milestones remain untouched.

## Debt ledger

```text
paradigm_lens::evaluate_lens()         CLOSED / DELETED  (was MUST_CLOSE_A5)
paradigm_lens::LensEvaluation type     OBSOLETE → DELETE (was MIGRATE_A5; reclassified)

DEFERRED (out of this slice):
  paradigm_lens::types                  requires kernel data path migration
  paradigm_lens::probes                 requires kernel data path migration
  paradigm_lens::translation            requires kernel data path migration
```
