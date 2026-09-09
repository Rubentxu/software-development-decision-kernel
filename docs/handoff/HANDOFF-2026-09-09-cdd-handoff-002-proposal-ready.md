# CDD-HANDOFF-002 — Orchestration Synthesis Receipt Substrate (Proposal ready)

- status: PROPOSAL READY
- cycle: CDD-HANDOFF-002 (order 265, horizon H4 — CDD Handoff)
- depends on: CDD-HANDOFF-001 (v1.99.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-086-ORCHESTRATION-SYNTHESIS-RECEIPT.md` (proposed, P14-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-OrchestrationSynthesisReceipt.md` (proposed, 391 lines)

## What this cycle delivers

A typed receipt substrate that makes **synthesis** loss-auditable on top
of CDD-HANDOFF-001's contribution envelope substrate:

1. **`OrchestrationSynthesisReceipt`** struct — receipt_id, delegation_id
   (binds to CDD-HANDOFF-001), join_id, synthesis_owner (from
   CDD-ROLE-001), produced_at_ms, consumed[], omitted[], conflicts[],
   dissent_preserved[], coverage_loss[], risk_carry_forward[],
   evidence_carry_forward[], downstream_recommendation, confidence,
   metrics.
2. **`ContributionRef`** — delegation_id + envelope_digest + consumed_at_ms
   + consumed_by + optional omission_reason.
3. **`ConflictEntry` / `DissentEntry` / `CoverageLossEntry` /
   `RiskCarryEntry` / `EvidenceCarryEntry`** — supporting typed entries.
4. **`InformationLossGuard`** closed-set enum — 6 variants
   (`#[non_exhaustive]`):
   MandatoryRiskNoDrop, MandatoryEvidenceNoDrop, DissentNoSilentDrop,
   CoverageLossRecorded, RecommendationBackedByConsumed,
   ConflictSurfacedOrJustified.
5. **`SynthesisStore` trait** + `InMemorySynthesisStore` reference impl.
6. **`SynthesisBuilder` trait** + `DefaultSynthesisBuilder` +
   `ReceiptBuilder` fluent API — deterministic construction that
   forces `omit_with_reason` with non-empty reason and rejects
   `confidence ∉ [0,1]` and `metrics` with NaN.
7. **`SynthesisValidator`** with 6 paths (`check_guard`), deterministic
   per-call API, backref to `EnvelopeStore` (CDD-HANDOFF-001) for
   `envelope_digest` verification.
8. **`SynthesisError`** closed-set taxonomy — 8 variants,
   `#[non_exhaustive]`.

## Shape chosen: P14-a (recommended, adopted)

- One receipt struct + 5 supporting entries + 1 closed-set guard enum.
- Two trait seams (SynthesisStore, SynthesisBuilder).
- One validator struct with deterministic per-call API mirroring
  `EnvelopeValidator` / `RoleValidator` shape.
- Backref to CDD-HANDOFF-001's `EnvelopeStore` for cross-check.

P14-b (extend `AgentContributionEnvelope`) and P14-c (free-function
validator) rejected — see ADR-086 §3.

## Acceptance criteria (from spec §Invariants)

All 6 invariants of the exit gate machine-validatable:

1. Receipt delegation binding (EnvelopeDigestMismatch on mismatch)
2. Envelope backref (EnvelopeDigestMismatch on missing/forged digest)
3. MandatoryRiskNoDrop → MandatoryRiskDropped(risk_id)
4. MandatoryEvidenceNoDrop → MandatoryEvidenceDropped(ref)
5. DissentNoSilentDrop → DissentSilentlyDropped(dissent_id)
6. CoverageLossRecorded → BuilderInvariantViolated(area) on empty reason

Plus 2 supporting invariants:

7. RecommendationBackedByConsumed → RecommendationOrphan(text)
8. ConflictSurfacedOrJustified → ConflictHidden(detail)

Implemented as 10 RED->GREEN tests in
`crates/sddk-engine/tests/orchestration_synthesis_tests.rs`.

## Backlog coverage

Exit gate order 265 objective: *"Add OrchestrationSynthesisReceipt,
dissent preservation and information-loss guards."*

| Backlog bullet                                | Field / Guard |
|----------------------------------------------|---------------|
| Coordinator/orchestrator synthesis records   | `OrchestrationSynthesisReceipt` |
| Consumed contributions                       | `consumed: Vec<ContributionRef>` |
| Omitted contributions                        | `omitted: Vec<ContributionRef>` (with reason) |
| Conflicts                                     | `conflicts: Vec<ConflictEntry>` |
| Dissent preservation                          | `dissent_preserved: Vec<DissentEntry>` + `DissentNoSilentDrop` |
| Evidence                                       | `evidence_carry_forward: Vec<EvidenceCarryEntry>` + `MandatoryEvidenceNoDrop` |
| Mandatory risk cannot disappear silently     | `risk_carry_forward: Vec<RiskCarryEntry>` + `MandatoryRiskNoDrop` |
| Coverage-loss recorded                       | `coverage_loss: Vec<CoverageLossEntry>` + `CoverageLossRecorded` |

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/orchestration_synthesis.rs` (~600 lines):
  types + `ReceiptBuilder` + `DefaultSynthesisBuilder` +
  `SynthesisValidator` + `InMemorySynthesisStore`.
- Re-export from `crates/sddk-engine/src/lib.rs`.
- `crates/sddk-engine/tests/orchestration_synthesis_tests.rs`:
  10 RED->GREEN scenarios as listed above.
- Promote `ADR-086` and `REQ-OrchestrationSynthesisReceipt.md` to
  `accepted` at apply time.
- Spine reconcile: `CDD-HANDOFF-002` PROPOSED → SHIPPED with evidence
  inline.
- Target version: **v1.146.0**.

## Toxicology check (decision sanity)

- All 6 invariants of the exit gate mapped 1:1 to validators.
- `InformationLossGuard` is closed-set (`#[non_exhaustive]`) so future
  guards are additive (variant extension, not field addition).
- Receipt is plain-data + immutable on validation; storage is a
  separate trait seam; validator is deterministic per-call.
- Cross-validator composition (`EnvelopeValidator` +
  `SynthesisValidator`) is deferred to a post-H4 follow-up — this
  substrate stays focused on the receipt.
- Dissent preservation is structural: orchestrator cannot compress
  dissent away without an explicit surface in `dissent_preserved` or
  `conflicts`.
- Mandatory risk/evidence carry-forward is structural: cannot be
  silenced without explicit justification in `coverage_loss` (risk)
  or `evidence_carry_forward` (evidence).
- Orthogonal to existing role/envelope/memory concerns; no cross-cycle
  coupling.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-OrchestrationSynthesisReceipt.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-086-ORCHESTRATION-SYNTHESIS-RECEIPT.md`
- Spine: CDD-HANDOFF-002 PROPOSED (genuinely pending, reconciled
  2026-09-09)
- Local workspace: clean (housekeeping commits 38e598d + da490f5
  pushed to origin/main)
- Binary: `sddk 1.145.2`

## Path

A-lite (propose → spec → tasks → apply → verify → debt-verify →
release → archive) — new architectural substrate with 8 invariants,
2 closed-set enums, ~600 LoC, 10 RED->GREEN tests, durable contract
in vault.
