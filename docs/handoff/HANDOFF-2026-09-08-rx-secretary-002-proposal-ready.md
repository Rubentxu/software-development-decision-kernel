# RX-SECRETARY-002 — Secretary L1 Closed-Set Proposal (Proposal ready)

- status: PROPOSAL READY
- cycle: RX-SECRETARY-002 (order 310, horizon H5)
- depends on: RX-SECRETARY-001 (v1.104.0)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-090-SECRETARY-L1-CLOSED-SET-PROPOSALS.md` (P19-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-SecretaryL1ClosedSetProposals.md`

## Deliverables

- ClosedSetKind (5 variants, #[non_exhaustive])
- ProposalTemplate + BoundedWindow (issued_by/valid_from/until/max_uses)
- SecretaryId newtype
- SecretaryProposal
- SecretaryL1Engine with templates/proposals/uses registry
- SecretaryL1Error (8 variants, #[non_exhaustive])
- 8 invariants machine-validatable
- 10 RED->GREEN tests

## Backlog boundary

Proposals flow through existing CDD/HX seams; no new authority path.

## Apply target

v1.105.0 — `secretary_l1.rs` ~500 lines.
