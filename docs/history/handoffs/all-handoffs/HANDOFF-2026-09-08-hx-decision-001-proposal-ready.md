# HX-DECISION-001 — Human Decision Substrate (Proposal ready, H5 opening)

- status: PROPOSAL READY
- cycle: HX-DECISION-001 (order 270, **H5 opening item**, horizon H5 — Human & Reactive Control)
- depends on: CDD-CONTINUE-001 (v1.100.0, SHIPPED — closing of H4)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-086-HUMAN-DECISION-SUBSTRATE.md` (proposed, P15-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-HumanDecision.md` (proposed, 350+ lines)

## What this cycle delivers

A typed human-in-the-loop substrate that shares one policy /
authorization / provenance path with the rest of the kernel:

1. **`HumanDecisionRequest`** struct — request_id, cycle_ref,
   question, context_refs, options, default_rationale,
   reversibility, requested_at_ms, expires_at_ms, requested_by.
2. **`HumanDecisionOption`** struct — option_id, label, summary,
   reversibility_override.
3. **`HumanDecision` enum** — Approved, Rejected, Deferred,
   Escalated, Selected (`#[non_exhaustive]`).
4. **`HumanDecisionReceipt`** — receipt_id, request_id, decision,
   decided_at_ms, digest, authority_path, evidence_refs.
5. **`HumanDecisionPort` trait** + `InMemoryHumanDecisionPort`
   reference impl (Mutex<BTreeMap>-backed).
6. **`HumanDecisionError`** closed-set taxonomy (8 variants,
   `#[non_exhaustive]`).
7. **Reused** `Reversibility` from CDD-CONTINUE-001 (v1.100.0) —
   one semantic across H4/H5.

## Shape chosen: P15-a (recommended, adopted)

- Three plain-data structs (one for the request, one for options,
  one for the receipt).
- One enum (`HumanDecision`) classifying decisions; non_exhaustive.
- One trait seam (`HumanDecisionPort`) for runtime integration.
- One validator/receiver sequence inside the port.
- Structural validation; risk policy is HX-DECISION-002.

P15-b (free function) and P15-c (trait-only abstraction) rejected —
see ADR-086 §3.

## Acceptance criteria (from spec §Invariants)

All 8 invariants machine-validatable:

1. Non-empty question + non-empty options
2. Unique option_id inside request
3. Reversibility severity override check
4. expires_at_ms ≥ requested_at_ms (when both present)
5. Single-submit semantics (no double decision)
6. Receipt authority_path non-empty
7. Receipt digest non-empty
8. Submit-after-expiry rejected

10 RED->GREEN tests in `human_decision.rs`.

## H5 opening

This cycle opens H5 (Human & Reactive Control). Followups:
- HX-DECISION-002 (280) — risk policy + CLI/AgentHost adapters
- HX-RESUME-001 (290) — `ResumeInfo`/`RehydrationPlan` over generic
  `ResumeView`
- RX-SECRETARY-001/002/003 (300-320) — deterministic L0 reactions
  + L1 proposals + bounded replan

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/human_decision.rs` (~500 lines): types +
  `HumanDecisionPort` trait + `InMemoryHumanDecisionPort`
- Re-export from `lib.rs`
- 10 unit tests in mod
- Target version: **v1.101.0** (next minor after v1.100.0)

## Toxicology check (decision sanity)

- All 8 invariants machine-validatable.
- Closed-set error taxonomy (`#[non_exhaustive]`) prevents
  downstream breakage.
- Reuses `Reversibility` from CDD-CONTINUE-001 (single semantic).
- Single-submit enforced; receipt immutable.
- Adapter layer deferred to HX-DECISION-002 — port seam is the
  only integration surface this cycle commits.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-HumanDecision.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-086-HUMAN-DECISION-SUBSTRATE.md`
- Spine: HX-DECISION-001 PROPOSED (H5 OPENING)
- Local workspace: clean
- Binary: `sddk 1.100.0`
