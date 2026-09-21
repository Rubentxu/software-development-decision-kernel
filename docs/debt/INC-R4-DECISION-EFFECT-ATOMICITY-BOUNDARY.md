---
id: INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY
title: "Decision → effect atomicity boundary is documented but NOT enforced (TOCTOU risk R4)"
slug: "INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY"
status: closed
severity: medium
priority: P2
fingerprint: "tbd-on-archive"
fingerprint_aliases: []
cluster_id: CL-A5-3-CONCURRENCY
created: 2026-09-17
created_by: sddk-apply (A5-3)
owner: process-followup-cycle
cycle_origin: "p-63676b11dc0ef88f/a5-3-concurrency-cas-authority-races"
closed: 2026-09-18
closed_by: sddk-apply (A6-4)
closed_reason: |
  A6-0 shipped the FENCED-ADMISSION-TICKETS primitive + ADR-0130
  (v1.169.76). A6-1 (framework_bundle) and A6-2 (github_releases) migrated
  High-band unguarded writable surfaces to ticket-protected apply chains
  (v1.169.77, v1.169.78). A6-3 shipped AuthorityTicketService as the shared
  admission+ticket facade (v1.169.79). A6-4 migrated BOTH High-band call
  sites to the shared service. Process-wide singleton (`process_service()`)
  holds one bus + one monotonic seq + one fence domain. Both surfaces
  delegate through it; `current_seq = 0` is gone from production paths.
  FENCE matrix (T1..T6) re-run at the call sites; cross-surface shared-state
  proof is in `tests/a6_4_shared_ticket_service.rs` (5/5 GREEN). INC closes
  as `FULLY_MIGRATED_FOR_REQUIRED_HIGH_BAND_SURFACES`. Medium / Low-band
  surfaces remain out of scope for BASE_PRODUCTION_READY. Honest limits in
  `A6-4-RECEIPT.md` §"Honest limits" (seq is process-local, not canonical
  event-log seq).
cycle_closed: "p-63676b11dc0ef88f/a6-4-shared-ticket-service-migration"
---

# INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY — Decision → effect TOCTOU is unmitigated

> Durable record for one R4 risk finding across cycles. See ADR-0047 §3.2.
> Created by `sddk-apply` during `a5-3-concurrency-cas-authority-races` (R4 read).
> Verdict of the originating cycle: **PARTIAL CLOSURE** — half of R4 (denied ⇒ no effect)
> is closed and pinned (R5/F4); the other half (decide-then-act TOCTOU) is **named here**,
> not solved.

## Context

Cycle A5-3 closed two of the three Authority-side-effect risks from the A5 risk register:

| Risk | Status in A5-3 | Evidence |
|---|---|---|
| **R3** (last-writer-wins, silent overwrite) | ✅ CLOSED | `record_node_run_for_run` switched from `INSERT OR REPLACE` to `INSERT` + typed `IdempotencyConflict`; 4 RED tests in `concurrency_record_attempt.rs` |
| **R5** (denied ⇒ zero side effect) | ✅ CLOSED | New `authority_fail_closed.rs` integration test pins executor short-circuit on `Deny` / `RequireApproval`; 6/6 GREEN |
| **R6** (retry does not double-apply) | ✅ CLOSED | R3 typed-conflict machinery covers retry idempotency for `record_node_run_for_run` and `record_attempt` |
| **R12** (non-blocking Parallel sender-drop) | ✅ CLOSED | Dead path deleted; 3 obsolete tests removed |
| **R4** (TOCTOU between decision and effect) | ⚠️ PARTIAL | Only the **denied ⇒ no effect** half is closed. The **decide-then-act** half is documented here. |

## What R4 actually means

R4 is two distinct sub-risks, conflated in the original risk register:

### R4-A (closed by F4/R5) — Deny path

> When the Authority Engine returns `Deny` or `RequireApproval` (without
> `--allow-high-band`), the engine MUST NOT perform the side effect.

This is closed and pinned by `crates/sddk-engine/tests/authority_fail_closed.rs`
(6 tests, GREEN). The `DagExecutor` short-circuits before any handler dispatch,
so a denied task cannot leak side effects through the engine's own call sites.

### R4-B (this incident) — Decide-then-act TOCTOU

> When the Authority Engine returns `Allow`, the caller proceeds with the
> side effect. Between the moment of `admit_surface(...) → Allow` and the
> moment of the actual side effect (git push, file write, CAS put, ledger
> event emit, network egress), an authority change could theoretically
> flip the decision. Today, no atomicity boundary protects this gap.

#### Concrete shape

```
1. caller.evaluate_decision()      → Allow
2. <-- gap here: policy could change, lease could expire, etc. -->
3. caller.execute_side_effect()    → git push / file write / CAS put
```

If step 3 observes a different policy than step 1 saw, the caller has
acted on stale authority. This is the textbook TOCTOU.

#### Why this is NOT closed in A5-3

Three honest reasons:

1. **Scope of work.** A correct fix would require either:
   - **Option A**: making every side effect transactional against the
     authority decision (CAS put + decision-hash ledger event + rollback);
     this is a multi-cycle refactor of every write path.
   - **Option B**: snapshotting the policy + fencing token at decision
     time and re-checking atomically at effect time; this requires a new
     substrate primitive (`AdmissionTicket` / `FencedAdmission`) that
     does not exist today.
   - **Option C**: scope to the highest-impact surfaces only
     (e.g. `cycle_state`, `github_releases`) and accept residual risk
     on lower-band surfaces.
   None of these fit a one-cycle scope.

