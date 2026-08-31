# SPIKE-001: Human Approval as First-Class Events

**Date:** 2026-08-18
**Status:** Research Complete
**Author:** SDDK Architecture Spike
**Related:** MUST #2 (Governed Capability Flow), MUST #3 (Agent/Behavior Hashes)

---

## 1. Current State Summary

### What Already Exists

The codebase already has a **synchronous, boolean approval mechanism**:

| File | Existing Concept |
|------|-----------------|
| `sddk-domain/src/workflow.rs:296` | `CapabilityDef::requires_approval()` — returns `true` if `risk ∈ {high, critical}` OR `consequence ∈ {irreversible, modifies}` |
| `sddk-domain/src/proposal.rs:186` | `ProposalPolicyDecision::ApprovalRequired` — policy decision variant |
| `sddk-gateway/src/gateway.rs:71` | `GatewayError::ApprovalRequired` — runtime error when approval missing |
| `sddk-gateway/src/gateway.rs:39` | `approve: bool` field in `CapabilityPlanInput` — passed through to policy |
| `sddk-gateway/src/policy.rs:48` | `PolicyDecision::requires_approval` — boolean decision field |
| `sddk-domain/src/cycle.rs:32` | `CycleStatus::UatWaiting` — existing precedent for cycle waiting on human |

### What Is Missing

1. **No approval events in the ledger** — `event_bus.rs` only emits `workflow.phase.*` and `workflow.transition.*`. No `approval.requested`, `approval.granted`, or `approval.denied` events exist.

2. **No `ApprovalPending` cycle status** — unlike `UatWaiting` which pauses a cycle waiting for UAT verdict, there is no equivalent for approval.

3. **No approval receipt type** — `CapabilityReceipt` tracks capability execution but there is no separate approval record.

4. **Synchronous only** — the current `approve: bool` flag is passed inline at request time. There is no async wait, no callback, no polling mechanism.

5. **No persistence of approval decisions before capability execution** — the approval exists only as an in-memory boolean at request time.

---

## 2. Design Options

### Option A: ApprovalPending Cycle Status (Recommended)

Introduce `CycleStatus::ApprovalPending` — the cycle transitions to this status when a high-risk capability requires approval. The workflow engine pauses on this status until `approval.granted` or `approval.denied` events are emitted to the ledger.

**Event sequence:**
1. `approval.requested` — emitted when `ProposalPolicyDecision::ApprovalRequired` is reached, before capability execution
2. `approval.granted` or `approval.denied` — emitted by human action (CLI, API, webhook)

**Key characteristics:**
- Cycle-scoped (like `UatWaiting`)
- Async: engine waits for events, does not block
- Approval record is persisted in the event ledger before capability executes
- Idempotency via `event_id` deterministic generation

**Tradeoffs:**
| Pros | Cons |
|------|------|
| Consistent with `UatWaiting` pattern | Ties approval to cycle lifecycle |
| First-class events in ledger | Requires cycle state machine changes |
| Async by design | Human must be aware of cycle context |
| Full audit trail with actor/timestamp | |

### Option B: Parallel Approval Capability

Model approval as a separate capability (`approval.request`, `approval.evaluate`) that runs parallel to the governed capability. The governed capability's execution waits until the approval capability produces a receipt.

**Tradeoffs:**
| Pros | Cons |
|------|------|
| Works outside cycle context | Adds new capability type overhead |
| Approval is just another receipt | Coordination complexity between parallel tracks |
| Flexible human-in-the-loop patterns | |

### Option C: Proposal-level Flag + Approval Artifact

Add `requires_human_approval: bool` to `Proposal`. Policy evaluates it. Approval becomes a required artifact (like `exploration-report`) before the capability transition can proceed.

**Tradeoffs:**
| Pros | Cons |
|------|------|
| Minimal changes to existing types | Still synchronous in nature |
| Leverages existing artifact/requirement model | Approval not first-class event |
| Simple to understand | No async waiting, no audit trail beyond receipt |

---

## 3. Recommended Approach

**Option A** is recommended because:

1. It aligns with the existing `UatWaiting` pattern in `CycleStatus` (line 32 of `cycle.rs`) — same conceptual model, same implementation approach.
2. It produces first-class events (`approval.requested`, `approval.granted`, `approval.denied`) in the event ledger, enabling full traceability.
3. It is async by design — the workflow engine waits for events rather than blocking.
4. The approval record is **persisted before capability execution**, satisfying the receipt-bound design principle.

**Rejected alternatives:**
- Option B adds unnecessary complexity with a parallel capability model.
- Option C does not produce first-class events and is still synchronous.

---

## 4. Estimated Effort

**Medium**

| Component | Effort | Notes |
|-----------|--------|-------|
| `sddk-domain/src/cycle.rs` | Low | Add `ApprovalPending` variant to `CycleStatus` |
| `sddk-domain/src/models.rs` | Low | Add `ApprovalReceipt` record type |
| `sddk-domain/src/event_envelope.rs` | Low | Approval event types already follow existing pattern |
| `sddk-engine/src/event_bus.rs` | Medium | Add `emit_approval_event()` function |
| `sddk-gateway/src/gateway.rs` | Medium | Modify `execute_governed` to emit `approval.requested` and wait |
| Tests | Medium | Add tests for approval event emission, async wait behavior |
| **Total** | **Medium** | ~3-5 days |

---

## 5. Key Files That Would Change

| File | Change |
|------|--------|
| `crates/sddk-domain/src/cycle.rs` | Add `ApprovalPending` to `CycleStatus` enum |
| `crates/sddk-domain/src/models.rs` | Add `ApprovalReceipt`, `ApprovalReceiptInput` |
| `crates/sddk-domain/src/event_envelope.rs` | Validate approval event types (`approval.*`) |
| `crates/sddk-engine/src/event_bus.rs` | Add `emit_approval_requested()`, `emit_approval_decision()` |
| `crates/sddk-gateway/src/gateway.rs` | In `execute_governed`: emit `approval.requested`, await `approval.granted/denied`, handle timeout |

---

## 6. Top 3 Risks

### Risk 1: Async Wait Complexity
The workflow engine has no existing async wait mechanism for arbitrary events. Adding one (or adapting the cycle engine to wait on `approval.*` events) introduces non-trivial state machine complexity.

**Mitigation:** Start with a polling approach in the gateway layer rather than modifying the core cycle engine. The gateway polls for `approval.granted` events on the ledger before proceeding with capability execution.

### Risk 2: Human Notification Channel
Emitting `approval.requested` events does not notify a human. The system needs an out-of-band notification mechanism (Slack, email, webhook) that is outside the current SDDK scope.

**Mitigation:** Design the approval event payload to include sufficient context for external notification systems to consume. The notification channel itself is out-of-scope for MUST #4.

### Risk 3: Timeout and Expiry
If a human never approves or denies, the capability (and possibly the cycle) hangs indefinitely. The existing `Proposal::expires_at` field is not consulted during the approval wait.

**Mitigation:** Add an `approval_expires_at` timestamp. If the approval decision is not recorded before expiry, the gateway returns `GatewayError::ApprovalExpired`. The cycle engine treats this as a transition failure.

---

## 7. Open Questions (For Design Phase)

1. What is the timeout for pending approvals? (proposal expiry? configurable per capability?)
2. Does the same human who requested the approval also decide it, or is there a separation of duties?
3. Should approval decisions be editable (can a granted approval be revoked before capability executes)?
4. How does this interact with idempotency — can the same proposal request approval multiple times?
