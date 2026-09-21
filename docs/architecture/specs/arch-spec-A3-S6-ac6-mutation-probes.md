---
id: arch-spec-A3-S6-ac6-mutation-probes
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-6-ac6-mutation-probes
source: arch-spec-038-architecture-mutation-and-counterfactual-probes + ADR-0116-ARCHITECTURE-MUTATION-AND-COUNTERFACTUAL-VERIFICATION
based_on: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/07-MUTATION-COUNTERFACTUAL-PROOF-CARRYING.md
---

# arch-spec-A3-S6 — Critical Mutation Probes

## Intent

AC6 of the Architecture Conformance track (per `09-ROADMAP.md`):

> For critical invariants, prove the guard catches an injected violation in
> sandbox. Start with provider type leak, Alignment→Governance, workbook write
> and second canonical writer.

A fitness rule existing is weaker evidence than proving it *detects* the
forbidden change. AC6 delivers the mechanism plus the four initial critical
mutations.

Inherits `arch-spec-038` (stays `proposed` upstream). Counterfactual
requirements (AC-038-004..006) are **AC13**, not this cycle.

## Upstream coverage

| Upstream | In scope | Where |
|---|---|---|
| AC-038-001 sandbox only | yes | REQ-AC6-001..003 |
| AC-038-002 names expected guard + records detection | yes | REQ-AC6-004..008 |
| AC-038-003 four critical mutations | yes | REQ-AC6-010..013 |
| AC-038-004..006 counterfactual | no — AC13 | — |

UAT: **AC-UAT-010**.

## Resolved open questions

(Q1) New module `crates/sddk-engine/src/architecture_mutation/`.
(Q2) In-memory disposable sandbox only; no filesystem mutation.
(Q3) No `ArchitectureClaim` construction; probes + optional witnesses only.
(Q4) CLI surface deferred, as with AC4.
(Q5) AC6 runs before AC4: it produces the `contradiction_witnesses` AC4 consumes.

## Scope (must)

- `MutationSandbox` = disposable in-memory `path -> content` map.
- `MutationKind` closed, 4 variants.
- `MutationInjection` closed application modes; deterministic.
- `MutationGuard { id, scope, check }` with `GuardCheck` closed (2 variants).
- `MutationSpec { id, kind, target_path, injection, expected_guard,
  expected_contract }`.
- `run_mutation_probe` / `run_mutation_suite` — pure.
- `critical_mutations()` returning the four upstream-named mutations.
- `MutationSuiteReceipt` with `all_detected` and a deterministic `digest`.
- `witnesses()` bridging detected probes to AC4's `contradiction_witnesses`.

## Scope (must NOT)

- NO filesystem write, NO working-tree mutation, NO persistence.
- NO counterfactual candidate projection (AC13).
- NO import of `paradigm_profile` (AC3), `alignment`/`debverify` (AC5),
  `provider`/`host_sdk`/`agent_host`.
- NO `ArchitectureClaim` construction; NO capability grant.
- NO real-time/IO in the probe path.

## Requirements (REQ-AC6-NNN)

### Sandbox (AC-038-001)

- **REQ-AC6-001** — `MutationSandbox` is an in-memory `BTreeMap<String, String>`.
  It has no `save`/`flush`/`write` method; the module performs no IO.
  Pin: `anti_encroachment_no_filesystem_writes`.
- **REQ-AC6-002** — `run_mutation_probe` never mutates its input sandbox: the
  caller's map is byte-identical after the call.
  Pin: `acceptance_probe_does_not_mutate_input_sandbox`.
- **REQ-AC6-003** — Injecting into an absent target path creates it; injecting
  into an existing path appends after the existing content.

### Mutation shape (AC-038-002)

- **REQ-AC6-004** — `MutationInjection` is closed with exactly 3 variants:
  `AppendLine(String)`, `PrependLine(String)`,
  `InsertBeforeFirstLineContaining { needle: String, line: String }`.
  Pin: `acceptance_injection_closed_three`.
- **REQ-AC6-005** — `InsertBeforeFirstLineContaining` whose `needle` is absent
  leaves the content unchanged and the probe records `mutation_applied: false`.
  Pin: `acceptance_injection_not_applied_when_needle_absent`.
- **REQ-AC6-006** — `MutationProbe` carries `mutation_applied: bool`,
  `expected_guard: GuardId`, `detected: bool`, `evidence: Vec<GuardHit>`.
  Pin: `acceptance_probe_shape` (compile-time + a run).
- **REQ-AC6-007** — `detected == true` iff the guard produced ≥1 hit on the
  mutated sandbox. `detected == false` when `mutation_applied == false`.
  Pin: `acceptance_detected_iff_guard_hits`.
- **REQ-AC6-008** — `GuardHit` records `path`, `line` (1-based) and the matched
  text, so `evidence` is actionable without re-running.
  Pin: `acceptance_guard_hit_names_path_and_line`.

### Guards

- **REQ-AC6-009** — `GuardCheck` is closed with exactly 2 variants:
  `ForbiddenLines { forbidden: Vec<String> }` (fail on any line containing a
  forbidden substring) and `MaxOccurrences { pattern: String, max_allowed:
  usize }` (fail when the occurrence count exceeds `max_allowed`).
  Pin: `acceptance_guard_check_closed_two`.
