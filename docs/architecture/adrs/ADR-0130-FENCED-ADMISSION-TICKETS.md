---
id: ADR-0130-FENCED-ADMISSION-TICKETS
status: accepted
supersedes_history: false
adopted_at: 2026-09-18
adoption_cycle: p-63676b11dc0ef88f/a6-0-r4b-admission-tickets
package_local_id: null
package_source: null
accepted_at: 2026-09-18
accepted_by_cycle: p-63676b11dc0ef88f/a6-0-r4b-admission-tickets
superseded_by: []
related_adrs:
  - ADR-0097-COMMON-REVISION-SUBSTRATE
  - ADR-0094-ONE-CANONICAL-FACT-LOG
  - ADR-0102-UNIFIED-AUTHORITY-ENGINE
stale_after: 2027-09-18
---

# ADR-0130 — Fenced Admission Tickets (R4-B atomicity boundary)

> New ADR written in repo (no package source).

## Context (R4-B)

The Authority Engine (`crates/sddk-engine/src/authority_engine.rs`) returns an
`AdmissionDecision` (`Allow` | `Deny` | `RequireApproval`). Callers hold the
decision and apply the side effect. The A5 risk register flagged R4 (P0,
G5) as **two distinct sub-risks**:

- **R4-A (closed by A5-3 F4)**: Deny ⇒ zero side effect. Pinned by
  `crates/sddk-engine/tests/authority_fail_closed.rs`. Engines
  short-circuit on `Deny` before any effect.
- **R4-B (open)**: **Decide-then-act TOCTOU**. Between the
  `admit_surface(...) → Allow` moment and the actual side effect (file
  write, CAS put, ledger event emit, gh CLI), an authority change could
  flip the decision. Today, no atomicity boundary protects this gap.

`docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` enumerates the
options (transactional / fenced tickets / band-only) and proposes fencing
as the smallest viable primitive. The A5 programme forbids new
abstractions, so this ADR lives outside A5; it is the founding decision
of the A6 workstream.

## Decision

Introduce a **Fenced Admission Ticket** substrate primitive that augments
the existing `AuthorityEngine::admit(...) → AdmissionDecision` contract
without modifying the trait:

```text
AuthorityAdmissionTicket
  = {
      decision:        AdmissionDecision,         // Allow | Deny | RequireApproval
      policy_digest:   DigestSha256,              // snapshot of policy at admit time
      fence_token:     u64,                       // monotonic lease from cycle ledger
      issued_at_seq:   u64,                       // ledger sequence at admit time
      ticket_id:       String,                    // sha256 of (decision, policy_digest,
                                                  //   fence_token, issued_at_seq)
    }

AuthorityAdmissionTicket::consume(at_seq: u64, current_policy_digest: DigestSha256)
  -> Result<Allow, AuthorityError>
  AuthorityError::PolicyChanged  { ticket_digest, current_digest }
  AuthorityError::FenceExpired   { ticket_fence, current_fence }
  AuthorityError::StaleTicket    { ticket_issued_at_seq, request_at_seq }
  AuthorityError::TicketAlreadyConsumed { ticket_id }
```

The ticket is **not** a new authority. It is a *revalidation envelope*
over the existing `AdmissionDecision` anchored to the canonical event log
(`workflow_run_events_v1`) and the registered policy versions. The engine
that issued the ticket **cannot** be flipped out from under a valid
ticket because:

1. `policy_digest` is `sha256(canonical_json(policy_snapshot))`; changing
   the policy at `register_policy` produces a different digest, which
   `consume` rejects explicitly.
2. `fence_token` is monotonic on each `register_policy` or cycle lease
   advance; the current fence is queried at consume time, mismatches
   refuse the effect.
3. `issued_at_seq` is anchored to the canonical ledger sequence, so a
   ticket is invalidated after any later canonical event that supersedes
   the snapshot it was bound to.
4. Each ticket can be consumed **exactly once**; a double-consume
   attempt returns `TicketAlreadyConsumed` from an in-memory `BTreeSet`
   maintained by the admission ticket bus.

### Why this is *one* primitive, not three

A unified envelope:
- covers re-validation (single `consume` call),
- covers fencing (the `fence_token` field, advanced by the engine),
- covers staleness (the `issued_at_seq` bound to the canonical log),
- covers one-shot semantics (ticket_id + consumed-set).

Keeping these on one envelope prevents the obvious anti-pattern of a
trio of separate "tokens" with mismatched identity spaces.

### Migration rule (strangler, not flag-day)

