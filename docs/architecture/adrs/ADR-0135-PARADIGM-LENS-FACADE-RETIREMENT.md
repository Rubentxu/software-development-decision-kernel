---
id: ADR-0135-PARADIGM-LENS-FACADE-RETIREMENT
status: accepted
supersedes_history: false
adopted_at: 2026-09-18
accepted_at: 2026-09-18
accepted_by_cycle: p-63676b11dc0ef88f/a5-4a-paradigm-lens-facade-retirement
---

# ADR-0135 — A5-4a: Retire `paradigm_lens::evaluate_lens()` Legacy Facade
- Supersedes: removal trigger in `ADR-0118-PARADIGM-LENS-EVALUATION`
  (post-acceptance amendment) and `ADR-0125-GENERIC-ALIGNMENT-LENS-KERNEL-REGISTRY`
  §"Post-acceptance amendment" entry 3.

## Context

A3-S8 introduced the `paradigm_lens` module with `evaluate_lens()` and a
typed `LensEvaluation` wrapper around the AC3 `LensAssessment`. A4-4M
explicitly demoted `evaluate_lens()` to a `LEGACY_READ_COMPAT` facade
whose body was zero-evaluation logic — it merely projected the
canonical substrate posture (`EvidencePosture`) onto the legacy
`LensStatus` vocabulary.

The closure receipt `A4-4C` (2026-09-17) deferred removal of the
facade to "a future A4-series or A4-CLOSEOUT cycle". A5-DEBT-DISPOSITION
§3.1 classified the facade as `MUST_CLOSE_A5 → DELETE` with
`NO_RUNTIME_CONSUMER` and the `LensEvaluation` type as
`MIGRATE_A5`. This ADR accepts A5-4a as the disposition cycle.

A5-5R closed the only external flake; `cargo test --workspace` was
green on `v1.169.81`. A5-4a executes the retirement against that
baseline.

## Decision

A5-4a retires `paradigm_lens::evaluate_lens()` and the legacy
`paradigm_lens::LensEvaluation` wrapper struct, with **no new
compatibility facade** introduced in their place.

Specifically:

1. **DELETE** `crates/sddk-engine/src/paradigm_lens/lenses.rs` (the
   file containing both `pub fn evaluate_lens` and
   `pub struct LensEvaluation`).
2. **DELETE** the re-exports `pub use lenses::{LensEvaluation,
   evaluate_lens}` from `paradigm_lens/mod.rs`.
3. **DELETE** the unit-test module `paradigm_lens/tests.rs` (16
   calls to the deleted facade, 100% legacy-only validation).
4. **DELETE** the integration test
   `crates/sddk-engine/tests/ac7_lens_over_ac3_profile.rs` (3
   tests verifying the legacy facade shape only).
5. **NARROW** the integration test
   `crates/sddk-engine/tests/a4_4m_convergence_pins.rs` from M7/M9
   legacy-shape pins to M3/M5/M10 textual + functional probes. The
   M7 corpus pins used to compare `legacy.assessment.status` to the
   substrate projection; without the facade, that comparison target
   is gone. The M7 coverage is delivered equivalently by
   `a4_4m_m0_migration_proof.rs` (substrate wiring proof) and
   `a4_4b_alignment_lens_kernel.rs` (kernel pathway).
6. **MIGRATE** the receipt composer's input field from
   `&'a [paradigm_lens::LensEvaluation]` to
   `&'a [paradigm_profile::LensAssessment]` — an info-preserving
   type narrowing. The composer only reads the five fields
   `(lens, anchor, status, basis, evidence_refs)` already present
   on `LensAssessment`; the AC7-specific wrapper fields
   (`basis: LensEvaluationBasis`, `provenance: LensProvenance`,
   `used_observations: Vec<LensObservation>`) are unreachable from
   the receipt path.
7. **REWRITE** the receipt-path integration test
   `ac8_full_chain_receipt.rs` and `architecture_receipt/tests.rs`
   to construct `LensAssessment` directly (no facade call).
