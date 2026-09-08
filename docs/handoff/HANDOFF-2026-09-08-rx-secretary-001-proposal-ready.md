# RX-SECRETARY-001 — Secretary L0 Reactive Rules (Proposal ready)

- status: PROPOSAL READY
- cycle: RX-SECRETARY-001 (order 300, horizon H5)
- depends on: HX-RESUME-001 (v1.103.0)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-089-SECRETARY-L0-REACTIVE-RULES.md` (proposed, P18-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-SecretaryL0ReactiveRules.md` (proposed)

## Deliverables

- ReactiveTrigger (6 variants, #[non_exhaustive])
- ReactiveSignal (5 variants, #[non_exhaustive])
- ReactiveMatcher (5 variants)
- ReactiveRule + ReactiveEvent
- SecretaryL0Engine with deterministic per-rule cooldown
- SecretaryL0Error (3 variants, #[non_exhaustive])
- 8 invariants machine-validatable
- 10 RED->GREEN tests

## Backlog boundary

"Secretary is never a second orchestrator, memory owner, or
authority path." Engine emits signals only; no mutation API.

## Apply target

v1.104.0 — `secretary_l0.rs` ~500 lines.
