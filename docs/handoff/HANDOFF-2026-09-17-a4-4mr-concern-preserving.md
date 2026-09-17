# A4-4MR — Concern-Preserving ParadigmLens Evaluation

> Cycle: `p-63676b11dc0ef88f-a4-4mr-concern-preserving`
> Closes: `docs/debt/FU-A4-4M-CONCERN-PRESERVATION.md` (P1)
> Single change budget: **PRESERVE INPUT CONCERN THROUGH PRODUCTION LENS**
> Successor: A4-5a (Intelligence Loop composition) is **structurally
> unblocked** by this cycle's release but **NOT auto-opened** per the
> per-cycle STOP rule.

## TL;DR

Production `ParadigmLens::evaluate(&LensInput)` in
`crates/sddk-engine/src/alignment_lens/paradigm.rs` iterated
`self.concerns`, overwrote `out` on each iteration, and returned only
the **last** contribution. So every call to a production lens via
`AlignmentLensKernel::evaluate` substituted the requested concern
with the lens's terminal declared concern:

- OO → returned `Coupling` for every requested concern.
- Functional → returned `SemanticOwnership`.
- ADT → returned `SemanticOwnership`.
- DSL → returned `TemporalCoupling`.

The contribution identity was wrong too, because
`derive_contribution_id` hashed the (incorrect) concern into the
content address. A4-4M shipped four production lenses but no pin in
`tests/a4_4m_convergence_pins.rs` exercised the production kernel
path, so the bug was invisible to the corpus.

A4-4MR fixes the bug locally in `ParadigmLens::evaluate`:

- `let concern = input.concern();` is the ONLY concern the lens
  considers.
- If `!self.concerns.contains(&concern)`: return
  `LensEvaluationOutcome::Refused(LensError::LensRejected { id, concern })`.
- Otherwise: exactly ONE `LensContribution` carrying
  `contribution.concern == concern` and identity
  `derive_contribution_id(self.id, versions::V1, concern, ...)`.

The legacy `paradigm_lens::evaluate_lens()` compatibility facade is
unchanged — it never instantiated `AlignmentLens` and never invoked
`AlignmentLensKernel`. Its `LensStatus` is a substrate posture
projection, not a concern-tagged answer. The production-vs-facade
boundary is now documented in the `paradigm_lens/lenses.rs` module
header.

## Release

- **Tag:** `v1.169.63`
- **Release SHA:** *(filled at release time, see `git ls-remote origin v1.169.63`)*
- **Cargo.toml bump:** `1.169.62 → 1.169.63`
- **Baseline:** `v1.169.62` / SHA `b9e928c6229e65dc141fb282c594931dc7ef7df3`
- **PublicReleaseGate:** PASS (scenario 9 against the real release at
  the GH API)
- **Release script:** `bash scripts/release.sh` (14 pasos + step 9b)
- **Local install:** `~/.local/share/sddk/framework/1.169.63/`

## Pre-flight gate (M0)

### Bug location and shape

`crates/sddk-engine/src/alignment_lens/paradigm.rs:209-227` (pre-fix):

```rust
let mut out: Option<LensContribution> = None;
for concern in self.concerns {
    let id = derive_contribution_id(
        self.id,
        versions::V1,
        *concern,                   // <-- iterates ALL declared concerns
        &set_digest,
        &resolution,
        &evidence_refs,
    );
    out = Some(LensContribution::assemble(
        id,
        self.id,
        versions::V1,
        *concern,                   // <-- overwrites on every iteration
        resolution.clone(),
        evidence_refs.clone(),
    ));
}
```

Two coupled defects in three lines: the variable name `concern` is a
declared concern (not the requested one), and `out` is overwritten
on every iteration. The composition has a third effect:
`derive_contribution_id` hashes the (wrong) concern into the content
address.

### Blast radius

- **Production lenses**: `ParadigmLens::ALL` (four lenses in
  `alignment_lens/paradigm.rs`). All four were buggy.
- **Kernel**: `AlignmentLensKernel::evaluate` is unchanged. It is
  concern-agnostic by design; the concern-preservation invariant is
  the lens's contract.
- **Legacy facade**: `paradigm_lens::evaluate_lens` is unchanged — it
  never instantiated `AlignmentLens`.
- **No new type, no new variant, no new vocabulary.** Fix is local
  to one function body.

### Why A4-4M did not catch this

`tests/a4_4m_convergence_pins.rs` covers:

| Pin | Subject | Production `ParadigmLens`? |
|---|---|---|
| M3 — registry composition | `reg.len() == 4`, ids prefix | No (registry only) |
| M5 — textual probes | removed legacy motors | No |
| M7 — corpus equivalence | `evaluate_lens(...)` ↔ substrate posture projection | No (facade only) |
| M9 / M10 | provenance / inferred-path | No |