8. **NO TOUCH** `paradigm_lens::probes`, `paradigm_lens::types`,
   `paradigm_lens::translation`. These are the production data
   path that the kernel `alignment_lens::paradigm` consumes as
   the **only** source of typed observations with provenance
   `ac7.probe.*`. Removing them would strand the kernel with no
   domain-specific observations. Their removal belongs to a
   separate future cycle that wires the kernel to a different
   observation producer.

## Disposition

```text
paradigm_lens::evaluate_lens()
  OBSOLETE → DELETE
  evidence: zero production call sites;
            3+ test consumers, all retired or migrated.

paradigm_lens::LensEvaluation
  OBSOLETE → DELETE
  evidence: sole production consumer is the receipt composer's
            input field; it received `lenses: &[]` at runtime.
            No disk serialization used the type.
            Migrated to `&[paradigm_profile::LensAssessment]`.

paradigm_lens::types (and its 7 types)
  KEPT (required for kernel data path)

paradigm_lens::probes (4 functions)
  KEPT (required for kernel data path)

paradigm_lens::translation (2 functions)
  KEPT (required for kernel data path)
```

## Canonical authority after A5-4a

```text
AlignmentLensRegistry
        ↓
AlignmentLensKernel
        ↓
LensEvaluation { contributions, gaps }      ← canon

paradigm_lens::probe_*                       ← evidence source (KEPT)
        ↓
paradigm_lens::translation                  ← bridge to substrate (KEPT)
        ↓
SoftwareObservation (observation substrate) ← the only substrate
        ↓
AlignmentLensKernel.evaluate                ← single evaluation authority
```

The legacy `paradigm_lens::lenses` module is gone. There is no
second evaluator.

## Constraints respected

- **No new compatibility facade.** `lenses.rs` is deleted whole;
  no thin shim remains.
- **No behaviour added.** The composer, kernel, lens registry and
  probe outputs are unchanged. Only one production-public type
  signature (`ReceiptInputs.lenses`) narrowed, preserving every
  observable downstream.
- **No unrelated A5 debt pulled in.** FU-A3-CO-1, FU-A3-CO-3,
  FU-A3-S15-4, ASC-MA-1, R12, R1, Authority and providers were
  not touched. Confirmed by file-level diff audit.
- **Scope prohibited items left alone.** All A5 scope-prohibited
  lines left unmodified. Confirmed by textual preflight audit.
- **Legacy M0–M9 milestones left intact.** No roadmap items opened
  or closed.

## Verification budget

Full workspace gate runs after the change:

```text
cargo fmt --all -- --check           exit 0
cargo clippy -p sddk-cli             exit 0
cargo clippy --workspace --all-targets -- -D warnings
                                   exit 0
cargo test --workspace
   passed=4729 failed=0 ignored=13
   (vs. baseline 4763/0/18 — delta is legacy-only tests)
```

Public release gate (step 9b) is required. Pre-push hook allows
the `chore(release): bump version` commit. Release runs **without
`--skip-tests`** per the A5-5R achievement.

## Out of scope (DEFERRED)

- `paradigm_lens::types` (and `LensError`, `family_for_kind`,
  `ObservationPolarity`, `LensFamily`, `LensObservation`,
  `LensEvaluationBasis`, `LensProvenance`): still required by
  probe + translation.
- `paradigm_lens::probes` (4 fns): the only domain-specific
  evidence emitter.
- `paradigm_lens::translation` (2 fns): the only bridge to
  `SoftwareObservation`.
- `AlignmentLensKernel::evaluate` simplification, kernel-aggregate
  ergonomics, advisory-ledger consolidation.
- Anything in scope-prohibited list.

These are real deletions to consider in a future cycle, once the
kernel data path is rewired to emit typed observations directly.
That is a non-trivial kernel change with its own ADR; this slice
deliberately stops at the legacy facade boundary so as to "do one
thing" per `AGENTS.md` rules.
