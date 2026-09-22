# C3a — RECEIPT

**Cycle:** C3a (Authority hardening, T19 + T20)
**Status:** `PASS_OBSERVED`
**Baseline at eval:** `main@ec86423` (workspace v1.169.142)
**Issued:** 2026-09-22T08:19:30Z
**Issuer:** orchestrator (direct implementation after subagent swarm limit hit; root cause `usage_limit_reached` documented)

## 1. Outcome

PASS_OBSERVED. Three new tests added to `crates/sddk-engine/src/authority_admission_ticket.rs::tests`:

- `t19_policy_swap_records_no_side_effects` — closes F3 gap (rejected consume must NOT poison `consumed` set).
- `t20_two_buses_with_divergent_policy_digests_dont_cross_accept` — two independent buses reject each other's tickets.
- `t20_concurrent_double_consume_only_one_succeeds` — concurrent race on `consume(same_ticket)` always yields exactly one Ok and one `TicketAlreadyConsumed`.

All 11 tests in the module pass green. 5/5 stress runs of the concurrency test confirmed zero flakiness.

## 2. Evidence

[UAT-EVIDENCE.yaml](UAT-EVIDENCE.yaml) contains the per-scenario contract, command, and observed output.

Verbatim summary:

```text
$ cargo test -p sddk-engine --lib authority_admission_ticket
running 11 tests
test authority_admission_ticket::tests::advance_fence_monotonic ... ok
test authority_admission_ticket::tests::t3_forged_higher_fence_refuses_with_fence_expired ... ok
test authority_admission_ticket::tests::t5_deny_decision_issues_no_ticket ... ok
test authority_admission_ticket::tests::t1_ticket_held_t0_to_t1_same_policy_fence_seq_allows ... ok
test authority_admission_ticket::tests::t3_stale_fence_token_refuses_with_fence_expired ... ok
test authority_admission_ticket::tests::t2_policy_changed_between_issue_and_consume_refuses_with_policy_changed ... ok
test authority_admission_ticket::tests::t4_ticket_consumed_twice_refuses_with_already_consumed ... ok
test authority_admission_ticket::tests::t19_policy_swap_records_no_side_effects ... ok
test authority_admission_ticket::tests::ticket_id_is_deterministic_per_inputs ... ok
test authority_admission_ticket::tests::t20_concurrent_double_consume_only_one_succeeds ... ok
test authority_admission_ticket::tests::t20_two_buses_with_divergent_policy_digests_dont_cross_accept ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1324 filtered out

$ cargo test -p sddk-engine --lib authority
test result: ok. 91 passed; 0 failed

$ cargo test -p sddk-engine --test a6_0_admission_tickets
test result: ok. 4 passed; 0 failed

$ cargo clippy -p sddk-engine --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 25s
(no warnings)
```

## 3. Production code: zero changes

Per SCOPE-CONTRACT §3 and STOP conditions: no production code modified. All three tests fit within the existing public API (`AuthorityNow`, `AdmissionTicketBus`, `AdmissionTicketError`, `DummyEngine`).

This is the cleanest possible outcome: **the existing code already satisfied T19 and T20 invariants**; the missing piece was empirical verification, which has now been supplied.

## 4. Implementation notes

- The T19 side-effects test relies on `AdmissionTicketBus::consume` performing all four guard checks (`TicketAlreadyConsumed`, `PolicyChanged`, `FenceExpired`, `StaleTicket`) **before** any state mutation (`consumed.insert` is the only mutation, at line 317). Verified by inspection of lines 287–319 prior to writing the test; the test then empirically confirms it.
- The T20 cross-policy test uses `Arc<Barrier>` to guarantee both threads complete their `issue` calls before either attempts cross-consume. This avoids a race between thread A's `issue` and thread B's `consume` that would otherwise intermittently observe `policy_b.policy_digest == None` mid-construction.
- The T20 atomicity test uses `Arc<Barrier>` with count=2 to align the two `consume` calls on the same `bus.clone()`. Because both clones share the underlying `Arc<Mutex<FenceState>>`, the race is genuine and the test is meaningful (not a no-op due to isolation).
- No helper additions were needed; all existing helpers (`policy_a`, `policy_b`, `DummyEngine`, `dummy_proposal`, `dummy_actor`, `dummy_facts`) covered the scenarios.

## 5. Risks

- The atomicity test depends on thread scheduling being able to actually overlap the two `consume` calls. On systems with very fast scheduling the second thread may complete before the first enters the mutex, which is fine because the assertion (`1 Ok + 1 AlreadyConsumed`) holds regardless of arrival order — both orderings produce exactly one Ok.
- `PolicySnapshot::Clone` is required for the cross-policy test's outer `policy_a().policy_digest` reads; this is already derived in `authority_engine.rs:307`.

## 6. Pointer reconciliation

| Field | Before (session-11 open) | After (this receipt) |
|---|---|---|
| `HEAD` | `ec86423` | `<post-c3a>` |
| Workspace version | `1.169.142` | `1.169.142` (no bump per scope) |
| `verified_components_at_current_sha.cargo_test_workspace` | "4998 passed; 0 failed; 15 ignored" (session-10) | updated by next CI/profile run |

## 7. Next WorkItem

Per AUTO initiative continuity and the C3 DAG, the next WorkItem is **C3b Storage adversarial** (T21, T22):

- **T21**: Crash/reopen alrededor de event append, CAS y receipt.
- **T22**: Contención SQLite en superficies IMMEDIATE activas.

Pre-flight will check `crates/sddk-storage/src/{backlog_store, event_store, cas}.rs` for current coverage and identify the highest-value test gaps.

C3c (security canarios), C3d (performance baseline), and C3e (schema resilience) follow.
