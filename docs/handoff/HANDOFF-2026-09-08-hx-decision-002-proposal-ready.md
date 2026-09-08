# HX-DECISION-002 — Risk-Sensitive Approval Policy (Proposal ready)

- status: PROPOSAL READY
- cycle: HX-DECISION-002 (order 280, horizon H5 — Human & Reactive Control)
- depends on: HX-DECISION-001 (v1.101.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-087-RISK-APPROVAL-POLICY.md` (proposed, P16-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-RiskApprovalPolicy.md` (proposed, 250+ lines)

## What this cycle delivers

A typed approval policy engine that gates every `HumanDecisionRequest`
before port admission:

1. **`RiskTier`** enum — Trivial/Low/Medium/High/Critical
   (strict partial order, `#[non_exhaustive]`).
2. **`ApprovalVerdict`** enum — AutoApprove/Clear/NeedsApproval/
   Escalate/Block (`#[non_exhaustive]`).
3. **`ApprovalPolicy`** struct — 6 fields: policy_id,
   tier_cap_for_auto, evidence_required_above,
   budget_review_threshold, irreversible_escalates,
   per_cycle_limit.
4. **`ApprovalContext`** struct — 6 fields: risk_tier,
   reversibility, evidence_refs, budget_tokens, actor,
   actions_seen_in_cycle.
5. **`ApprovalDecision`** struct — verdict, reasons,
   required_authority.
6. **`ApprovalEngine`** struct with deterministic per-call
   evaluation and threshold validation at construction.
7. **`PolicyStore` trait** + `InMemoryPolicyStore` reference.
8. **`PolicyError` closed-set taxonomy** (3 variants,
   `#[non_exhaustive]`).

## Shape chosen: P16-a (recommended, adopted)

- Three plain-data structs (mirrors existing pattern).
- Two enums (tier + verdict).
- One trait seam (`PolicyStore`).
- One engine struct with deterministic evaluation.
- Reuses `Reversibility` from CDD-CONTINUE-001 / HX-DECISION-001.

P16-b (free function) and P16-c (actor) rejected — see ADR-087 §3.

## Acceptance criteria (8 invariants)

1. Tier discipline at construction
2. Evidence required above threshold
3. Budget review above threshold
4. Irreversible escalation opt-in
5. Per-cycle limit blocks
6. Auto-approve below cap
7. Clear ceiling between cap and threshold
8. Authority requirement for `NeedsApproval`/`Escalate`/`Block`

10 RED->GREEN tests in `risk_approval_policy.rs`.

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/risk_approval_policy.rs` (~500 lines):
  types + `ApprovalEngine` + `InMemoryPolicyStore`
- Re-export from `lib.rs`
- 10 unit tests in mod
- Target version: **v1.102.0** (next minor after v1.101.0)

## Toxicology check

- 8 invariants machine-validatable.
- Closed-set error taxonomy; forward-compatible enums.
- Construction-time validation surfaces threshold misuse at
  engine build, not on first evaluate call.
- Reuses `Reversibility` — one semantic across H4/H5.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-RiskApprovalPolicy.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-087-RISK-APPROVAL-POLICY.md`
- Spine: HX-DECISION-002 PROPOSED
- Local workspace: clean
- Binary: `sddk 1.101.0`
