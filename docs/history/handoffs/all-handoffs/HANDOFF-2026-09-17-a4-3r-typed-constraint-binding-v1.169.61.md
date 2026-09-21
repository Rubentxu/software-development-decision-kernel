# Handoff — A4-3R — Typed Constraint Binding (v1.169.61)

> **Cycle:** `p-63676b11dc0ef88f/a4-3r-typed-constraint-binding`
> **Status:** closed (released v1.169.61)
> **Date:** 2026-09-17
> **Released baseline inherited:** v1.169.60 / `ca14e46078b3c9c10c541ec6b24a6483d4f1fc82`
> **Release tag:** `v1.169.61` → SHA `64c29adb2a9570febd29424e9da912bfb792c630`
> **Final marker commit:** `6e15c45f8f9ee82f7c98625560a5a5f4e73408b9` (Cargo.lock + ceremonial `chore(release): bump version` per INC-M7-9)
> **Branch state:** `HEAD == origin/main == 6e15c45`. Working tree clean.

## What shipped

A4-3R closes `FU-A4-3-CONSTRAINT-BINDING` (P1) by replacing the
substring-match rule inside `software_alignment::reduce_alignment` with
a typed binding chain:

```text
ExplicitConstraint.contract_ref
       ↓ typed lookup (ContractId equality, no string matching)
ArchitecturalContract
       ↓ typed payload
ContractPayload (SingleAuthority | UniqueOwner | ForbiddenDependency)
       ↓ typed BindingTarget
ObservationSubject (Unit | SoftwareRelation)
```

### Type-system changes (closed enums — additions only, never widening)

- `FindingCause::{None, ContractViolation(ContractViolationCause)}` —
  typed cause attached to every `AlignmentFinding` with `#[serde(default)]`
  on legacy data. `derive_id()` now includes `cause.canonical()` in the
  identity hash.
- `ContractViolationCause::{ContradictsMust, EvidenceGap { kind_tag }}` —
  `EvidenceGap { kind_tag }` carries the contract's canonical kind tag
  (e.g. `"projection_only"`, `"provider_boundary"`, `"custom_kind"`).
- `AlignmentFinding::contract_violation_cause()` accessor returns
  `Option<&ContractViolationCause>` — typed extraction; no string parse.

### Reducer changes (`software_alignment/reducer.rs`)

- `BindingOutcome::{UnknownContract, Binding(BindingTarget), NonBinding { kind_tag }}`.
- `BindingTarget::{ForbiddenDependency { from, to }, SingleAuthority(ComponentRef), UniqueOwner(EntityRef)}`.
- `derive_binding_outcome(contract_ref, contracts, intent_constraints)` does
  ContractId equality lookup against `contracts: &[ArchitecturalContract]`,
  then dispatches on `ContractPayload` kind to compute `BindingTarget`.
- `observation_matches_target(subject, &binding_target)` enforces typed
  equality between `ObservationSubject` and `BindingTarget`. The function
  is `false` for typed mismatches (e.g. ForbiddenDependency vs Unit
  observation); `false` for rendered text/canonical_tag substrings;
  `false` for unrelated contract_refs even if they share a substring.

### Substring / rendered-text defence (cannot regress)

- The reducer no longer calls `SoftwareRelation::render()` for matching.
- The reducer no longer calls `canonical_tag().contains(contract_ref)`.
- The reducer no longer iterates `subject_canonical_tag` strings against
  contract_refs.
- Any future attempt to reintroduce string-based binding requires
  editing `BindingTarget` (closed enum) — compile error otherwise.

### Non-binding kinds → typed `EvidenceGap`

`ProjectionOnly`, `BoundedCompatibility`, `ProviderBoundary`, and
`Extension(k)` contracts are non-binding. The reducer emits a single
finding with `cause = EvidenceGap { kind_tag }` and empty
`evidence_observations` — typed gap, distinct from
`ContradictsMust`. The kind_tag is the payload's canonical kind tag
(closed enum derived from `ContractPayload::kind_tag()`).

## Tests

