# HX-RESUME-001 — Human Resume View & Rehydration Plan (Proposal ready)

- status: PROPOSAL READY
- cycle: HX-RESUME-001 (order 290, horizon H5 — Human & Reactive Control)
- depends on: HX-DECISION-002 (v1.102.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-088-HUMAN-RESUME-VIEW.md` (proposed, P17-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-HumanResumeView.md` (proposed, 250+ lines)

## What this cycle delivers

A typed human-facing projection of `ResumeView`:
`ResumeInfo` + `RehydrationPlan` + `RehydrationStep`:

1. **`ResumeInfo`** struct — info_id, cycle_ref, head_ref,
   human_id, visited_at_ms, last_decisions (DecisionSummary),
   open_blockers, pending_asks, last_view_view_id, produced_at_ms.
2. **`DecisionSummary`** struct — decision_ref, verdict, at_ms,
   digest.
3. **`RehydrationPlan`** struct — plan_id, info_id, steps,
   authority_required, expires_at_ms, produced_at_ms.
4. **`RehydrationStep` enum** (6 variants, `#[non_exhaustive]`):
   ReadContext, AcknowledgeDecision, ProvideApproval,
   SurfaceQuestion, WaitFor, PersistEvidence.
5. **`HumanResumeValidator`** structural validator with 8 invariants.
6. **`RehydrationStore` trait** + `InMemoryRehydrationStore` reference.
7. **`HumanResumeError`** closed-set taxonomy (6 variants,
   `#[non_exhaustive]`).

## Shape chosen: P17-a

Mirrors existing substrate pattern (struct + traits + validator).
P17-b (free function) and P17-c (composition with CDD-CONTINUE)
rejected — see ADR-088 §3.

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/human_resume_view.rs` (~500 lines):
  types + validator + store
- Target version: **v1.103.0** (next minor after v1.102.0)

## Toxicology check

- 8 invariants machine-validatable.
- Closed-set error taxonomy.
- Reuses `Reversibility` indirectly via HX-DECISION-001 receipts.
- Validator + executor are decoupled — substrate ships without
  runtime execution.

## State at handoff

- Spec: PROPOSED
- ADR: PROPOSED
- Spine: HX-RESUME-001 PROPOSED
- Binary: `sddk 1.102.0`
