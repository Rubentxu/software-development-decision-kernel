---
id: ADR-0111-EXPLICIT-APPROVAL-POLICY
status: accepted
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: 2026-09-14
accepted_by_cycle: "p-63676b11dc0ef88f/production-readiness-a0"
implementation_evidence:
  - "crates/sddk-engine/src/authority_engine.rs — `admit_traced` step 5 triggers RequireApproval only from `PolicySnapshot.approval_required_for`"
  - "crates/sddk-engine/src/authority_engine/bridge.rs — `default_approval_required_for()` + `default_policy_for_surface`"
  - "crates/sddk-cli/src/admission.rs — `ENFORCEMENT_STAGE = All`; action-driven approval matrix + zero-bypass tests"
superseded_by: []
related_adrs:
  - "ADR-0102-UNIFIED-AUTHORITY-ENGINE"
  - "ADR-0001-ADR-PROMOTION-PROCESS"
stale_after: 2027-09-14
---

# ADR-0111 — Explicit Approval Policy

## Context

The 2026-09-14 production-readiness audit (`PR-GAP-006`) found that the
AuthorityEngine treated `RiskBand::High` as an automatic synonym of
`RequireApproval`:

```text
if policy.approval_required_for.contains(action) || policy.risk_band == High
```

Because `bridge::default_policy_for_surface` classifies six surfaces as High
(`cycle_state`, `gate_receipts`, `plan_revisions`, `transition_records`,
`framework_bundle`, `github_releases`), completing the M4 `LowMedium → All`
enforcement rollout with that rule made **every action on those surfaces**
require human approval. A measured flip to `All` broke 16 core CLI workflows
(gate evaluation, release apply, phase remediate, dogfood) by turning routine
operations into approval-gated ones.

The defect is not the approval loop nor the choke point: it is that risk
classification and approval necessity were conflated.

## Decision

Separate three concepts explicitly:

```text
RiskBand                      = risk classification / policy input
approval_required_for         = explicit policy decision: which actions require approval
AdmissionDecision::RequireApproval = authoritative result of the policy
EnforcementStage::All         = every RequireApproval actually blocks
```

**High risk does not imply human approval by itself. Approval is an explicit
policy decision over action/context. Risk is an input to policy, not authority
on its own.**

Concretely:

1. `DefaultAuthorityEngine::admit_traced` raises `RequireApproval` **only** when
   `policy.approval_required_for` contains the action. The `risk_band == High`
   disjunction is removed. `RiskBand` remains available for classification,
   explanation, policy selection and future Governance rules.
2. `bridge::default_approval_required_for()` is the explicit default policy set.
   It lists only demonstrably dangerous, hard-to-reverse actions:
   `CycleSupersede`, `CliRelease`, `CliShip`, `PackInstall`.
3. `default_policy_for_surface` keeps the surface→band classification and sets
   `approval_required_for` from (2). Routine High-band operations
   (`CycleTransition`, `CyclePause`, `CycleResume`, gate receipts, plan
   revisions, derived transition records) are **not** approval-gated.
4. `EnforcementStage::All` is the production stage: every `RequireApproval`
   blocks through the live M2 approval loop; a granted approval matching the
   stable request hash re-admits as `Allow`.

## Non-goals

- No runtime-configurable policy language, no policy daemon, no second policy
  abstraction (that would be option C, deferred).
- The surface→band table itself is not changed; only its coupling to approval
  is removed.

## Consequences

- Routine workflows on High-band surfaces stay usable without artificial
  approval; genuinely dangerous actions require explicit approval.
- `authority.admission.decided` is recorded for blocking/approval decisions;
  routine `Allow` admissions are not recorded (as before the advisory path
  existed).
- `RiskBand` is now purely informational; any future rule that wants to gate an
  action must add it to `approval_required_for` (or a successor explicit field).

## Compliance

Each new core abstraction states what it replaces: this ADR replaces the
implicit `risk_band == High ⇒ RequireApproval` rule in the engine decision path.
