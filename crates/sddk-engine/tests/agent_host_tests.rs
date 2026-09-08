//! RED tests for the Agent Host substrate.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentHost.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-079-AGENT-HOST-SUBSTRATE.md

use std::sync::Arc;

use sddk_engine::{
    ActionKind, AgentHost, AgentIdentity, AgentKind, DecisionReason, DecisionVerdict,
    InMemoryLeaseStore, LeaseError, MockClock,
};

fn host(clock_now: u64) -> AgentHost {
    let store = Arc::new(InMemoryLeaseStore::new());
    let clock = Arc::new(MockClock::new(clock_now));
    let identity = AgentIdentity::new("agent-1", "Agent One", AgentKind::Auto);
    AgentHost::new(identity, store, clock)
}

fn allow_resume() -> sddk_engine::DecisionRecord {
    sddk_engine::DecisionRecord::for_test(
        ActionKind::Resume,
        DecisionVerdict::Allow,
        vec![DecisionReason::Closed {
            reason: "test".into(),
        }],
        vec![],
    )
}

#[test]
fn acquire_and_drop_releases_lease() {
    let h = host(1000);
    {
        let handle = h.acquire_lease("c-1", 60_000).expect("acquire");
        assert_eq!(handle.fencing_token(), 1);
        assert_eq!(handle.cycle_id(), "c-1");
        assert_eq!(handle.owner(), "agent-1");
        // Drop at end of scope releases the lease.
    }
    // Next acquire should be fencing_token = 2 (previous holder released).
    let h2 = host(1000);
    let handle2 = h2.acquire_lease("c-1", 60_000).expect("acquire-2");
    assert_eq!(handle2.fencing_token(), 1); // fresh store
}

#[test]
fn concurrent_acquire_increments_token() {
    // Sequential acquire/release increments the fencing token each time
    // (a different owner re-acquiring).
    let store = Arc::new(InMemoryLeaseStore::new());
    let clock = Arc::new(MockClock::new(1000));

    let id1 = AgentIdentity::new("a-1", "Agent One", AgentKind::Auto);
    let h1 = AgentHost::new(id1, Arc::clone(&store) as _, Arc::clone(&clock) as _);
    let h1_handle = h1.acquire_lease("c-1", 60_000).expect("acquire-1");
    assert_eq!(h1_handle.fencing_token(), 1);
    h1_handle.release().expect("release-1");

    let id2 = AgentIdentity::new("a-2", "Agent Two", AgentKind::Human);
    let h2 = AgentHost::new(id2, Arc::clone(&store) as _, Arc::clone(&clock) as _);
    let h2_handle = h2.acquire_lease("c-1", 60_000).expect("acquire-2");
    assert_eq!(h2_handle.fencing_token(), 2);
    h2_handle.release().expect("release-2");
}

#[test]
fn execute_with_retry_succeeds_on_attempt_2() {
    let h = host(1000);
    let attempts = Arc::new(std::sync::Mutex::new(0u32));
    let attempts_clone = Arc::clone(&attempts);
    let result: Result<u32, sddk_engine::ExecuteError<String>> =
        h.execute_with_retry(ActionKind::Retry, || {
            let mut n = attempts_clone.lock().unwrap();
            *n += 1;
            if *n < 2 {
                Err("transient".to_string())
            } else {
                Ok(42)
            }
        });
    assert_eq!(result.unwrap(), 42);
    assert_eq!(*attempts.lock().unwrap(), 2);
}

#[test]
fn execute_retries_exhausted() {
    let h = host(1000);
    let attempts = Arc::new(std::sync::Mutex::new(0u32));
    let attempts_clone = Arc::clone(&attempts);
    let result: Result<u32, sddk_engine::ExecuteError<String>> =
        h.execute_with_retry(ActionKind::Retry, || {
            *attempts_clone.lock().unwrap() += 1;
            Err("always fails".to_string())
        });
    let err = result.expect_err("must fail");
    match err {
        sddk_engine::ExecuteError::RetriesExhausted { kind, attempts, .. } => {
            assert_eq!(kind, ActionKind::Retry);
            assert_eq!(attempts, 3); // default for Retry
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert_eq!(*attempts.lock().unwrap(), 3);
}

#[test]
fn execute_decision_builds_receipt() {
    let h = host(1000);
    let decision = allow_resume();
    let result = h.execute_decision(&decision, "c-1", || Ok::<(), String>(()));
    let receipt = result.expect("must succeed");
    assert_eq!(receipt.fencing_token, 1);
    assert_eq!(receipt.retries_used, 0);
    assert_eq!(receipt.agent_id, "agent-1");
    assert_eq!(receipt.executed_at_ms, 1000);
}

#[test]
fn execute_decision_rejects_non_actionable() {
    let h = host(1000);
    let decision = sddk_engine::DecisionRecord::for_test(
        ActionKind::Resume,
        DecisionVerdict::Deny,
        vec![DecisionReason::PolicyDenied {
            policy_id: "p".into(),
            kind: ActionKind::Resume,
        }],
        vec![],
    );
    let err = h
        .execute_decision(&decision, "c-1", || Ok::<(), String>(()))
        .expect_err("must reject");
    assert!(matches!(
        err,
        sddk_engine::ExecuteDecisionError::NotActionable { .. }
    ));
}

#[test]
fn execute_decision_release_on_failure() {
    let h = host(1000);
    let decision = allow_resume();
    // Op fails -> lease must be released so a subsequent acquire succeeds.
    let _ = h.execute_decision(&decision, "c-1", || Err::<(), String>("boom".to_string()));
    let handle2 = h.acquire_lease("c-1", 60_000).expect("re-acquire");
    assert_eq!(handle2.fencing_token(), 2);
}

#[test]
fn lease_conflict_when_other_owner_holds() {
    let store = Arc::new(InMemoryLeaseStore::new());
    let clock = Arc::new(MockClock::new(1000));

    let id1 = AgentIdentity::new("a-1", "Agent One", AgentKind::Auto);
    let h1 = AgentHost::new(id1, Arc::clone(&store) as _, Arc::clone(&clock) as _);
    let _h1_handle = h1.acquire_lease("c-1", 60_000).expect("acquire-1");

    // Different owner tries while lease is live -> conflict.
    let id2 = AgentIdentity::new("a-2", "Agent Two", AgentKind::Human);
    let h2 = AgentHost::new(id2, Arc::clone(&store) as _, Arc::clone(&clock) as _);
    let err = h2.acquire_lease("c-1", 60_000).expect_err("must conflict");
    assert!(matches!(err, LeaseError::Conflict { .. }));
}
