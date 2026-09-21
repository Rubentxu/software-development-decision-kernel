# RX-SECRETARY-003 — Secretary L2 Bounded Cognitive Replan (Proposal ready, H5 closing)

- status: PROPOSAL READY
- cycle: RX-SECRETARY-003 (order 320, horizon H5 — **CLOSING H5**)
- depends on: RX-SECRETARY-002 (v1.105.0)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-091-SECRETARY-L2-BOUNDED-REPLAN.md` (P20-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-SecretaryL2BoundedReplan.md`

## Deliverables

- CognitiveDrainage (4 fields: budget_tokens, budget_iterations,
  evidence_budget, policy_ceiling)
- CognitiveInput (cycle_ref, head_ref, drainage, exhausted_*,
  remaining_frontier, human_authority_token)
- CognitiveReplanVerdict { GiveUp, Recommend(...), DeferredUntil{...} }
- CognitiveRecommendation (id, summary, candidate_summary,
  reversibility, evidence_required, human_authority_required,
  produced_at_ms)
- CognitiveReplan + SecretaryL2ReplanEngine
- SecretaryL2Error (4 variants, #[non_exhaustive])
- 8 invariants machine-validatable
- 10 RED->GREEN tests

## Backlog boundary

L2 emits only ContinuationCandidate-shaped recommendations through
existing seams; no new authority path.

## Apply target

v1.106.0 — `secretary_l2_replan.rs` ~500 lines. **Closes H5.**
