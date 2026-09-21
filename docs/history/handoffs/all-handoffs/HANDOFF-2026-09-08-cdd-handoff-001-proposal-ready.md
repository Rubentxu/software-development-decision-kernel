# CDD-HANDOFF-001 — Agent Contribution Envelope Substrate (Proposal ready)

- status: PROPOSAL READY
- cycle: CDD-HANDOFF-001 (order 263, horizon H4 — CDD Handoff)
- depends on: CDD-ROLE-001 (v1.98.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-084-AGENT-CONTRIBUTION-ENVELOPE.md` (proposed, P13-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentContributionEnvelope.md` (proposed, 350+ lines)

## What this cycle delivers

A typed handoff substrate that makes delegation loss-auditable
**without** copying full worker contexts into the orchestrator:

1. **`DelegationRequest`** struct — delegation_id, from_role, to_role,
   objective, context_rev, context_lease, scope, budget_tokens,
   created_at_ms.
2. **`ContextLease`** struct — lease_id, capsule_rev, digest,
   issued_to, issued_at_ms, valid_until_ms (immutable/versioned).
3. **`AgentContributionEnvelope`** — full 12-field projection
   (objective, coverage, findings, proposals/alternatives/rejections,
   pros/cons, assumptions/uncertainty, risks/open_questions,
   evidence_refs/artifact_refs, context_delta, recommendation,
   confidence, metrics).
4. **Supporting types** — `Finding { id, summary, severity }`,
   `Severity { Info, Low, Medium, High, Critical }`,
   `Proposal { id, summary, rationale }`, `Rejection`, `ContextDeltaEntry`.
5. **`EnvelopeStore` trait** + `InMemoryEnvelopeStore` reference impl.
6. **`LeaseRegistry` trait** (minimal HashMap-backed seam).
7. **`EnvelopeValidator`** with structural checks for 8 invariants.
8. **`EnvelopeError`** closed-set taxonomy (6 variants,
   `#[non_exhaustive]`).

## Shape chosen: P13-a (recommended, adopted)

- Three plain-data structs (mirrors `AgentRoleContract`,
  `ContextCapsule`, `AgentHost` patterns).
- Two trait seams (`EnvelopeStore`, `LeaseRegistry`).
- One validator struct with deterministic per-call API.
- Structural validation; semantic context-loss checks wait for
  CDD-HANDOFF-002.

P13-b (free function) and P13-c (enum-tagged envelope) rejected —
see ADR-084 §3.

## Acceptance criteria (from spec §Invariants)

All 8 invariants machine-validatable:

1. Envelope context_rev matches request
2. Envelope lease_id matches request lease
3. Envelope delegation_id matches request
4. Envelope produced_at_ms non-zero
5. Dissent preservation (alternatives ⇒ rejections)
6. Evidence refs required (structural)
7. Confidence bounded [0, 1]
8. Metrics scalar-only (no NaN)

Implemented as 10 RED->GREEN tests in `agent_contribution_envelope.rs`.

## 12-field envelope coverage

All 12 backlog bullets covered:

| Backlog bullet | Field |
|---|---|
| objective + context revision | objective, context_rev |
| coverage satisfied/missing | coverage_satisfied, coverage_missing |
| findings | findings: Vec<Finding> |
| proposals/alternatives/rejections | proposals, alternatives, rejections |
| pros/cons | pros, cons |
| assumptions/uncertainty | assumptions, uncertainty |
| risks/open questions | risks, open_questions |
| evidence refs/artifact refs | evidence_refs, artifact_refs |
| context delta | context_delta: Vec<ContextDeltaEntry> |
| recommendation/confidence/metrics | recommendation, confidence, metrics |

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/agent_contribution_envelope.rs` (~500 lines):
  types + `EnvelopeValidator` + `InMemoryEnvelopeStore` + helpers
- Re-export from `lib.rs`
- 10 unit tests in mod
- Target version: **v1.99.0** (next minor after v1.98.0)

## Toxicology check (decision sanity)

- All 12 backlog bullets covered verbatim by field set.
- Closed-set error taxonomy (`#[non_exhaustive]`) prevents
  downstream breakage when new variants are added.
- Static validation matches the exit gate "structural validation";
  semantic checks (loss guard) are post-H4 followups.
- Orthogonal to existing identity / role / capsule concerns.
- Dissent preservation is structural — alternatives without
  rejections is fail-closed at the validator.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentContributionEnvelope.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-084-AGENT-CONTRIBUTION-ENVELOPE.md`
- Spine: CDD-HANDOFF-001 PROPOSED (unchanged from baseline)
- Local workspace: clean
- Binary: `sddk 1.98.0`
