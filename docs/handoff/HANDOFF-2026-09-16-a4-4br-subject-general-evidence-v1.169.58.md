# Handoff: A4-4bR — Subject-General Evidence Resolution → v1.169.58

**Date:** 2026-09-16
**Cycle:** `p-63676b11dc0ef88f/a4-4br-subject-general-evidence`
**Status:** SHIPPED + RELEASED.
**Release tag:** `v1.169.58` → SHA `b37321ff2fd337f36cda822bd96385ddf768a3b7`
**GH Release URL:** https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.58

## Scope

A4-4bR is a **ONE-budget generalization** (`GENERALISE EVIDENCE TARGET`). It
fixes the conceptual incompatibility found in the A4-4M preflight:
`ObservationSubject` (A4-0) supports four target kinds but
`EvidenceResolution` could only resolve around a single `RelationId`,
forcing AC7-style unit observations into fabricated self-relations.

## What landed

### Commits (this cycle)

| # | Hash | Subject |
|---|------|---------|
| 0 | 5f7a3e6 | docs(preflight): A4-4bR — reconcile release version + ADR attribution + drift fixes |
| 1 | 4de3dd1 | docs(preflight): A4-4bR — fix alignment_lens doc-comment drifts |
| 2 | 86606c6 | feat(engine): A4-4bR — subject-general EvidencePosture algebra |
| 3 | 1a8cd33 | docs: ADR-0125 amendment + roadmap delta + handoff |
| 4 | b37321f | chore(release): bump version 1.169.57 -> 1.169.58 (lockfile sync) |

### Source

- `crates/sddk-engine/src/observation/posture.rs` (NEW): `EvidencePosture<Target>`,
  `InsufficientGap`, `ObservationTargetRef` (Relation/Unit/Contract/Knowledge
  with `canonical_tag()`).
- `crates/sddk-engine/src/observation/resolution.rs`:
  `EvidenceResolution = EvidencePosture<RelationId>`; `resolve_relation`
  signature and behavior unchanged; new subject-general `resolve_subject`.
- `crates/sddk-engine/src/alignment_lens/`: `LensContribution` and
  `LensContributionId` migrated to `LensEvidenceResolution =
  EvidencePosture<ObservationTargetRef>`; **target kind enters identity**.

### Tests

- `crates/sddk-engine/tests/a4_4br_subject_general_evidence.rs`: 29 pin tests
  (P01-P24 of the spec plus equivalence decomposition), including:
  - equivalence `resolve_relation` old vs generalized path (equality, not congruence)
  - `Unit(foo) != Relation(foo->foo)` in identity
  - `Conflicted(Unit)` preserves both observation sets
  - `Insufficient` carries typed `InsufficientGap`, never a free-form string
  - anti-encroachment probes (no second taxonomy, no paradigm_lens import,
    no synthetic relations in Freshness fixture)
- A4-4b pins still green (24/24); A4-0/A4-2 relation tests unchanged and green.

## Gate results

| Gate | Result |
|------|--------|
| `cargo build -p sddk-engine --tests` | clean |
| `cargo test -p sddk-engine --test a4_4br_subject_general_evidence` | 29/29 pass |
| `cargo test -p sddk-engine --test a4_4b_alignment_lens_kernel` | 24/24 pass |
| `cargo test -p sddk-engine --lib` | 1259/1259 pass |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace --offline` | 210 suites, 0 failed |

Post-release gates: PublicReleaseGate PASS (9/9 assets HTTP 200), CDN
sha256 verified, install from URL, doctor `all_present: true`, prune,
distrib round-trip OK. Full evidence in
`.sddk/cycles/p-63676b11dc0ef88f-a4-4br-subject-general-evidence/archive-manifest.md`.

## Debt notes

- `FU-A4-3-CONSTRAINT-BINDING` (P1) remains open; NOT fixed here.
- `INC-A4-RELEASE-VERSION-DRIFT` (P2) registered this cycle; NOT fixed here
  (docs/debt/INC-A4-RELEASE-VERSION-DRIFT.md).

## Roadmap delta

```text
A4-4b   Generic AlignmentLens kernel/registry   ✓ v1.169.57
A4-4bR  Subject-General Evidence Resolution     ✓ v1.169.58 (this cycle)
A4-4M   AC7 paradigm_lens convergence           NEXT (unblocked; does NOT auto-open — STOP)
```

## STOP

A4-4M does NOT auto-open after A4-4bR closes. It requires explicit user
green-light per the cycle spec.
