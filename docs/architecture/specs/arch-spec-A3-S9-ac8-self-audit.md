---
id: arch-spec-A3-S9-ac8-self-audit
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-9-ac8-self-audit-receipt
source: arch-spec-041-sddk-architecture-self-audit + ADR-0112/0113/0116/0117/0118
based_on: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/11-FITNESS-RECEIPTS.md
---

# arch-spec-A3-S9 — SDDK Self-Audit + ArchitectureConformanceReceipt

## Intent

AC8 of the Architecture Conformance track (per `09-ROADMAP.md`):

> Run SDDK against itself. Reproduce a representative subset of A0/A1 findings
> using native capabilities and produce a named `ARCHITECTURE-CONFORMANCE-RECEIPT`.
> This is a Base production-readiness gate because the feature claims conformance
> over its own architecture.

`10-UAT.md` §Production acceptance:

> Base cannot claim the AC capability production-ready until AC-UAT-001..016 pass
> at a named commit and `ARCHITECTURE-CONFORMANCE-RECEIPT` lists **zero
> unresolved MUST findings**.

Inherits `arch-spec-041` (stays `proposed`).

## What AC8 is (and is not)

AC8 is an **aggregator + judge**, not a new detector. Every historical class is
already reproducible natively:

| Historical class (`01-VISION` A0/A1 closeout) | Native reproducer |
|---|---|
| duplicate/shadow authority | AC5 `ShadowAuthority` |
| dependency boundary | AC5 `AuthorityBypass` |
| bounded compatibility | AC5 `StaleCompatibility` / `MissingOwner` |
| projection/authority confusion | AC5 `Contradiction` |
| missing negative evidence | AC6 mutation suite `all_detected` |

The receipt composes AC2 (graph digest), AC4 (delta), AC5 (audit), AC6
(mutations) and AC7 (lenses) into the upstream-defined receipt shape, and judges
the verdict from unresolved MUST findings.

## Upstream coverage

| Upstream | In scope | Where |
|---|---|---|
| AC-041-001 revision + contract-set digest | yes | REQ-AC8-001..003 |
| AC-041-002 five historical classes | yes | REQ-AC8-011..016 |
| AC-041-003 unresolved MUST blocks PASS | yes | REQ-AC8-008..010 |
| AC-041-004 delete/rebuild equivalence | yes | REQ-AC8-017 |
| AC-041-005 Base mode without CogniCode/Chronos/LLM | yes | REQ-AC8-018 |
| AC-041-006 providers strengthen evidence only | yes | REQ-AC8-019 |

UAT: **AC-UAT-016**.

## Scope (must)

- `ArchitectureConformanceReceipt` carrying the upstream fields.
- `ReceiptVerdict` closed with exactly 3 variants.
- `ReceiptBasis` with revision, contract-set digest, graph digest, plan digest,
  knowledge basis (AC-041-001).
- `compose_receipt(inputs, now)` — pure aggregation of existing capability outputs.
- `HistoricalClass` (5-closed) + `ClassCoverage` + `evaluate_class_coverage`.
- Verdict rules driven by unresolved MUST findings and supplied waivers.
- Deterministic receipt identity (`ReceiptId`) derived from the basis + verdict.

## Scope (must NOT)

- MUST NOT re-derive AC4/AC5/AC6/AC7 logic; it consumes their outputs.
- MUST NOT construct `ArchitectureClaim`; MUST NOT write to the graph or IO.
- MUST NOT depend on CogniCode / Chronos / any LLM / provider (AC-041-005).
- MUST NOT invent or persist waivers; it records supplied ones only.
- MUST NOT emit a numeric score (AC-UAT-043 discipline).
- MUST NOT flip `BASE_PRODUCTION_READY`; it emits the receipt only.

## Requirements (REQ-AC8-NNN)

### Basis (AC-041-001)

- **REQ-AC8-001** — `ReceiptBasis { revision, contract_set_digest,
  semantic_graph_digest, verification_plan_digest, knowledge_basis }`.
  Pin: `acceptance_basis_shape`.
- **REQ-AC8-002** — `contract_set_digest` and `verification_plan_digest` come
  from AC4's delta unchanged; `semantic_graph_digest` from AC2.
  Pin: `acceptance_basis_carries_ac4_and_ac2_digests`.
- **REQ-AC8-003** — `ReceiptId` is derived from the basis digests + verdict so
  two composers over identical inputs yield the same id.
  Pin: `acceptance_receipt_id_is_derived`.

### Verdict (AC-041-003)

- **REQ-AC8-004** — `ReceiptVerdict` is closed with exactly 3 variants:
  `Pass | PassWithWaivers | Blocked`. Pin: `acceptance_verdict_closed_three`.
- **REQ-AC8-005** — Unresolved MUST findings are: every AC4 `unknown`, every
  AC4 `contradiction`, every AC5 finding of severity `Critical` or `High`, and
  every AC6 probe that was **not** detected.
  Pin: `acceptance_unresolved_derivation`.
- **REQ-AC8-006** — No unresolved findings ⇒ `Pass`.
  Pin: `acceptance_pass_when_clean`.
- **REQ-AC8-007** — Unresolved findings all covered by supplied waivers ⇒
  `PassWithWaivers`. Pin: `acceptance_pass_with_waivers`.
- **REQ-AC8-008** — Any unresolved finding not covered by a waiver ⇒ `Blocked`.
  Pin: `acceptance_blocked_without_waiver`.
- **REQ-AC8-009** — A waiver covers a finding iff the waiver string appears in
  `unresolved.waiver_refs` for that finding; the composer never invents one.
  Pin: `acceptance_waiver_matching_is_explicit`.
