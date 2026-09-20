# SCOPE-CONTRACT — S2 — UAT C05, C07, C09

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s2-uat-c05-c07-c09`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Status:** planning + implementation in one session.

## §1 Goal

Cover `PR-UAT-C05, C07, C09` from
`docs/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md`
§E end-to-end. Three orthogonal invariants:

- **C05 — no DTO leakage through `analyze_impact`.** SDDK surfaces are
  free of any `cognicode::*` / `chronos::*` / `prost::*` / `tonic::*`
  type in the post-`analyze_impact` state.
- **C07 — restart/reconnect with stable refs.** Across a simulated
  provider restart mid-request, stable references retain their
  declared semantics (`restart_observed` is set; the prior digest is
  preserved; a reconnect re-issues the same snapshot digest).
- **C09 — contradiction preservation.** When provider evidence
  contradicts an existing `KnowledgeAssertion`, the prior assertion
  is **not** overwritten. The basis is invalidated with
  `InvalidationReason::Contradicted` and a **reconciliation note** is
  registered (second assertion with a distinct `KnowledgeId`) so the
  contradiction is auditable, not erased.

## §2 UAT rows in scope

| UAT id | Scenario | Expected invariant | Test |
|---|---|---|---|
| `PR-UAT-C05` | Impact analysis crosses dependency boundary | provider result maps to SDDK Evidence/Knowledge without provider DTO leakage | `t_uat_c05_no_dto_leakage_through_analyze_impact` |
| `PR-UAT-C07` | Provider restart/reconnect | SDDK resumes with explicit lifecycle transition; prior stable refs retain declared semantics | `t_uat_c07_restart_preserves_prior_stable_refs` |
| `PR-UAT-C09` | Provider evidence contradicts existing KnowledgeAssertion | contradiction is preserved and reconciled explicitly, not overwritten | `t_uat_c09_contradiction_preserves_prior_assertion_and_records_reconciliation` |

## §3 Hard constraints

- **C1**: Zero new ADTs added to `sddk-engine` source unless an existing
  one cannot represent the invariant. If so, STOP and report.
- **C2**: Zero modifications to `arch-spec-021` or
  `arch-acceptance-coverage-001` (those are normative contracts).
- **C3**: Zero modifications to `Evidence` / `EvidencePosture`
  (A4 §2 freeze).
- **C4**: Zero modifications to `sddk-storage`. If durability of the
  reconciliation note requires a new schema, STOP and report; S4 owns
  durability.
- **C5**: Zero provider DTO types in `sddk-domain` (continuation of
  CC-S0/CC-S1 invariant).
- **C6**: No new dependency added.
- **C7**: Workspace test green; CC-S0 + CC-S1 + S1 still pass.

## §4 STOP conditions

| Condition | Action |
|---|---|
| Discovery that the existing `KnowledgeBasis::invalidate(Contradicted, _)` mechanism cannot preserve the prior assertion's content (e.g. because `assertions` is consumed or hashed out) | **STOP** that line only; report with proposed schema. Continue S6 (fake relocation, independent). |
| Discovery that `analyze_impact` cannot return without leaking a provider DTO into SDDK | **STOP** that line only; report. |
| `cargo fmt --check` / `cargo clippy -p sddk-engine --all-targets -- -D warnings` fails | Fix and continue (not a STOP). |
| Discovery that the existing `KnowledgeId` namespace cannot host a "reconciliation note" assertion alongside the contradicted one without collision | **STOP** that line; propose a `KnowledgeId` derivation rule. |

The operator's instruction in this session explicitly authorises **only
that line to stop** if C09 reveals an architectural gap; the rest of
the macro-ciclo (S3, S4, S5, S6) continues independently where its
dependencies allow.

## §5 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../slices/s2-uat-c05-c07-c09/SCOPE-CONTRACT.md` | ✅ |
| Test binary with 3 UAT tests | `crates/sddk-engine/tests/a6_s2_uat_c05_c07_c09.rs` | 🔲 |
| UAT evidence rows | `tests/cycle-artifacts/.../slices/s2-uat-c05-c07-c09/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `tests/cycle-artifacts/.../slices/s2-uat-c05-c07-c09/RECEIPT.md` | 🔲 |
| 1 commit `feat(engine)` (test-only, no source mods expected) | — | 🔲 |

## §6 Out of scope

- **AC10** (S3), **durability** (S4), **real CogniCode EXT** (S5),
  **fake relocation** (S6), **closeout** (S7).
- New ADTs; new event classes; new storage schemas. If any of these
  appear necessary, STOP and report.

## §7 References

- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/SCOPE-CONTRACT.md` §S2 (macro-cycle plan)
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md` §E, §I
- `crates/sddk-engine/src/knowledge.rs` (existing `KnowledgeBasis`,
  `InvalidationReason::Contradicted`, `InvalidatedKnowledgeBasis`,
  `KMT`)
- `crates/sddk-engine/src/code_intelligence_port.rs` (existing
  `AnalysisResult`, `ObservationSet`, `ProviderKind`)
- `tests/cycle-artifacts/.../slices/s1-uat-coverage-fake/RECEIPT.md` (S1 close)
