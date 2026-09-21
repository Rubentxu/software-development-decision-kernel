//! AIW-S8 X04 — Two-CLI concurrency integration tests.
//!
//! Simulates two CLI processes (each with its own `AgentHost` but sharing
//! one lease store) racing for the same cycle lease. Closes X04:
//! "dos CLIs contra mismo storage con update. No doble autoridad ni
//! state divergence."
//!
//! Spec: tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s8-x04-two-cli-concurrency/SCOPE-CONTRACT.md

use std::sync::Arc;

use sddk_engine::{AgentHost, AgentIdentity, AgentKind, InMemoryLeaseStore, LeaseError, MockClock};

/// A "CLI process": its own host identity, but sharing the storage.
fn host(name: &str, store: Arc<InMemoryLeaseStore>) -> AgentHost {
    let identity = AgentIdentity::new(name, name, AgentKind::External);
    AgentHost::new(identity, store, Arc::new(MockClock::new(1000)))
}

#[test]
fn two_hosts_share_one_lease_store_first_wins() {
    let store = Arc::new(InMemoryLeaseStore::new());
    let host_a = host("cli-a", store.clone());
    let host_b = host("cli-b", store.clone());

    let handle_a = host_a.acquire_lease("cycle-1", 60_000).expect("cli-a wins");
    let result_b = host_b.acquire_lease("cycle-1", 60_000);

    // Only one authority: the second CLI gets a conflict.
    match result_b {
        Err(LeaseError::Conflict {
            ref owner,
            fencing_token,
        }) => {
            assert_eq!(owner, "cli-a");
            assert_eq!(fencing_token, handle_a.fencing_token());
        }
        other => panic!("expected LeaseError::Conflict, got {other:?}"),
    }
}

#[test]
fn loser_acquires_after_winner_releases() {
    let store = Arc::new(InMemoryLeaseStore::new());
    let host_a = host("cli-a", store.clone());
    let host_b = host("cli-b", store.clone());

    let handle_a = host_a.acquire_lease("cycle-1", 60_000).expect("cli-a wins");
    handle_a.release().expect("release");

    let handle_b = host_b
        .acquire_lease("cycle-1", 60_000)
        .expect("cli-b wins after release");
    // Fencing token advances monotonically across acquirers.
    assert_eq!(handle_b.fencing_token(), 2);
}

#[test]
fn different_cycle_ids_no_conflict() {
    let store = Arc::new(InMemoryLeaseStore::new());
    let host_a = host("cli-a", store.clone());
    let host_b = host("cli-b", store.clone());

    let _handle_a = host_a
        .acquire_lease("cycle-1", 60_000)
        .expect("cli-a wins c1");
    let _handle_b = host_b
        .acquire_lease("cycle-2", 60_000)
        .expect("cli-b wins c2 (no conflict)");
}

#[test]
fn lease_store_state_divergence_check() {
    let store = Arc::new(InMemoryLeaseStore::new());
    let host_a = host("cli-a", store.clone());
    let host_b = host("cli-b", store.clone());

    let handle_a = host_a.acquire_lease("cycle-1", 60_000).expect("cli-a wins");
    let _ = host_b.acquire_lease("cycle-1", 60_000); // expected conflict

    // After the failed attempt, re-acquire with the same owner as A is a
    // no-op re-fence: the store must still show exactly the winner's state
    // (single record, owner cli-a, token not advanced by the loser).
    use sddk_engine::LeaseStore;
    let view = store
        .acquire("cycle-1", "cli-a", 1000, 61_000)
        .expect("re-fence same owner");
    assert_eq!(view.owner, "cli-a");
    assert_eq!(view.fencing_token, handle_a.fencing_token() + 1);
    assert!(
        !store
            .release("cycle-1", "cli-b", 999)
            .expect("release no-op")
    );
    // Loser cannot release what it never held: state stays with cli-a.
    let still = store.acquire("cycle-1", "cli-b", 1000, 62_000);
    assert!(still.is_err(), "loser must not take over while cli-a holds");
}
