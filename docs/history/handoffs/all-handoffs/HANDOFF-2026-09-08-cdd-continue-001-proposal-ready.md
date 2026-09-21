# CDD-CONTINUE-001 — Continuation Candidate & Resume View (Proposal ready)

- status: PROPOSAL READY
- cycle: CDD-CONTINUE-001 (order 269, **H4 closing item**, horizon H4 — AgentHost, Context Compiler & Decision Memory)
- depends on: CDD-HANDOFF-001 (v1.99.0, SHIPPED)
- ADR: `~/.sddk-knowledge/sddk-framework/adrs/ADR-085-CONTINUATION-CANDIDATE-RESUME-VIEW.md` (proposed, P14-a)
- spec: `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ContinuationCandidate.md` (proposed, 350+ lines)

> Note: prior session summary referenced "CDD-AUDIT-001" as a closing
> item, but the actual H4 spine enumerated in
> `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-TIMELINE.md`
> closes H4 at order 269 = `CDD-CONTINUE-001`. We proceed with the
> canonical cycle name from the spine.

## What this cycle delivers

A typed frontier of next-action candidates that a fresh LLM or
returning operator can act on without re-deriving state from
prose:

1. **`ContinuationCandidate` struct** (15 fields) — action, kind,
   prerequisites, pros/cons, risks, reversibility, confidence,
   uncertainty, evidence_refs, expected_value/cost, blocks/unlocks,
   human_authority_required, source_ref, produced_at_ms.
2. **`ContinuationKind` enum** — Resume, Dispatch, Decide,
   OpenCycle, Audit, Defer (`#[non_exhaustive]`).
3. **`Reversibility` enum** — FullyReversible, PartiallyReversible,
   Irreversible, Unknown.
4. **`ResumeView` struct** — view_id, head_ref, frontier,
   blockers, pending_decisions, must_read, recovery_state_ref,
   generated_at_ms.
5. **`FrontierStore` trait** + `InMemoryFrontierStore` reference impl.
6. **`FrontierValidator`** with 7 invariant checks.
7. **`FrontierError` closed-set taxonomy** (6 variants,
   `#[non_exhaustive]`).

## Shape chosen: P14-a (recommended, adopted)

- Two plain-data structs (mirrors existing substrate pattern).
- Two enums for kind and reversibility.
- One trait seam (`FrontierStore`).
- One validator struct with deterministic per-call API.
- Structural validation; live ranking is post-H4 policy.

P14-b (free function) and P14-c (enum-tagged candidate) rejected —
see ADR-085 §3.

## Acceptance criteria (from spec §Invariants)

All 7 invariants machine-validatable:

1. Non-empty action
2. Bounded confidence [0, 1]
3. Finite cost/value (no NaN/Inf)
4. Irreversible ⇒ human authority required
5. Reversibility mismatch surfaced
6. Non-empty source_ref
7. produced_at_ms non-zero

10 RED->GREEN tests in `continuation_candidate.rs`.

## 11-field candidate coverage

All 11 backlog bullets covered:

| Backlog bullet | Field |
|---|---|
| action/kind/prerequisites | action, kind, prerequisites |
| pros/cons | pros, cons |
| risks | risks |
| reversibility | reversibility |
| confidence/uncertainty | confidence, uncertainty |
| evidence refs | evidence_refs |
| expected value/cost | expected_value, expected_cost |
| what it blocks/unlocks | blocks, unlocks |
| human-authority requirement | human_authority_required |
| cold start semantic equivalents | ResumeView |
| memory log/graph/diff/show/etc. | Out of scope (CDD-MEMORY-001) |

## Apply-phase deliverables (next session)

- `crates/sddk-engine/src/continuation_candidate.rs` (~500 lines):
  types + `FrontierValidator` + `InMemoryFrontierStore`
- Re-export from `lib.rs`
- 10 unit tests in mod
- Target version: **v1.100.0** (next minor after v1.99.0)

## Toxicology check (decision sanity)

- All 11 backlog bullets covered verbatim.
- Closed-set error taxonomy (`#[non_exhaustive]`) prevents
  downstream breakage.
- `Irreversible ⇒ human_authority_required` is structural
  cross-validation that aligns with H4 exit gate.
- Orthogonal to existing identity / role / envelope / context.
- Memory traversal left to CDD-MEMORY-001/002 (267/268).

## H4 close

Once shipped, H4 spine (orders 230-269) will be complete:
- AGENT-HOST-001/002 (v1.94.0, v1.95.0)
- CTX-COMPILER-001/002 (v1.96.0, v1.97.0)
- CDD-ROLE-001 (v1.98.0)
- CDD-HANDOFF-001 (v1.99.0)
- CDD-CONTINUE-001 (v1.100.0)

Plus the two higher orders (267/268) left for natural followup.

## State at handoff

- Spec: PROPOSED in `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ContinuationCandidate.md`
- ADR: PROPOSED in `~/.sddk-knowledge/sddk-framework/adrs/ADR-085-CONTINUATION-CANDIDATE-RESUME-VIEW.md`
- Spine: CDD-CONTINUE-001 PROPOSED
- Local workspace: clean
- Binary: `sddk 1.99.0`