- **REQ-AC8-010** — AC5 `Medium` findings are recorded but are not MUST
  (they do not by themselves block). Pin: `acceptance_medium_is_advisory`.

### Historical classes (AC-041-002, AC-UAT-016)

- **REQ-AC8-011** — `HistoricalClass` is closed with exactly 5 variants:
  `DuplicateAuthority | DependencyBoundary | BoundedCompatibility |
  ProjectionOnly | MissingNegativeEvidence`. Pin: `acceptance_class_closed_five`.
- **REQ-AC8-012** — `DuplicateAuthority` is reproduced iff AC5 reports a
  `ShadowAuthority`. Pin: `acceptance_class_duplicate_authority`.
- **REQ-AC8-013** — `DependencyBoundary` is reproduced iff AC5 reports an
  `AuthorityBypass`. Pin: `acceptance_class_dependency_boundary`.
- **REQ-AC8-014** — `BoundedCompatibility` is reproduced iff AC5 reports a
  `StaleCompatibility` or a `MissingOwner`.
  Pin: `acceptance_class_bounded_compatibility`.
- **REQ-AC8-015** — `ProjectionOnly` is reproduced iff AC5 reports a
  `Contradiction`. Pin: `acceptance_class_projection_only`.
- **REQ-AC8-016** — `MissingNegativeEvidence` is reproduced iff the AC6 mutation
  suite is non-empty and `all_detected` (the guards provably fire).
  Pin: `acceptance_class_missing_negative_evidence`.
- **REQ-AC8-017** — `evaluate_class_coverage` returns exactly 5 entries, each
  with concrete evidence locators. Pin: `acceptance_class_coverage_shape`.

### Base mode and providers (AC-041-005/006)

- **REQ-AC8-018** — `provider_basis` is empty in Base mode and the composer
  requires no provider. Pin: `acceptance_base_mode_is_provider_free`.
- **REQ-AC8-019** — Adding `provider_basis` entries does not change the verdict
  rules (they are additive evidence). Pin: `acceptance_providers_do_not_change_verdict`.
- **REQ-AC8-020** — The module imports no provider/host/LLM/Chronos/CogniCode
  surface. Pin: `anti_encroachment_no_provider_imports`.

### Determinism

- **REQ-AC8-021** — `compose_receipt` is pure: identical inputs ⇒ identical
  receipt (including digest). Pin: `acceptance_receipt_is_deterministic`.
- **REQ-AC8-022** — Rebuilding the graph overlay from the same inputs yields an
  equivalent receipt (`semantic_graph_digest` + class coverage unchanged).
  Pin: `acceptance_rebuild_preserves_findings` (AC-041-004).

### Anti-encroachment

- **REQ-AC8-023** — No `ArchitectureClaim` construction, no graph mutation, no
  `std::fs`, no numeric score field.
  Pin: `anti_encroachment_read_only_and_no_score`.
- **REQ-AC8-024** — The module does not re-implement AC4/AC5/AC6/AC7 detection:
  it contains no observation/probe/detector logic.
  Pin: `anti_encroachment_aggregator_only`.

## Acceptance tests (planned, 24)

| # | Test | REQ |
|---|---|---|
| 1 | `acceptance_basis_shape` | 001 |
| 2 | `acceptance_basis_carries_ac4_and_ac2_digests` | 002 |
| 3 | `acceptance_receipt_id_is_derived` | 003 |
| 4 | `acceptance_verdict_closed_three` | 004 |
| 5 | `acceptance_unresolved_derivation` | 005 |
| 6 | `acceptance_pass_when_clean` | 006 |
| 7 | `acceptance_pass_with_waivers` | 007 |
| 8 | `acceptance_blocked_without_waiver` | 008 |
| 9 | `acceptance_waiver_matching_is_explicit` | 009 |
| 10 | `acceptance_medium_is_advisory` | 010 |
| 11 | `acceptance_class_closed_five` | 011 |
| 12 | `acceptance_class_duplicate_authority` | 012 |
| 13 | `acceptance_class_dependency_boundary` | 013 |
| 14 | `acceptance_class_bounded_compatibility` | 014 |
| 15 | `acceptance_class_projection_only` | 015 |
| 16 | `acceptance_class_missing_negative_evidence` | 016 |
| 17 | `acceptance_class_coverage_shape` | 017 |
| 18 | `acceptance_base_mode_is_provider_free` | 018 |
| 19 | `acceptance_providers_do_not_change_verdict` | 019 |
| 20 | `acceptance_receipt_is_deterministic` | 021 |
| 21 | `acceptance_rebuild_preserves_findings` | 022 |
| 22 | `anti_encroachment_no_provider_imports` | 020 |
| 23 | `anti_encroachment_read_only_and_no_score` | 023 |
| 24 | `anti_encroachment_aggregator_only` | 024 |
| 25 | `acceptance_all_five_classes_reproduced` | 011..017, AC-UAT-016 |
| 26 | `bonus_empty_inputs_are_blocked_by_negative_evidence` | 016 |

## Determinism contract

Identical `(inputs, now)` ⇒ identical `ReceiptId`, identical `verdict`,
identical `class_coverage`, identical `digest`.

## State classes (ADR-0095)

| Value | Class |
|---|---|
| `ArchitectureConformanceReceipt` | PROJECTION (derived; reconstructible; no persistence) |
| `ReceiptBasis`, `ClassCoverage`, `UnresolvedFinding`, `ClaimResult`, `MutationResult`, `LensResult` | EPHEMERAL computed values |
| `ReceiptVerdict`, `HistoricalClass` | closed enums |
