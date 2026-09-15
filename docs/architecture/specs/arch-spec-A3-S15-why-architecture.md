---
id: arch-spec-A3-S15-why-architecture
title: Architecture provenance traversal (why)
status: accepted
cycle: p-63676b11dc0ef88f/a3-15-why-architecture
based_on: arch-spec-A3-S14-architecture-findings + ADR-0120 + arch-spec-033 (single semantic graph)
---

# arch-spec-A3-S15 — `why architecture` provenance traversal

Answers: **"why does SDDK claim this about the architecture?"**

READ / EXPLAIN only. Not verify, not deb-verify, not governance. It reuses what
AC1–AC5 already computed and navigates provenance; it runs no second audit and
creates no authority.

Target chain:

```text
finding → claim assessment → architectural contract → decision/spec → evidence → software relation/unit
```

## Naming disposition (recorded, not reopened)

`sddk graph why` exists and is canonical **for the event-ledger reactive graph**.
The architecture substrate is not in that graph: the AC2 overlay is an in-memory
projection rebuilt from the declaration (ADR-0120 keeps the declaration as input,
the AC1 objects as authority), so there is nothing to extend.

| Surface | Substrate | Question |
|---|---|---|
| `sddk graph why --entity <kind:id>` | event-ledger graph | why does the graph hold this node/relation? |
| `sddk why architecture <id>` | AC2 overlay + AC1 claims + AC5 audit | why does SDDK claim this about the architecture? |

**Not created:** `sddk architecture why` (a second surface for one question),
`sddk why graph` (an alias of `graph why`), `sddk deb-verify`, `sddk why-not`.

## Scope

**In:** `FindingId`; the `SpecifiedBy` AC2 edge; the `ArchitectureWhy` model and
its traversal; `sddk why architecture`; the finding id printed by `findings`; the
shared `ArchitectureContext` seam.

**Out:** a `why-not` surface (the ADT is prepared, unused); any mutation,
counterfactual, time-travel, lane/DSL, governance or UI work; any new graph, DB or
index; closing the `evidence → software relation` leg; repointing the existing
`VerifiedBy → spec` relation; the two A3-S14 observations.

## Constraints

- MUST NOT run a second conceptual audit: reuse AC1's claims, AC2's overlay and
  AC5's audit as computed by the shared seam.
- MUST NOT create a new graph, DB, index or authority. `FindingId` is a derived
  digest, not stored identity (ADR-0113 / arch-spec-033).
- MUST NOT make an ADR/spec file runtime authority (ADR-0120).
- MUST NOT fabricate an edge the substrate does not have; an absent leg is
  `UNKNOWN`, visible in `unresolved_edges`.
- MUST NOT convert an audit error into `0 findings` or into an empty explanation.
- MUST NOT silently choose a namespace when an id matches both.
- MUST NOT include `message` in `FindingId`.
- MUST preserve finding→contract cardinality (1→1, 1→2, 1→3).
- MUST be deterministic in ordering and must mutate nothing.

## Requirements (REQ-A3S15-NNN)

**Identity**

- **REQ-A3S15-001** — `FindingId = sha256(domain | basis | kind | subjects |
  contract_ids)` with `message` and `severity` excluded. Pin: engine
  `acceptance_finding_id_excludes_message`, `…_excludes_severity`.
- **REQ-A3S15-002** — The basis is clock-stable, so one declaration yields the
  same finding ids at any `--now-ms`. Pin: engine
  `acceptance_finding_id_is_clock_stable`, e2e
  `why_finding_ids_are_stable_across_clocks`.
- **REQ-A3S15-003** — Two findings on one subject get different ids. Pin: engine
  `acceptance_two_findings_one_subject_do_not_collapse`.
- **REQ-A3S15-004** — `sddk architecture findings` prints the id per row, so an id
  can be fed straight back. Pin: e2e `findings_prints_ids`.

**Resolution**

- **REQ-A3S15-005** — The argument resolves against the contract namespace and the
  finding namespace explicitly; the outcome is reported as `resolved_as`. Pin: e2e
  `why_resolves_contract`, `why_resolves_finding`.
- **REQ-A3S15-006** — An id matching both namespaces is an ambiguity error
  (exit 2) naming both, never a silent choice. Pin: e2e `why_ambiguous_id_fails_closed`.
- **REQ-A3S15-007** — An id matching neither is an unknown error (exit 2) that
  states both namespaces were tried. Pin: e2e `why_unknown_id_fails_closed`.
- **REQ-A3S15-008** — A missing/invalid declaration fails closed (exit 2). Pin: e2e
  `why_declaration_invalid_fails_closed`.

**The chain**

- **REQ-A3S15-009** — For a finding, the answer preserves `kind`, `severity`,
  `subjects` and **all** `contract_ids`, with one leg per contract and an
  explanation of why each participates. Pin: e2e `why_finding_one_contract`,
  `why_shadow_authority_two_contracts`, `why_finding_three_contracts`.
- **REQ-A3S15-010** — Each contract leg carries its assessment (outcome) and its
  declared intent (decisions, specs) and its evidence. Pin: e2e
  `why_leg_carries_assessment_intent_evidence`.
- **REQ-A3S15-011** — The overlay gains `SpecifiedBy` (contract → spec), emitted
  unconditionally, so `contract → spec` is a real edge. Pin: engine
  `acceptance_specified_by_edge_is_emitted`.