| Suite | Count | Status |
|---|---|---|
| `cargo test -p sddk-engine --lib software_alignment` | 24 | green |
| `cargo test -p sddk-engine --test a4_3r_typed_constraint_binding` | 18 | green |
| `cargo test -p sddk-engine --lib` | 1263 | green |
| `cargo clippy --workspace --all-targets -- -D warnings` | — | clean |
| `cargo fmt --all -- --check` | — | ok |

### Pin test coverage (a4_3r_typed_constraint_binding.rs)

| Pin | Subject |
|---|---|
| `pin_r01` | ForbiddenDependency typed mismatch against Unit relation |
| `pin_r02` | ForbiddenDependency mismatch when observation lacks relation |
| `pin_r03` | ForbiddenDependency mismatch on unrelated target |
| `pin_r04` | SingleAuthority matches on Unit (Denies) |
| `pin_r05` | SingleAuthority no match on substring collision |
| `pin_r06` | UniqueOwner matches on entity |
| `pin_r07` | UniqueOwner no match on similar entity |
| `pin_r08` | UnknownContractId emits nothing |
| `pin_r09` | ProjectionOnly emits typed EvidenceGap |
| `pin_r10` | ProviderBoundary emits typed EvidenceGap |
| `pin_r11` | Extension kind emits typed EvidenceGap |
| `pin_r12` | ContractId substring of unrelated subject does not bind |
| `pin_r13` | Rendered text does not participate in matching |
| `pin_r14` | Observation ordering is independent |
| `pin_r15` | `AlignmentFinding` has no AuthorityDecision field |
| `pin_r16` | Reducer module path is single source of truth |
| `pin_r17` | `FindingCause` is a closed enum (no A4-5 introgression) |
| `pin_r18` | A4-3 corpus regression under typed rule |

## Release pipeline

`bash scripts/release.sh` — 14/14 steps green:

| # | Step | Result |
|---|---|---|
| 0 | Preflight | clean: branch main, HEAD = `chore(release): bump version` |
| 1 | Workspace green | fmt + clippy -D warnings + cargo test --workspace: green |
| 2 | Read version | `1.169.61` |
| 3 | Build binary | `cargo build --release --bin sddk` clean |
| 4 | Manifest | `sddk dev manifest --root . --verify` — RDI clean |
| 5 | Bundle tarball | `tar czf` with prefix `software-development-decision-kernel/` |
| 6 | BUNDLE.toml (v2) | `schema_version=2`, manifest_sha256 injected |
| 7 | Unified tarball | `bin/sddk` + `framework/` (defensive chmod 0755 on binary) |
| 8 | sha256 + CHECKSUMS + sbom | all present (CycloneDX 1.5) |
| 9 | `gh release create` | 9/9 assets published |
| 9b | PublicReleaseGate | tag SHA anchored, `isDraft=false`, `isPrerelease=false`, 9/9 HTTP 200 |
| 10 | Install from URL | exit 0, binary exec bit set, bundle coherent |
| 11 | `sddk dev doctor` | `binary.bundle_coherence: present`, `all_present: true` |
| 12 | `sddk dev update --prune-only --keep 1` | removed `1.169.60`, kept `1.169.61` |
| 13 | Final state | `binary: sddk 1.169.61`, `bundle: 1.169.61`, `current: 1.169.61` |

## Commit map

| SHA | Subject |
|---|---|
| `6d25e6c` | `feat(uat): typed constraint binding (A4-3R)` |
| `7b0c841` | `docs(architecture): A4-3R closed — typed constraint binding (commit 6d25e6c)` |
| `64c29ad` | `chore(release): bump version to 1.169.61` |
| `6e15c45` | `chore(release): bump version` (Cargo.lock refresh + ceremonial marker) |

## Disposition review (no implementation)