- **REQ-AC6-010** — A guard's `scope` is a list of path prefixes; only paths
  starting with one of them are inspected. An empty scope means the whole
  sandbox.
  Pin: `acceptance_guard_scope_limits_inspection`.

### The four critical mutations (AC-038-003)

- **REQ-AC6-011** — `MutationKind` is closed with exactly 4 variants:
  `ProviderTypeLeak`, `AlignmentToGovernance`, `WorkbookCanonicalWrite`,
  `SecondCanonicalWriter`.
  Pin: `acceptance_mutation_kind_closed_four`.
- **REQ-AC6-012** — `critical_mutations()` returns exactly 4 specs, one per
  kind, each with a non-empty target path, injection and expected guard.
  Pin: `acceptance_critical_mutations_four_and_well_formed`.
- **REQ-AC6-013** — Every critical mutation is detected by its own guard on a
  sandbox seeded with a minimal representative source tree. This is
  **AC-UAT-010**.
  Pin: `acceptance_all_critical_mutations_detected` (and per-mutation tests).
- **REQ-AC6-014** — Every critical guard has a **negative control**: on the
  unmutated seed the guard produces zero hits.
  Pin: `acceptance_critical_guards_have_no_false_positive`.

### Suite + determinism

- **REQ-AC6-015** — `run_mutation_suite(sandbox, specs)` returns a
  `MutationSuiteReceipt` with `probes` sorted by `MutationId`, `all_detected`
  and a sha256 `digest` over the canonical receipt payload.
  Pin: `acceptance_suite_is_sorted_and_deterministic`.
- **REQ-AC6-016** — `all_detected == true` iff every probe has
  `detected == true`. `undetected()` lists the failing ids, sorted.
  Pin: `acceptance_all_detected_semantics`.
- **REQ-AC6-017** — Two runs over identical `(sandbox, specs)` yield
  byte-identical digests.
  Pin: `acceptance_suite_digest_is_stable`.

### AC4 seam

- **REQ-AC6-018** — `MutationSpec.expected_contract: Option<ContractId>`;
  `MutationSuiteReceipt::witnesses()` returns the contracts of detected probes
  that declare one, sorted and deduplicated, in the shape AC4's
  `contradiction_witnesses` consumes.
  Pin: `acceptance_witnesses_from_detected_probes`.

### Anti-encroachment

- **REQ-AC6-019** — `architecture_mutation` imports none of `paradigm_profile`,
  `alignment`, `debverify`, `provider`, `host_sdk`, `agent_host`,
  `effective_instructions`, `capability`.
  Pin: `anti_encroachment_no_forbidden_imports`.
- **REQ-AC6-020** — No `ArchitectureClaim` construction; no `std::fs`,
  `std::io::Write`, `File::create` in the module.
  Pin: `anti_encroachment_no_filesystem_writes`.

## Acceptance tests (planned, 21)

| # | Test | REQ |
|---|---|---|
| 1 | `acceptance_injection_closed_three` | 004 |
| 2 | `acceptance_injection_not_applied_when_needle_absent` | 005 |
| 3 | `acceptance_probe_shape` | 006 |
| 4 | `acceptance_detected_iff_guard_hits` | 007 |
| 5 | `acceptance_guard_hit_names_path_and_line` | 008 |
| 6 | `acceptance_guard_check_closed_two` | 009 |
| 7 | `acceptance_guard_scope_limits_inspection` | 010 |
| 8 | `acceptance_mutation_kind_closed_four` | 011 |
| 9 | `acceptance_critical_mutations_four_and_well_formed` | 012 |
| 10 | `acceptance_probe_does_not_mutate_input_sandbox` | 002 |
| 11 | `acceptance_inject_creates_absent_target` | 003 |
| 12 | `acceptance_provider_type_leak_detected` | 013 (AC-UAT-010) |
| 13 | `acceptance_alignment_to_governance_detected` | 013 |
| 14 | `acceptance_workbook_canonical_write_detected` | 013 |
| 15 | `acceptance_second_canonical_writer_detected` | 013 |
| 16 | `acceptance_all_critical_mutations_detected` | 013 |
| 17 | `acceptance_critical_guards_have_no_false_positive` | 014 |
| 18 | `acceptance_suite_is_sorted_and_deterministic` | 015/017 |
| 19 | `acceptance_all_detected_semantics` | 016 |
| 20 | `acceptance_witnesses_from_detected_probes` | 018 |
| 21 | `anti_encroachment_no_forbidden_imports` | 019 |
| 22 | `anti_encroachment_no_filesystem_writes` | 001/020 |
| 23 | `bonus_max_occurrences_boundary` | 009 |
| 24 | `bonus_empty_suite_is_vacuously_detected` | 016 |

## Determinism contract

Two `run_mutation_suite` calls over identical `(sandbox, specs)` yield
byte-identical `digest`, identical `probes` ordering, and identical
`undetected()`/`witnesses()`.

## State classes (ADR-0095)

| Value | Class |
|---|---|
| `MutationSandbox` | EPHEMERAL (in-memory only, never persisted) |
| `MutationProbe`, `MutationSuiteReceipt`, `GuardHit`, `MutationSpec` | EPHEMERAL (computed values) |
| `MutationKind`, `GuardCheck`, `MutationInjection`, `GuardId` | variant enums / typed newtypes |
