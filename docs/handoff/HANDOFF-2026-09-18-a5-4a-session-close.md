# Handoff — A5-4a — Retire paradigm_lens::evaluate_lens() (2026-09-18)

## State at close

```text
released_baseline:
  v1.169.82
  (this handoff cycle's release tag, target)

development_head:
  (this handoff commit, ahead of the version bump)
```

`v1.169.82` carries the retirement of the legacy
`paradigm_lens::evaluate_lens()` function and the legacy
`paradigm_lens::LensEvaluation` wrapper struct, with **no new
compatibility shim** introduced.

## What landed

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
A  docs/architecture/a5/A5-4a-RECEIPT.md
M  docs/architecture/a5/A5-DEBT-DISPOSITION.md            (mark CLOSED_A5 in §3.1 + §3.5)
```

## Disposition summary

| Item | Decision | Evidence |
|---|---|---|
| `paradigm_lens::evaluate_lens()` | **OBSOLETE → DELETE** | zero production callers; 3 test files retired/migrated |
| `paradigm_lens::LensEvaluation` | **OBSOLETE → DELETE** | sole production site is receipt composer's input field, which receives `lenses: &[]` at runtime |
| `paradigm_lens::types`, `probes`, `translation` | **KEPT** | required by `alignment_lens::paradigm` (production kernel data path) |

The composer field `ReceiptInputs.lenses` was narrowed from
`&'a [paradigm_lens::LensEvaluation]` to
`&'a [paradigm_profile::LensAssessment]` — an info-preserving
type narrowing (the composer only reads the five fields it
needed from the wrapper).

## Honest limits carried over

- `paradigm_lens::types`, `probes`, `translation` remain.
  These are the only domain-specific observation producer for the
  canonical kernel. Removing them would strand the kernel with no
  AC7 observations. Their retirement belongs to a future cycle
  that rewires the kernel to emit its own typed observations —
  that is a kernel-data-path change, not a facade retirement.
- The textual-style probes (`probe_oo_observations`, etc.) live
  in `paradigm_lens::probes` and continue to be the ONLY
  evidence source for `AlignmentLensKernel::evaluate`-driven
  production advisory lenses.
- No behavior was added. No semantics of `Alignment`,
  `LensContribution`, `EvidencePosture`, `SoftwareObservation`
  changed.

## Verification budget

```text
cargo build --tests                          exit 0
cargo fmt --all -- --check                   exit 0
cargo clippy --workspace --all-targets -- -D warnings
                                            exit 0
cargo test --workspace
   passed=4729 failed=0 ignored=13
   (vs. baseline 4763/0/18)
cargo test -p sddk-engine
   all green
```

## Anti-encroachment

Out-of-scope items confirmed untouched:

```text
FU-A3-CO-1, FU-A3-CO-3, FU-A3-S15-4, ASC-MA-1, R12, R1
Authority (a6_*)
providers
M0–M9 milestones
```

## Roadmap delta

- `paradigm_lens::evaluate_lens` **DELETED** → removes 1 MUST_CLOSE_A5 item.
- `paradigm_lens::LensEvaluation` **DELETED** → resolves the
  `MIGRATE_A5` ambiguity that §3.1 carried; the migration
  target is `paradigm_profile::LensAssessment` (the AC3 shape).
- Remaining A5 MUST_CLOSE items: FU-A3-CO-1, FU-A3-CO-3,
  FU-A3-S15-4, ASC-MA-1. Two ignored-test signals still open:
  R1 (`restart-survival`), R12 (`sender-drop`).
- Remainder of M0–M9 untouched (deferred to future cycles).

## What's next (NOT to be opened automatically)

```text
A5-4b  FU-A3-CO-1 / FU-A3-CO-3 / FU-A3-S15-4 / ASC-MA-1 + the
       lint/doctor UX conversion — one bounded cycle.
R1     restart-survival
R12    parallel sender-drop

M0–M9  untouched
```

A5-4a did **not** auto-open any of these. Next session starts
with ROADMAP-CLOSEOUT AUDIT and user green-light.