These carry-overs are reviewed and dispositioned but NOT implemented
(this is documentation only, per the user's "review only" mandate):

| ID | Severity | Disposition |
|---|---|---|
| `FU-A3-CO-1` | P2 | review note: not blocking A4-3R; remains in `docs/debt/` |
| `FU-A3-CO-3` | P3 | review note: trivial; remains in `docs/debt/` |
| `FU-A3-S15-3` | P3 | review note: trivial; remains in `docs/debt/` |
| `FU-A3-S15-4` | P3 | review note: trivial; remains in `docs/debt/` |
| `ASC-MA-1` | P3 | review note: tracked in UAT dashboard; remains open |
| Stale UAT flake | — | review note: pre-existing flake unrelated to A4-3R; remains tracked |
| `INC-A4-RELEASE-VERSION-DRIFT` | P2 | review note: pre-bump vs release-tag confusion (cycle-46 install-coherence contract); not triggered in v1.169.61 |

## Compatibility mandate (honoured)

- 7 `AlignmentState` variants unchanged.
- 3 `AlignmentFindingKind` variants unchanged.
- `AcceptedDecision` semantics unchanged.
- `ReviewDue` semantics unchanged.
- `AlignmentAssessment.id` derivation unchanged in shape; `cause.canonical()`
  is appended to the hash but does NOT change the AssessmentState or
  FindingKind semantics.

## What was actually hard

1. **Pin test #1 first compile had 5 errors.** `SpecRef::new(s)` does
   not exist (use `SpecRef::Spec(s.to_string())`). `AlignmentFinding`
   doesn't implement `DeserializeOwned` (so the compile-time witness for
   "no authority_decision field" uses the typed `FindingCause` enum
   directly). No `declare_extension` constructor exists — used raw
   `ArchitecturalContract::declare(..., ContractKind::Extension(kind_ref),
   ContractPayload::Extension { ... })`. `SoftwareUnitRef` lives in
   `crate::architecture_graph` (one extra import path).
2. **Lint `clippy::cloned_ref_to_slice_refs` flagged `&[contract.clone()]`.**
   Replaced with `std::slice::from_ref(&contract)`.
3. **4 unused helpers in `software_alignment/tests.rs`** (`entity_ref`,
   `contract_forbidden_dependency`, `contract_unique_owner`,
   `contract_projection_only`) — leftovers from earlier test-rewrite
   rounds; deleted.
4. **`cargo fmt` on the new test file** — multi-line `let _: FindingCause
   = FindingCause::ContractViolation(...)`; cargo fmt collapsed it.

## Next cycle (A4-5) — STOP rule applies

A4-5 does NOT auto-open. Requirements:
1. New ROADMAP-SYNC preflight.
2. New scope contract at `.sddk/cycles/p-63676b11dc0ef88f-a4-5-.../spec.md`.
3. **`FU-A4-3-CONSTRAINT-BINDING` is closed (this cycle).** A4-5's only
   external blocker is A4-4C (also closed at v1.169.60).
4. User green-light.

Sequence per user: A4-5P → A4-5a → A4-5b → A4-5C → A4-CLOSEOUT → A5.

## Files of interest

- `crates/sddk-engine/src/software_alignment/types.rs` — FindingCause, ContractViolationCause (mod)
- `crates/sddk-engine/src/software_alignment/reducer.rs` — BindingOutcome, BindingTarget, derive_binding_outcome, observation_matches_target (mod)
- `crates/sddk-engine/src/software_alignment/tests.rs` — 5 migrated tests (mod)
- `crates/sddk-engine/tests/a4_3r_typed_constraint_binding.rs` — 18 pin tests (NEW)
- `.sddk/cycles/p-63676b11dc0ef88f-a4-3r-typed-constraint-binding/spec.md` — scope contract (UPDATED)
- `docs/architecture/README.md` — A4-3R roadmap row + A4-4b carry-over flipped to closed (UPDATED)
- `docs/handoff/HANDOFF-2026-09-17-a4-3r-typed-constraint-binding-v1.169.61.md` — this file (NEW)

## Resume point for the next session

CWD: `/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/sddk-framework`
Branch: `main` at `6e15c45`
Working tree: clean
Binary: `sddk 1.169.61` installed at `/home/rubentxu/.local/bin/sddk`
Framework bundle: `~/.local/share/sddk/framework/1.169.61/`
Released tag: `v1.169.61` → `64c29ad`
Local symlink: `current -> 1.169.61`
