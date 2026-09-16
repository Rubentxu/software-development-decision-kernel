# Handoff: A4-4b — Generic AlignmentLens kernel/registry → v1.169.56

**Date:** 2026-09-16
**Cycle:** `p-63676b11dc0ef88f/a4-4b-alignment-lens-kernel`
**Status:** feat/test green; release pending (commit `chore(release)` next).

## Scope

A4-4b is **ABSTRACTION/KERNEL ONLY**. It introduces a paradigm-agnostic
`AlignmentLens` trait, `AlignmentLensRegistry`, content-addressed
`LensContributionId`, and a total-function `AlignmentLensKernel` that
returns a typed `NotEvaluated` gap distinct from `NotApplicable`.

The kernel reuses `observation::EvidenceResolution` and adds a typed
`InsufficientGap` enum so the free-form `gap: String` of `Insufficient`
cannot reach the contribution id.

The kernel does **not** call `software_alignment::reduce_alignment`,
does **not** touch `paradigm_lens/`, and ships **no** production
concrete OO/FP/ADT/DSL lens impls.

## What landed

### Commits (this cycle)

| # | Hash  | Subject |
|---|-------|---------|
| 1 | 14819e7 | feat(engine): A4-4b — generic AlignmentLens kernel/registry |
| 2 | 4b81120 | test(engine): A4-4b — pin tests + reference fixture lenses |
| 3 | (this commit) | docs: ADR-0125 + handoff + roadmap delta |

### Source

- `crates/sddk-engine/src/alignment_lens/` (9 files, ~1750 lines):
  types, id (LensContributionId), lens trait, registry, error, kernel,
  contribution, not_evaluated, internal tests.
- `crates/sddk-engine/src/lib.rs`: adds `pub mod alignment_lens;` only.

### Tests

- `crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs`:
  24 pin tests (structural, identity, type-locked
  anti-encroachment, boundary).
- `crates/sddk-engine/tests/alignment_lens_fixture.rs`: two
  heterogeneous reference lenses (DependencyDirection, Freshness).
  Test-only; NOT exported as production API.

### ADR

- `docs/architecture/adrs/ADR-0125-GENERIC-ALIGNMENT-LENS-KERNEL-REGISTRY.md`
  (accepted).

## Gate results (all green)

| Gate | Result |
|------|--------|
| `cargo build -p sddk-engine` | clean |
| `cargo build -p sddk-engine --tests` | clean (no warnings) |
| `cargo test -p sddk-engine --test a4_4b_alignment_lens_kernel` | 24/24 pass |
| `cargo test -p sddk-engine --lib` | 1245/1245 pass |
| `cargo test --workspace --offline` | all pass |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo build --release -p sddk-cli --bin sddk` | clean |
| `bash tests/test_release_public_gate.sh` | PASS=11 FAIL=0 |

## Kernel design contract (locked)

1. `ApplicableConcern + no registered lens` → `NotEvaluated { reason: NoRegisteredLens }`.
2. `ApplicableConcern + lens + insufficient` → `EvidenceResolution::Insufficient` (carries typed `InsufficientGap`).
3. `ApplicableConcern + lens + contradicted-or-conflicted` → preserve both sides (`Conflicted` carries supporting AND contradicting).
4. `LensVersion` is in identity.
5. `LensInput::try_new` refuses `NotApplicable` at the boundary; the kernel never sees `NotApplicable`.
6. Duplicate `LensId` with same `LensVersion` + same `supported_concerns` → idempotent success.
7. Duplicate `LensId` with any divergence → typed refusal (`InconsistentSupportedConcerns` / `InconsistentLensVersion`).
8. Identity (`LensContributionId`) excludes wall clock, message text, registration order, vector insertion order, severity, producer labels.
9. The free-form `gap: String` of `EvidenceResolution::Insufficient` does **not** reach identity; only `InsufficientGap` enum values do.

## Anti-encroachment pins (structural, not aspirational)

- `pin_11_no_score_or_confidence_field` — `LensContribution` has no `f64` field.
- `pin_12_no_alignment_state_field` — no `AlignmentState` in the kernel.
- `pin_13_no_authority_engine_dependency` — kernel does not import authority.
- `pin_14_no_capability_field` — no `Capability` field.
- `pin_15_no_instruction_compiler_dependency` — no instruction compiler.
- `pin_16_no_provider_sdk_dependency` — no provider SDK import.
- `pin_17_no_cli_surface_change` — no CLI binary touched.
- `pin_18_no_production_concrete_alignment_lens_impl` — fixture lenses are under `tests/`, not `src/`.
- `pin_19_kernel_does_not_depend_on_paradigm_lens` — no `paradigm_lens::` import.
- `pin_a2_does_not_call_reduce_alignment_textual_probe` — textual grep, doc-comments excluded.

## Debt notes

- `FU-A4-3-CONSTRAINT-BINDING` (P1) remains registered; **not** fixed in A4-4b.
- No FU items closed in this cycle.
- No debt introduced.

## Cycle spec compliance

The cycle spec at
`.sddk/cycles/p-63676b11dc0ef88f-a4-4b-alignment-lens-kernel/spec.md`
required: 14 MUST, 15 MUST_NOT, 20 pin tests, ≤3 commit budget.

- MUST (14): all satisfied.
- MUST_NOT (15): all satisfied (anti-encroachment pins).
- Pin tests: 24 delivered (≥20 required).
- Commits so far: 3 (feat, test, docs) — within budget.

## Roadmap delta

A4-4b is the **CURRENT** milestone. Status:

- A4-4aR: `closed v1.169.54`.
- A4-4b: `CURRENT` (this cycle).
- A4-4M: `blocked_by A4-4b`.
- A4-4C: `blocked_by A4-4M`.
- A4-5: `blocked_by A4-4C`.

`docs/architecture/README.md` was updated in the preflight commit
(`a54f016`).

## Release pending

Next commits (after this handoff lands):

1. `chore(release): bump version 1.169.56 → 1.169.57`.
2. `bash scripts/release.sh` for v1.169.56 release (must pass step 9b
   `PublicReleaseGate` as a regression test).

## STOP — Do NOT auto-open A4-4M

A4-4M is the next milestone but is **blocked** on user green-light.
Do not start work on A4-4M until the user explicitly opens it.