No pin in A4-4M invokes a production `ParadigmLens` through
`AlignmentLensKernel::evaluate`. The bug was invisible to the corpus
because the corpus never entered the production kernel path.

### Decision: option B — local fix in `ParadigmLens::evaluate`

- Option A (kernel-side guard): rejected — conflates layers.
- Option C (new trait method `evaluate_for`): rejected — adds API
  surface for one bug.
- Option B (lens body fix): chosen — local, minimal, structurally
  correct.

## Change budget

**PRESERVE INPUT CONCERN THROUGH PRODUCTION LENS.**

### What changed

1. **`crates/sddk-engine/src/alignment_lens/paradigm.rs`** —
   `ParadigmLens::evaluate(&LensInput)` rewritten to use
   `let concern = input.concern();` exclusively, with typed
   `LensError::LensRejected { id, concern }` refusal for unsupported
   concerns. Module header documents the new contract.

2. **`crates/sddk-engine/src/paradigm_lens/lenses.rs`** — module
   header gains the **Production-vs-facade boundary** section
   clarifying that `evaluate_lens` is LEGACY_READ_COMPAT for the AC7
   corpus and does NOT invoke `AlignmentLensKernel`. Future A4-5a
   consumers MUST go through the kernel.

3. **`crates/sddk-engine/tests/a4_4mr_concern_preserving.rs`** — new
   corpus with **16 pins across seven pin families**.

4. **`docs/architecture/a4-4m-migration-matrix.md`** — new §8
   "M11 — Concern-Preserving ParadigmLens Evaluation (corrective
   slice)" recording the bug, the M0 evidence, and the fix.

5. **`docs/architecture/README.md`** — A4-4MR row inserted (alongside
   A4-4M / A4-4bR / A4-3R2); A4-5a entry gate annotated with
   "concern-preservation invariant now enforced by A4-4MR".

6. **`.sddk/followups/a4-followups.md`** — A4-4MR row added with
   CLOSED status and a cross-reference to `FU-A4-4M-CONCERN-PRESERVATION`.

7. **`docs/debt/FU-A4-4M-CONCERN-PRESERVATION.md`** — `Status: OPEN`
   → `Status: CLOSED`; §0 closure evidence block added (see below).

8. **`Cargo.toml` / `Cargo.lock`** — workspace version `1.169.62 → 1.169.63`.

### What did NOT change

- `AlignmentLensKernel` (kernel is concern-agnostic by design).
- `LensContribution` shape, identity, posture variants.
- `EvidencePosture`, `ObservationSubject`, `ObservationSet`,
  `SoftwareObservation`, `paradigm_profile::*`, `paradigm_lens::*`
  (except the `lenses.rs` header doc), `software_alignment::*`,
  `architecture_why::*`, `debverify_kernel::*`, CLI.
- `LensInput`, `LensDescriptor`, `LensError`, `LensId`, `LensVersion`.
- No new `LensError` variant — `LensError::LensRejected { id,
  concern }` already exists from A4-4b.
- No removal of the legacy `evaluate_lens` facade.
- No A4-5 Intelligence Loop composition. **A4-5a does NOT
  auto-open.**
- No score / confidence / priority / weight fields.
- No change to the four `EvidencePosture` variants.
- No change to the 7 `AlignmentState`, 3 `AlignmentFindingKind`,
  `AcceptedDecision`, `ReviewDue`, `AlignmentAssessment`.

## Pin corpus (16 pins, 7 families)

`crates/sddk-engine/tests/a4_4mr_concern_preserving.rs`:

- **P1** — exhaustive positive: every production lens × every
  declared concern through `AlignmentLensKernel` (4 tests).
- **P2** — production registry aggregate: declared concerns emit
  contributions; undeclared concerns emit `NotEvaluated {
  reason: NoRegisteredLens }` (2 tests).
- **P3** — defence-in-depth: direct `Lens::evaluate` refusal path;
  every lens × every unsupported concern → typed `LensRejected` (1
  test, ~28 sub-iterations).
- **P4** — multi-lens invariants: same concern across multiple
  matching lenses yields one contribution per lens, each carrying
  the requested concern (3 tests covering StateSafety, BoundaryIntegrity,
  DependencyDirection mixed-success-and-refusal).
- **P5** — falsification: posture class does not change contribution
  concern (Supported / Contradicted / Conflicted / Insufficient); one
  test verifying all four posture classes; one test verifying
  content-identical determinism across two distinct `ObservationSet`s.
- **P6** — registry proof UAT: every (lens, declared concern) pair
  exercised end-to-end through the production registry + kernel; one
  test verifying `contribution.id` is canonical for the requested
  concern (2 tests).
- **P7** — cross-concern identity: two distinct concerns at the same
  lens yield two distinct ids and each carries its own concern; one
  test hand-computing the expected id via `derive_contribution_id(...
  concern, ...)` and asserting byte-equality (2 tests).

