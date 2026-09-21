---
id: arch-spec-A3-S7-ac5-debverify-audit
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-7-ac5-debverify-audit
source: arch-spec-034-architecture-conformance-verification (AC-034-002)
based_on: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/09-ROADMAP.md
---

# arch-spec-A3-S7 — DebVerify Architecture Audit

## Intent

AC5 of the Architecture Conformance track (per `09-ROADMAP.md`):

> Global challenge pass that looks for duplicate/shadow authorities, missing
> owners, bypasses, stale compatibility and contradictions even when no recent
> delta points at them.
> **Exit:** no conflation with `verify --full`.

Upstream `arch-spec-034` AC-034-002 (remains `proposed`):

> DebVerify independently challenges global assumptions and is not
> `verify --full`.

UAT: **AC-UAT-009**.

## The exit criterion, made structural

AC4 is change-scoped; AC5 is global. This is not a naming preference: it is
enforced by the module's shape.

| | AC4 | AC5 |
|---|---|---|
| signature | `(overlay, ConformanceInputs{changed_units,...}, now)` | `(overlay, contracts, now)` |
| scope | contracts reachable from changed units | every registered contract |
| output | `ArchitectureConformanceDelta` | `DebVerifyAudit` |
| runs when no change happened? | n/a (nothing affected) | **yes** |

Enforced by REQ-AC5-001/002/003.

## Scope (must)

- `DebVerifyFindingKind` closed with 5 variants (one per roadmap noun).
- `FindingSeverity` closed with 3 variants, deterministic per kind.
- `DebVerifyFinding { kind, severity, subjects, contract_ids, message }`.
- `DebVerifyAudit { findings, audited_contracts, graph_digest, digest,
  evaluated_at }` with `by_kind`, `counts`, `is_clean`.
- `run_debverify_audit(overlay, contracts, now)` — global, pure.
- Five detectors, each with a negative control.

## Scope (must NOT)

- MUST NOT accept or read a change basis / `changed_units`.
- MUST NOT import `architecture_conformance` (AC4) or its delta type.
- MUST NOT import `architecture_mutation`, `paradigm_profile`, `provider`,
  `host_sdk`, `agent_host`.
- MUST NOT construct `ArchitectureClaim`; MUST NOT mutate the graph.
- MUST NOT emit a numeric/weighted score.

## Requirements (REQ-AC5-NNN)

### The exit criterion (AC-034-002)

- **REQ-AC5-001** — `run_debverify_audit(overlay: &ArchitectureGraphOverlay,
  contracts: &[ArchitecturalContract], now: EventTime) -> DebVerifyAudit`.
  No change-basis parameter. Pin: `exit_criterion_signature_is_global`
  (call site compiles with exactly those three arguments).
- **REQ-AC5-002** — The module contains no reference to `changed_units`,
  `ConformanceInputs`, `compute_conformance_delta` or
  `ArchitectureConformanceDelta`. Pin: `exit_criterion_no_ac4_coupling`.
- **REQ-AC5-003** — `DebVerifyAudit` and `ArchitectureConformanceDelta` are
  distinct types with no conversion between them. Pin:
  `exit_criterion_audit_is_not_a_delta` (type-name + no `From` impl usage).
- **REQ-AC5-004** — The audit runs meaningfully when nothing changed: a
  duplicate authority is reported with no change basis involved (AC-UAT-009).
  Pin: `acceptance_ac_uat_009_duplicate_authority_with_empty_delta`.

### Findings

- **REQ-AC5-005** — `DebVerifyFindingKind` is closed with exactly 5 variants:
  `ShadowAuthority`, `MissingOwner`, `AuthorityBypass`, `StaleCompatibility`,
  `Contradiction`. Pin: `acceptance_finding_kind_closed_five`.
- **REQ-AC5-006** — `FindingSeverity` is closed with exactly 3 variants:
  `Critical`, `High`, `Medium`; the mapping kind→severity is total and
  deterministic. Pin: `acceptance_severity_mapping_total`.
- **REQ-AC5-007** — A `DebVerifyFinding` names the kind, severity, the sorted
  subject identifiers, the involved `ContractId`s (sorted, deduplicated) and a
  human message. Pin: `acceptance_finding_shape`.

### Detectors

- **REQ-AC5-008** — `ShadowAuthority`: two or more `SingleAuthority` contracts
  naming the same `ComponentRef`, or two or more `UniqueOwner` contracts naming
  the same `EntityRef`. Severity `Critical`.
  Pin: `acceptance_shadow_authority_component`,
  `acceptance_shadow_authority_entity`.
- **REQ-AC5-009** — `MissingOwner`: a `SingleAuthority`/`UniqueOwner` contract
  whose owner identifier appears in no graph node locator. Severity `High`.
  Pin: `acceptance_missing_owner_detected`.
- **REQ-AC5-010** — `AuthorityBypass`: a `ForbiddenDependency { from, to }`
  contract while the graph contains a relation joining the two identifiers.
  Severity `Critical`. Pin: `acceptance_authority_bypass_detected`.