For each effect surface, the call site changes from:

```rust
let decision = engine.admit(proposal, actor, facts, policy);
if decision.is_allow() { apply_side_effect()?; }  // ← TOCTOU window
```

to:

```rust
let ticket = runtime.issue_admission_ticket(proposal, actor, facts, policy)?;
runtime.with_admission_ticket(ticket, |t| {                     // consumes at effect
    apply_effect(t)?;                                           // ← atomic w.r.t. policy
})?;
```

The original `engine.admit(...)` call site remains; the ticket wraps the
decision between admit-time and effect-time. **Nothing in `AuthorityEngine`
itself changes.** This is the strangler.

### Order of migration (cycle scope)

**This cycle (A6-0)** migrates only the surfaces named in the G5 audit as
**High-band and unguarded**:

| Surface | Reason High-band | Reason unguarded |
|---|---|---|
| `framework_bundle` write path | ships artifacts | no atomicity across admit/effect today |
| `github_releases` write path | ships artifacts | no atomicity; uses `gh` CLI |

Other surfaces (Medium-band and Low-band) migrate in **separate cycles**
named in `A6-0-RECEIPT.md §6 follow-up`.

## Non-goals

- **No new authority.** The ticket derives from `AuthorityEngine`; the
  engine remains the sole decision authority.
- **No `RequireApproval` re-design.** The ticket short-circuits on the
  `Deny` half of `Deny` and `Allow` half of `Allow`; approval flows
  remain as they were (out of R4-B).
- **No Medium/Low-band migration in this cycle.**
- **No async / long-running tickets.** Tickets are short-lived
  (T0 → T1 within the same effect transaction); long-lived async resumes
  belong to a different primitive.
- **No ledger-schema migration.** Tickets live in process memory; the
  ledger event is the *anchor*, not the *content* of the ticket.
- **No A4 semantic re-interpretation.** Does not touch
  `EvidencePosture`/`UniversalConcern`/alignment/verify/debverify.

## Consequences

Positive:
- R4-B is closed with a primitive small enough to land in one cycle.
- A5-C's G5 gate is then satisfiable: `BASE_PRODUCTION_READY` is
  achievable (subject to A5-4 / A5-5 closure).
- Migration is additive: each call site that opts in is one extra
  `with_admission_ticket` wrapper; no rewrite required.

Negative / honest risks:
- The in-memory `consumed_set` is per-process. Two processes issuing and
  consuming the same ticket_id would both succeed. A future cycle may
  move ticket_id to a `Ref` (ADR-0097 `RefStore::cas`) for cross-process
  atomicity; this cycle ships the single-process version and **the
  audit pin** in `AuthorityError::TicketAlreadyConsumed` makes the
  contract explicit.
- `policy_digest` hashing depends on canonical JSON of `PolicySnapshot`:
  a future change to `PolicySnapshot` shape (fields added) changes the
  digest format. Documented in `A6-0-RECEIPT.md §honesty markers`.

## Acceptance (G5 closure evidence)

The cycle's receipt (`docs/architecture/a6/A6-0-RECEIPT.md`) MUST show
all FENCE tests GREEN:

| Test | Expected |
|---|---|
| T1: ticket held T0→T1 same policy, fence, seq | `Allow` returned; effect applied |
| T2: ticket consumed after `register_policy` advanced | `Err(PolicyChanged { .. })`; effect refused |
| T3: ticket consumed with stale fence | `Err(FenceExpired { .. })`; effect refused |
| T4: ticket consumed twice | second `Err(TicketAlreadyConsumed)` |
| T5: `engine.admit` returning `Deny` ⇒ ticket constructor returns `None` | no ticket issued |

If any of T1–T5 fails, **STOP the cycle**, do not claim R4-B closed.

## Implementation (cross-reference for the context_fitness lint)

The primitive ships at:

- `crates/sddk-engine/src/authority_admission_ticket.rs` — `AuthorityAdmissionTicket`, `AdmissionTicketBus`, `AdmissionTicketError`, `AuthorityNow`.
- Re-exports under `crates/sddk-engine/src/lib.rs:31` (`pub mod authority_admission_ticket;`).
- Integration tests: `crates/sddk-engine/tests/a6_0_admission_tickets.rs` (FENCE matrix over the real `DefaultAuthorityEngine`).
- Unit tests inside the module (`mod tests`): T1..T5 + monotonicity + id-determinism.

The module name `authority_admission_ticket` is the canonical binding the
project's `no_new_root_level_context_module_without_adr` lint searches for;
this ADR is the registration record.