**Verification:** the corpus fails 13/16 against the pre-fix code
(`for concern in self.concerns` loop), passes 16/16 against the
post-fix code (`let concern = input.concern();` exclusive).

## Closure evidence (FU-A4-4M-CONCERN-PRESERVATION §0)

- Release: **v1.169.63** (release tag `v1.169.63` → SHA recorded
  after `gh release create`).
- Cycle: `p-63676b11dc0ef88f-a4-4mr-concern-preserving`.
- Scope contract: `.sddk/cycles/p-63676b11dc0ef88f-a4-4mr-concern-preserving/spec.md`.
- Handoff: this file.
- PublicReleaseGate: PASS (scenario 9 against the real release at the
  GH API).
- `cargo test --workspace` → **4586 passed, 0 failed**; `cargo fmt
  --check` clean; `cargo clippy --workspace --all-targets -- -D
  warnings` clean.
- Falsification corpus: `crates/sddk-engine/tests/a4_4mr_concern_preserving.rs`
  (16 pins, 7 families).
- A4-4M / A4-4b / A4-4bR / A4-4C / A4-3R / A4-3R2 regression: all
  green; the legacy `paradigm_lens::evaluate_lens()` facade is
  unchanged and its tests still pass.

## After closure

A4-5a (Intelligence Loop composition) is **structurally unblocked**:

- The kernel returns contributions whose `concern` equals the
  request.
- The kernel's per-lens refusal path is typed (`LensError::LensRejected
  { id, concern }`).
- The kernel aggregate behaviour for undeclared concerns is
  deterministic (`NoRegisteredLens`) and for partial-success
  (some-lenses-succeed, some-refuse) is the silent absorption of
  refusals (per spec §3).

A4-5a is **NOT auto-opened** by this cycle. The next move is the
user's choice (likely a brief acknowledgement of the closure and an
explicit green-light for A4-5a, given the user's pre-flight preference
"After A4-4MR, I want A4-5a opened **without another intermediate
preflight**").

## A4-5a frontier (anticipated)

When A4-5a opens (no preflight):

- Frontier: `KnowledgeBasis + ObservationSet → {Verify, DebVerify,
  production LensContribution[], AlignmentAssessment} →
  IntelligenceLoopResult + IntelligenceLoopReceipt`.
- Composition only. **No** `OverallStatus` collapse. Preserve
  side-by-side `VerificationResult ≠ ReconciliationSummary ≠
  AlignmentState`.
- `LensContribution` remains advisory without `AlignmentState` /
  authority / capability / instruction source (per MISALIGNED ≠
  DENY; no `Alignment → Authority | Capability | InstructionSource`
  shortcuts).

## Pinned reference points

| Object | SHA / version |
|---|---|
| A4-4MR release tag | `v1.169.63` (recorded at `gh release view v1.169.63`) |
| A4-4MR release SHA | *(recorded at release)* |
| Baseline | `v1.169.62` → `b9e928c6229e65dc141fb282c594931dc7ef7df3` |
| Workspace version | `1.169.63` |

## References

- FU (now closed): `docs/debt/FU-A4-4M-CONCERN-PRESERVATION.md`
- Bug site (pre-fix): `crates/sddk-engine/src/alignment_lens/paradigm.rs:150-237`
- Identity site: `crates/sddk-engine/src/alignment_lens/id.rs:87-101`
- Kernel (unchanged): `crates/sddk-engine/src/alignment_lens/kernel.rs`
- Types (unchanged): `crates/sddk-engine/src/alignment_lens/types.rs`
- Errors (unchanged, `LensRejected` already exists):
  `crates/sddk-engine/src/alignment_lens/error.rs:54-57`
- Legacy facade (unchanged): `crates/sddk-engine/src/paradigm_lens/lenses.rs`
- A4-4M corpus: `crates/sddk-engine/tests/a4_4m_convergence_pins.rs`
- A4-4b corpus: `crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs`
- Migration matrix §8: `docs/architecture/a4-4m-migration-matrix.md`
- Architecture roadmap entry: `docs/architecture/README.md`
- A4 follow-ups: `.sddk/followups/a4-followups.md`
- Prior cycle (analogous shape): A4-3R2 —
  `.sddk/cycles/p-63676b11dc0ef88f-a4-3r2-namespace-safe-targets/spec.md`
- A4-5P entry gate: `docs/handoff/HANDOFF-2026-09-17-a4-5p-intelligence-loop-entry-gate.md`
- A4-3R2 handoff (template):
  `docs/handoff/HANDOFF-2026-09-17-a4-3r2-namespace-safe-targets.md`
- INC-A4-RELEASE-VERSION-DRIFT: `docs/debt/INC-A4-RELEASE-VERSION-DRIFT.md`
- INC-M7-9 (pre-push hook): `.sddk/followups/a4-followups.md`