- **REQ-A3S15-012** — The `evidence → software relation` leg does not exist in the
  substrate and is reported as an explicit `unresolved_edge` with a reason. Pin:
  engine `acceptance_evidence_to_software_is_unresolved`, e2e
  `why_reports_unresolved_evidence_leg`.
- **REQ-A3S15-013** — Software units reachable from a contract's claim are
  reported, so the answer reaches software where the substrate allows it. Pin: e2e
  `why_reaches_software_units`.

**Model and behaviour**

- **REQ-A3S15-014** — The model distinguishes OBSERVED/VERIFIED provenance,
  DECLARED intent, ASSESSMENT and UNKNOWN by construction (separate fields, no
  merged blob). Pin: engine `acceptance_model_keeps_provenance_classes_apart`.
- **REQ-A3S15-015** — The WHY-NOT ADT exists (missing evidence, stale basis,
  unknown relation, provider unavailable, contract not evaluable, contradiction)
  and is populated from `missing_evidence`, but no `why-not` surface is created.
  Pin: engine `acceptance_why_not_adt_is_prepared`.
- **REQ-A3S15-016** — Output is deterministic and text/JSON carry the same facts.
  Pin: e2e `why_is_deterministic`, `why_text_and_json_agree`.
- **REQ-A3S15-017** — `why` writes nothing and appends nothing to any canonical
  log. Pin: e2e `why_mutates_nothing`.
- **REQ-A3S15-018** — An audit failure is an explicit error, never `0 findings`
  and never an empty explanation. Pin: engine
  `acceptance_audit_error_is_not_empty_explanation`.
- **REQ-A3S15-019** — The three surfaces share one seam: identical
  load → validate → project → audit for `receipt`, `findings` and `why`, with the
  AC4 delta remaining caller-specific. Pin: e2e
  `surfaces_share_the_audit_seam`.

## Design sketch

**Engine, `crates/sddk-engine/src/architecture_why/` (ADR-0121):**

```rust
pub struct ArchitectureWhy {
    pub query: String,
    pub resolved_as: WhyResolvedAs,          // Contract | Finding
    pub basis: WhyBasis,                     // revision, knowledge_basis, finding_basis_digest
    pub finding: Option<WhyFinding>,         // id, kind, severity, subjects
    pub contracts: Vec<WhyContract>,         // one per contract_id — cardinality preserved
    pub unresolved_edges: Vec<WhyUnresolved>,
    pub why_not: Vec<WhyNotReason>,
}

pub struct WhyContract {
    pub contract: String,
    pub participates_because: String,        // why this contract is in the finding
    pub kind: String,
    pub subject: String,
    pub revision: String,
    pub assessment: WhyAssessment,           // ASSESSMENT
    pub intent: WhyIntent,                   // DECLARED: decisions, specs
    pub evidence: Vec<WhyEvidence>,          // OBSERVED
    pub software_units: Vec<String>,         // reached via the claim
}
```

`FindingId` lives with the finding it identifies
(`architecture_debverify/finding_id.rs`), so identity travels with the object and
`findings` can print ids without depending on `architecture_why`.

**CLI:** one `ArchitectureContext` seam (declared, now, overlay, claims, audit)
built once and consumed by `receipt`, `findings` and `why`; the delta stays
caller-specific so A3-S14's recorded `findings`-without-delta behaviour is
preserved. New root `why` verb with one `architecture` subcommand.

## Acceptance tests (planned, 22)

Engine — `architecture_debverify` (identity):

1. `acceptance_finding_id_excludes_message`
2. `acceptance_finding_id_excludes_severity`
3. `acceptance_finding_id_is_clock_stable`
4. `acceptance_two_findings_one_subject_do_not_collapse`

Engine — `architecture_graph` (the added edge):

5. `acceptance_specified_by_edge_is_emitted`
6. `acceptance_overlay_relation_vocabulary_is_fifteen`

Engine — `architecture_why`:

7. `acceptance_model_keeps_provenance_classes_apart`
8. `acceptance_evidence_to_software_is_unresolved`
9. `acceptance_why_not_adt_is_prepared`
10. `acceptance_audit_error_is_not_empty_explanation`
11. `acceptance_traversal_is_deterministic`

CLI e2e — `tests/architecture_why_cli_e2e.rs`:

12. `findings_prints_ids`
13. `why_finding_ids_are_stable_across_clocks`
14. `why_resolves_contract`
15. `why_resolves_finding`
16. `why_unknown_id_fails_closed`
17. `why_ambiguous_id_fails_closed`
18. `why_declaration_invalid_fails_closed`
19. `why_finding_one_contract`
20. `why_shadow_authority_two_contracts`
21. `why_finding_three_contracts`
22. `why_two_findings_same_subject_keep_distinct_ids`
23. `why_leg_carries_assessment_intent_evidence`
24. `why_reports_unresolved_evidence_leg`
25. `why_reaches_software_units`
26. `why_is_deterministic`
27. `why_text_and_json_agree`
28. `why_mutates_nothing`
29. `surfaces_share_the_audit_seam`

(The brief's 15 minimum cases all map onto these; 15, 18 and 29 are covered by
engine pins plus the e2e pair `why_is_deterministic` / `why_text_and_json_agree`,
and by `architecture_findings_agree_with_receipt` staying green after the seam is
extracted.)