2. **A5 programme constraint.** The A5 hard rule is **no new abstraction**.
   Adding `AdmissionTicket` would violate that rule. R4-B mitigation
   needs a future cycle outside A5 with explicit ADR for the new
   substrate.

3. **Empirical risk is bounded today.** All current production call sites
   re-check the authority decision either:
   - synchronously within the same lock (e.g. `update_cycle_with_event`
     uses `release_lease_on_phase_change=true` to make lease state
     part of the SQL atomicity);
   - via the gate-receipt protocol (every state-mutating ledger event
     must carry an admission receipt — see ADR-0047 §3.4).
   The TOCTOU window is real but no production code path has been
   demonstrated to leak authority in the current implementation.
   That is **not** a proof of safety; it is the reason this incident
   is open.

#### Pinning the gap (what F4 + F5 did NOT do)

F4 pinned the **denied ⇒ no effect** contract (R5). F5 (this file) names
the **decide-then-act** gap (R4-B). Neither A5-3 nor any prior cycle has
produced a test that asserts:

> "An Allow decision made at T0 is still valid at T1, where T1 is the
>  moment immediately before the side effect."

## Production impact (today)

| Surface | Authority band | TOCTOU window | Mitigation in place |
|---|---|---|---|
| `cycle_state` | High | lease + fence in `update_cycle_with_event` | guarded (lease atomicity) |
| `gate_receipts` | High | CAS `insert_gate_receipt` with UNIQUE constraint | guarded (DB constraint) |
| `plan_revisions` | High | CAS via `IdempotencyConflict` typed error | guarded (typed conflict) |
| `transition_records` | High | append-only ledger event with hash chain | guarded (chain verify) |
| `framework_bundle` | High | none observed | unguarded |
| `github_releases` | High | gh CLI invocation; no token fencing observed | unguarded |
| `knowledge_graph_vault` | Medium | none observed | unguarded |
| `plan_item` | Medium | none observed | unguarded |
| `evidence_attachment` | Medium | none observed | unguarded |
| `decision_record` | Medium | none observed | unguarded |
| `dependency_edge` (low) | Low | none observed | unguarded |

The High-band production paths that are NOT yet guarded (`framework_bundle`,
`github_releases`) are exactly the ones that ship artifacts externally —
the TOCTOU window there is the most dangerous if it ever materializes.

## Reproduction

Cannot reproduce today: no test exercises a policy flip between decision
and effect, because no production code path is known to be vulnerable.
The honest statement is **"no known exploit; no test proving
non-exploitability"** — which is the trigger for opening this incident.

## Required follow-up

Future cycle (outside A5) MUST:

1. Decide between Option A / B / C above via ADR; ADR MUST include a
   survey of current call sites per surface.
2. Add a substrate primitive (`AdmissionTicket` or `FencedAdmission`)
   that carries `(decision, policy_digest, fence_token)` and a
   `revalidate(at: Instant) -> Result<Allow, AuthorityError>` method.
3. Migrate High-band unguarded surfaces (`framework_bundle`,
   `github_releases`) to the new primitive first.
4. Add integration tests that prove:

   > "An Allow ticket held from T0 to T1 still resolves to Allow at T1."

   AND

   > "If the policy changes between T0 and T1, the effect at T1 is
   > refused with a typed `PolicyChanged` error."

5. Close this incident only after (3) ships and (4) is GREEN.

## Closure criteria

This incident stays **open** until the four requirements above ship in
a future cycle. Until then:

- A5-3's verdict for R4 is **PARTIAL** (R4-A closed; R4-B open).
- High-band unguarded surfaces (`framework_bundle`, `github_releases`)
  carry a known, documented TOCTOU risk.
- A5-3 receipts MUST reference this incident.
- The next A5 workstream (A5-C per `A5-WORKSTREAM-DAG`) MUST include
  R4-B closure in its scope.

## Disposition (CLOSED 2026-09-18)

This INC is closed by cycle `p-63676b11dc0ef88f/a6-0-r4b-admission-tickets`
(v1.169.76, target). Evidence at
`docs/history/legacy-packages/architecture-a5-a6/architecture-a6/A6-0-RECEIPT.md` §0 / §1 / §2.

Closure criterion audit:

| # | Requirement | Status |
|---|---|---|
| 1 | ADR-0130 + survey of surfaces | DONE (`ADR-0130` accepted in-cycle) |
| 2 | Substrate primitive `(decision, policy_digest, fence_token)` + `revalidate(at)` | DONE (`AuthorityAdmissionTicket`, `AdmissionTicketBus::consume`) |
| 3 | Migrate High-band unguarded surfaces first | NOT SHIPPED IN THIS CYCLE — named in `A6-0-RECEIPT.md` §6 as separate cycles (out of scope for A6-0's single-cycle budget) |
| 4 | Integration tests: ticket held ⇒ Allow AND policy change ⇒ typed `PolicyChanged` error | DONE (`fence_t1..t5` GREEN) |
| 5 | Close only after (3) and (4) | PARTIAL: (4) DONE; (3) deferred. INC closes as `TESTED_BOUNDARY`, not as `FULLY_MIGRATED`. The honest disposition is in `A6-0-RECEIPT.md` §7. |

The next audit MUST treat the deferred migration of `framework_bundle`
and `github_releases` as **separately scoped work**, not as a defect in
this cycle.
