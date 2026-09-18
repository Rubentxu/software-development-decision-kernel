---
id: ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES
status: accepted
supersedes_history: false
adopted_at: 2026-09-18
adoption_cycle: p-63676b11dc0ef88f/a6-1-framework-bundle-migration
package_local_id: null
package_source: null
accepted_at: 2026-09-18
accepted_by_cycle: p-63676b11dc0ef88f/a6-1-framework-bundle-migration
superseded_by: []
related_adrs:
  - ADR-0130-FENCED-ADMISSION-TICKETS
  - ADR-0097-COMMON-REVISION-SUBSTRATE
  - ADR-0102-UNIFIED-AUTHORITY-ENGINE
stale_after: 2027-09-18
---

# ADR-0131 — Migration Pattern for Writable Surfaces (R4-B, A6-1 onward)

> New ADR written in repo (no package source).

## Context

`ADR-0130-FENCED-ADMISSION-TICKETS` (A6-0) introduced `AdmissionTicketBus`
as the substrate primitive that closes R4-B at the *type-system* level
(T1..T5 proven in `crates/sddk-engine/tests/a6_0_admission_tickets.rs`).
The incident (`INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY`) requires
migration of High-band unguarded surfaces to actually use the primitive
before the INC closes as `FULLY_MIGRATED`.

The two named surfaces are `framework_bundle` (A6-1) and `github_releases`
(A6-2). This ADR documents the **migration pattern** so A6-2 (and any
future surface migration) consumes the same recipe verbatim.

## Decision

A writable surface opts into the ticket wrapper by:

1. **Existing call shape preserved**: `AuthorityContext::for_cli(...)`
   + `validate(WritableSurface)` (or `admit_governed(...)` if the surface
   goes through the runner) continues to run **at admit time**. This
   satisfies R4-A (denied ⇒ zero side effect).

2. **Ticket issued immediately after admit, before any side effect**:
   the call site produces a `PolicySnapshot` derived from the
   `RunnerVerdict.explanation` (or a deterministic re-construction) and
   `AdmissionTicketBus::issue(engine, proposal, actor, facts, policy, seq)`
   returns a ticket. `seq` is read from the cycle ledger (or `0` if no
   cycle context applies — see "Honest limits" below).

3. **Side effect runs inside `with_ticket(...)`**: the call site wraps
   every `std::fs`/`std::process`/git-push/CAS-put in a closure that
   takes the ticket. The wrapper calls `consume(&ticket, now)` at the
   closure entry and bails out with the typed `AdmissionTicketError`
   before any write. Honesty: the ticket is **consumed** before the
   side effect, not after — `consume` is the atomicity point.

4. **On `Ok(())`**: the ticket is one-shot; no reuse.

## Pattern (idiomatic Rust sketch, FOR ILLUSTRATION)

```rust
use sddk_engine::authority_admission_ticket::{AdmissionTicketBus, AdmissionTicketError, AuthorityNow};

pub(crate) fn with_framework_bundle_ticket<F, T>(
    actor: &str,
    actor_kind: ActorKind,
    proposal: ActionProposal,
    policy: PolicySnapshot,
    body: F,
) -> Result<T, WithTicketError>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    let runner = AuthorityEngineRunner::new();
    let mut facts = Facts::default();
    facts.decision_refs.push(DecisionRef(proposal.target_id.clone()));

    let verdict = runner
        .admit_surface(actor, "framework_bundle", proposal.kind, &proposal.target_id, facts)
        .map_err(|e| WithTicketError::Authority(e.to_string()))?;

    if !verdict.decision.is_allow() {
        return Err(WithTicketError::Denied(format!("{:?}", verdict.decision)));
    }

    let bus = AdmissionTicketBus::new();
    let ticket = bus
        .issue(&runner_engine_for_admission(), &proposal, &actor_from_verdict(&verdict), &facts, &policy, /* seq= */ 0)
        .map_err(|e| WithTicketError::Ticket(e))?;

    let now = AuthorityNow {
        current_policy_digest: policy.policy_digest.clone(),
        current_fence: bus.current_fence(),
        current_seq: 0,  // see "Honest limits" below
    };

    // consume before the side effect
    bus.consume(&ticket, &now).map_err(|e| WithTicketError::Ticket(e))?;

    // body runs only after the ticket is consumed
    body()
}
```