- **REQ-AC5-011** — `StaleCompatibility`: a `BoundedCompatibility` contract with
  `now >= deprecated_after` and `replaced_by == None`. Severity `Medium`.
  Pin: `acceptance_stale_compatibility_detected`.
- **REQ-AC5-012** — `Contradiction`: the same subject identifier declared both
  authority-owned (`SingleAuthority`/`UniqueOwner`) and `ProjectionOnly`.
  Severity `High`. Pin: `acceptance_contradiction_detected`.

### Audit

- **REQ-AC5-013** — `findings` is sorted deterministically by
  `(kind, subjects, contract_ids)`. Pin: `acceptance_findings_sorted`.
- **REQ-AC5-014** — `counts()` returns a `BTreeMap<kind, usize>` of findings per
  kind; `is_clean()` is true iff there are no findings.
  Pin: `acceptance_counts_and_is_clean`.
- **REQ-AC5-015** — `audited_contracts` equals the number of contracts
  inspected. Pin: `acceptance_audited_contracts_count`.
- **REQ-AC5-016** — `digest` is sha256 over the canonical audit payload; two
  runs over identical `(overlay, contracts, now)` yield identical digests.
  Pin: `acceptance_audit_digest_is_stable`.
- **REQ-AC5-017** — `graph_digest` equals `overlay.digest()`. Pin:
  `acceptance_graph_digest_matches_overlay`.
- **REQ-AC5-018** — Pure: no IO, no clock reads; identical inputs → identical
  audit. Pin: `acceptance_audit_is_deterministic`.

### Negative controls

- **REQ-AC5-019** — A coherent architecture (no duplicates, owners present, no
  forbidden edge, unexpired compatibility, no projection conflict) yields zero
  findings. Pin: `acceptance_clean_architecture_has_no_findings`.

### Anti-encroachment

- **REQ-AC5-020** — No import of `architecture_conformance`,
  `architecture_mutation`, `paradigm_profile`, `provider`, `host_sdk`,
  `agent_host`, `capability`, `effective_instructions`.
  Pin: `anti_encroachment_no_forbidden_imports`.
- **REQ-AC5-021** — No `ArchitectureClaim` construction; no graph mutation; no
  `std::fs`/`File::`. Pin: `anti_encroachment_read_only_and_no_claims`.
- **REQ-AC5-022** — No numeric score field/method in the public types. Pin:
  `anti_encroachment_no_numeric_score`.

## Acceptance tests (planned, 24)

| # | Test | REQ |
|---|---|---|
| 1 | `acceptance_finding_kind_closed_five` | 005 |
| 2 | `acceptance_severity_mapping_total` | 006 |
| 3 | `acceptance_finding_shape` | 007 |
| 4 | `exit_criterion_signature_is_global` | 001 |
| 5 | `exit_criterion_no_ac4_coupling` | 002 |
| 6 | `exit_criterion_audit_is_not_a_delta` | 003 |
| 7 | `acceptance_ac_uat_009_duplicate_authority_with_empty_delta` | 004 |
| 8 | `acceptance_shadow_authority_component` | 008 |
| 9 | `acceptance_shadow_authority_entity` | 008 |
| 10 | `acceptance_missing_owner_detected` | 009 |
| 11 | `acceptance_authority_bypass_detected` | 010 |
| 12 | `acceptance_stale_compatibility_detected` | 011 |
| 13 | `acceptance_contradiction_detected` | 012 |
| 14 | `acceptance_findings_sorted` | 013 |
| 15 | `acceptance_counts_and_is_clean` | 014 |
| 16 | `acceptance_audited_contracts_count` | 015 |
| 17 | `acceptance_audit_digest_is_stable` | 016 |
| 18 | `acceptance_graph_digest_matches_overlay` | 017 |
| 19 | `acceptance_audit_is_deterministic` | 018 |
| 20 | `acceptance_clean_architecture_has_no_findings` | 019 |
| 21 | `anti_encroachment_no_forbidden_imports` | 020 |
| 22 | `anti_encroachment_read_only_and_no_claims` | 021 |
| 23 | `anti_encroachment_no_numeric_score` | 022 |
| 24 | `bonus_empty_contract_set_is_clean` | boundary |

## Determinism contract

Two `run_debverify_audit` calls over identical `(overlay, contracts, now)`
yield byte-identical `digest`, identical `findings` ordering, identical
`counts()`.

## State classes (ADR-0095)

| Value | Class |
|---|---|
| `DebVerifyAudit` | PROJECTION (derived, reconstructible from graph + contracts + now) |
| `DebVerifyFinding`, `FindingSeverity`, `DebVerifyFindingKind` | EPHEMERAL computed values / closed enums |
| `ArchitecturalContract` | OBJECT (owned by AC1; read-only) |

## Relationship to AC4 (documented, not imported)

AC4 answers "did this change break a contract?"; AC5 answers "is the
architecture itself coherent?". They deliberately share no types and no inputs
beyond the underlying AC1 contracts and AC2 overlay.
