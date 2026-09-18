# A6-0 — R4-B Fenced Admission Tickets (close R4-B + unblock A5-C)

> Cycle: `p-63676b11dc0ef88f/a6-0-r4b-admission-tickets`
> Status: **planning only — nothing here is implemented yet**
> Standing on: A4-CLOSEOUT v1.169.68 (`A4_CERTIFIED`), A5-1 v1.169.71 (release),
> A5-2 v1.169.74 (durability), A5-3 v1.169.75 (concurrency; R4-B OPEN).
> Out of A5 (new substrate primitive; A5 programme forbids new abstraction).

## Why this cycle, why now

A5-3 closed R3, R5, R6, R12. **R4 was partially closed**: R4-A (denied ⇒ no
effect) pinned by `authority_fail_closed.rs`; **R4-B (decide-then-act TOCTOU)
is open and named in `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md`**.

The A5 production-readiness contract §G5 is a **blocker rule**:

> denied ⇒ zero side effect; **the decision→effect atomicity boundary is
> named and tested (or declared a blocker)**.

R4-B is currently a soft boundary: `AuthorityEngine::admit() → AdmissionDecision`,
no re-validation at effect time. A5-C cannot declare `BASE_PRODUCTION_READY`
while R4-B is open (G5 fail-closed). Closing R4-B unblocks A5-C.

The 3 options named in the incident:

| Option | Why skipped | Why picked |
|---|---|---|
| **A** transactional effects | Multi-cycle refactor of every write path; explodes change budget | — |
| **B** fenced admission tickets | New substrate primitive (would have violated A5) | **THIS CYCLE** |
| **C** band-only migration | Leaves Medium/Low unguarded; no proof of safety on the unguarded | — |

This cycle is outside A5 by design (A5 forbids new abstraction). Naming it
`a6-0` because the next milestone after A5-C is A6 / POST_BASE.

## Scope budget (one cycle)

1. **ADR** — `ADR-0130-FENCED-ADMISSION-TICKETS` (follow-up to ADR-0097).
   Choose Option B; bound the change to a *thin* substrate primitive that
   augments the existing `AuthorityEngine` without breaking its contract.
2. **New substrate primitive** — `AuthorityAdmissionTicket`:
   - carries `(decision, policy_digest, fence_token, issued_at_ledger_seq)`;
   - exposes `consume(at_ledger_seq: Seq) -> Result<Allow, AuthorityError>`;
   - revalidates against canonical policy at consume time; rejects on
     `PolicyChanged` (typed error) with the actual current policy digest.
3. **Ledger-anchored effect gate** — every side effect in the engine goes
   through `EngineRuntime::with_admission_ticket(|ticket| { ... })`; ticket
   must be alive at the moment of the effect, not at the moment of the call.
4. **Migrate the two High-band unguarded surfaces** first:
   - `sddk dev install` → `framework_bundle` write path
   - `sddk dev release` → `github_releases` write path (the `gh release create`
     and `gh release upload` invocations)
5. **Tests** — FENCE falsification matrix (RED → GREEN):
   - T1: Allow ticket held from T0 to T1 still resolves Allow at T1.
   - T2: Policy changes between T0 and T1 → effect at T1 refused (PolicyChanged).
   - T3: Fence token mismatch → denied (FenceMismatch).
   - T4: Ticket reused after consume → denied (TicketAlreadyConsumed).
   - T5: Engine short-circuit: `Deny` decision never yields a ticket.
6. **Receipts** — `docs/architecture/a6/A6-0-RECEIPT.md` modelling on
   `A5-3-RECEIPT.md`, with falsification matrix gates and "still-IGNORED/not
   migrated" list. Close `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY`.

## Out of scope (named explicitly)

- **No R9 migration** (CAS put side-effect races); owned by A5-5 / A6 future.
- **No Medium/Low band surfaces migration** (`plan_item`, `evidence_attachment`,
  `decision_record`, `dependency_edge`); named in receipt as future work.
- **No knowledge_graph_vault fencing**; vault is human-source, not authority.
- **No governance / approval flow re-design** (RequireApproval path is
  orthogonal; it already short-circuits via R4-A and `RequireApproval` is
  not a side-effect itself).

## Hard constraints

| Constraint | Why | Honoured by |
|---|---|---|
| **No A4 semantic re-interpretation** | A4 §2 freeze | spec uses `AdmissionDecision` verbatim; does not add fields to `EvidencePosture`/`UniversalConcern`/etc. |
| **Strangler over flag-day** | AGENTS §2.10 | `AuthorityEngine::admit` is unchanged; new primitive wraps. |
| **No new authority** | A4 §12 invariant #1 | ticket is *derived from* the existing engine, not a new arbiter. |
| **Falsify before fix** | A5 contract §7/§8 | every test written against pre-fix code; RED captured. |
| **Stop on semantic defect** | A5 contract §2 | if discovered, STOP and recertify. |
| **Receipt on close** | A5 contract §6 | `A6-0-RECEIPT.md` mandatory. |

## Risk gate (initial)

- `cargo test --workspace`: green at A5-3 close (`2f37fb6`),
  minus the 12 still-ignored tests.
- `AuthorityEngine::admit` is the existing authority; no caller is being
  broken (verified by reasoning — adding ticket wrapping is additive).
- FENCE tests will be written **before** the ticket primitive so RED is
  pinned to current `main`.

## Follow-ups deferred (named, not silent)

After A6-0 closes:

1. **A5-C** becomes executable (no G5 blocker on R4-B).
2. **Medium-band migration** of `plan_item`, `evidence_attachment`,
   `decision_record` — separate cycle, scope separate from this one.
3. **Vault authority boundary** — separate; vault is knowledge, not authority.
4. **ADR-0097 documentation residue** — update ADR-0097 to reference
   `AdmissionTicket` as the boundary-aware extension (named in receipt).

## Roadmap Delta (at A6-0 close)

```text
A4-CLOSEOUT   CLOSED v1.169.68
A5-1          CLOSED v1.169.71
A5-2          CLOSED v1.169.74
A5-3          CLOSED v1.169.75  (R4-A closed; R4-B open)
A6-0          NEXT (this cycle)  R4-B fenced admission tickets  ← YOU ARE HERE
A5-4..A5-5    unblocked at A5-3 close; R4-B was the missing gate
A5-C          blocked_by A5-1..A5-5 + A6-0 (R4-B must close first)
```

## See also

- `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` (the incident)
- `docs/architecture/adrs/ADR-0097-COMMON-REVISION-SUBSTRATE.md`
- `docs/architecture/a5/A5-3-PLAN.md` R4 read
- `docs/architecture/a5/A5-PRODUCTION-READINESS-CONTRACT.md` §G5