The actual `sddk dev install` wrapper lives at
`crates/sddk-cli/src/dev/install.rs::with_framework_bundle_ticket` and
is the canonical example for this pattern.

## Honest limits in A6-1

This cycle's `framework_bundle` wrapper does NOT currently exercise
**T2 (policy-digest mismatch via `Err(PolicyChanged)`)** because:

- The `AuthorityEngineRunner` returns a `RunnerVerdict` with a
  `decision_digest` (in `AdmissionExplanation`), but does **not**
  expose the underlying `PolicySnapshot::policy_digest`.
- Reconstructing the policy from the verdict is non-trivial (the runner
  constructs policies inside `bridge::default_policy_for_surface`),
  and threading the live snapshot through the runner is out of scope
  for A6-1 (one-cycle change budget).
- Without that, the ticket issued at the wrapper site anchors to a
  `policy_digest` that the engine cannot revalidate against the live
  registered policy without an API change.

**What A6-1 DOES pin for `framework_bundle`**:

- T1 (ticket held T0→T1) — pinned via `fence_t1_*` integration.
- T4 (consumed twice) — pinned via `fence_t4_*` integration.
- T5 (Deny ⇒ no ticket) — pinned via `fence_t5_*` integration.
- T6 (NEW — the consume anchor is the verdict, not an arbitrary policy):
  pinned via `fence_t6_*` — the ticket carries `policy_digest = ...`
  derived from the verdict; an attempt to swap the live engine for a
  different policy (e.g. through a forged AuthorityContext) yields a
  typed refusal at consume time.

**T2 (PolicyChanged at effect-time)** is therefore re-pinned **only**
inside `crates/sddk-engine/tests/a6_0_admission_tickets.rs` (A6-0
FENCE integration). The receipt `A6-1-RECEIPT.md` §honesty lists T2
as **not directly pinned for this call site**, and the upgrade path is
named as a future cycle:

> **A6-3 (future)**: extend `AuthorityEngineRunner` to expose the
> registered `PolicySnapshot::policy_digest` alongside `RunnerVerdict`,
> so call-site wrappers can construct tickets anchored to the live
> policy. Until A6-3, T2 coverage for `framework_bundle` is the engine
> primitive's coverage (proven at A6-0 close).

## Non-goals

- **No modification of `AuthorityEngine::admit` or `DefaultAuthorityEngine`.**
- **No modification of `AuthorityContext`.**
- **No change to `WritableSurface` matrix or the actor-kind policy.**
- **No thread-safety guarantee beyond what `AdmissionTicketBus`
  already provides (single process, single bus instance per call).**

## Consequences

Positive:
- `framework_bundle` opts into the ticket wrapper; A6-2 can copy the
  recipe verbatim with a different surface name.
- The A6-1 FENCE integration proves T1, T4, T5, T6 for this specific
  surface. The primitive-level T2 stays pinned at the engine.
- No regression risk: the existing `AuthorityContext::validate(...)`
  short-circuit path is preserved; the ticket wrapper runs **on top**.

Negative:
- T2 isn't directly pinned for `framework_bundle`. Honesty:
  bookkeeping for the A6-3 upgrade.

## See also

- `docs/architecture/adrs/ADR-0130-FENCED-ADMISSION-TICKETS.md`
- `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md`
- `docs/architecture/a6/A6-0-RECEIPT.md` (the primitive)
- `docs/architecture/a6/A6-1-RECEIPT.md` (this cycle's evidence)
- `docs/architecture/a6/A6-1-PLAN.md`
