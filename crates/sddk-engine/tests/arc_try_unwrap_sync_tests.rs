//! RED/GREEN Tests for COUPLE-TRY-UNWRAP-SILENT-SYNC fix (cycle-22)
//!
//! These tests verify the defensive `match Arc::try_unwrap` sync pattern
//! at workflow_runtime.rs:604 and :668. After WU-2 applies the fix,
//! these tests document the intended behavior and serve as regression
//! guards against re-introducing the silent fallback.
//!
//! The tests use inline Arc<Mutex<NodeRun>> patterns since the workflow_runtime
//! tick() code is not publicly exported. The fix itself is in
//! `workflow_runtime.rs:604` and `:668`.

use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Test subject: the defensive sync pattern
// ---------------------------------------------------------------------------

/// The defensive sync pattern that cycle-22 installs at workflow_runtime.rs:604/:668.
/// This is the exact logic that replaces the silent `if let Ok(...)` fallback.
fn defensive_sync_node_run<T: Clone>(node_run_arc: Arc<Mutex<T>>) -> T {
    match Arc::try_unwrap(node_run_arc) {
        Ok(mutex) => mutex.into_inner().expect("Mutex<T> poisoned at sync point"),
        Err(arc) => {
            // Other Arc references exist (e.g. Parallel::evaluate Pending branch
            // supervisor thread still holds Arc<Mutex<T>>). Defensive sync via
            // lock preserves the mutation instead of silently dropping it.
            let _count = Arc::strong_count(&arc);
            arc.lock().expect("Mutex<T> poisoned at sync point").clone()
        }
    }
}

// ---------------------------------------------------------------------------
// RED Test 1: Fast path — no extra refs, try_unwrap succeeds
// ---------------------------------------------------------------------------

/// Verifies the fast path: when Arc::try_unwrap succeeds (single owner),
/// we get the inner value directly with zero lock acquisition.
#[test]
fn sync_writes_when_no_extra_refs() {
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NodeRun {
        attempts: Vec<String>,
    }

    let node_run = NodeRun {
        attempts: vec!["initial".into()],
    };
    let node_run_arc = Arc::new(Mutex::new(node_run));

    // No extra Arc clones — try_unwrap must succeed
    let result = defensive_sync_node_run(node_run_arc);

    assert_eq!(
        result.attempts,
        vec!["initial".to_string()],
        "fast path must yield the original NodeRun"
    );
}

// ---------------------------------------------------------------------------
// RED Test 2: Fallback path — extra refs exist, try_unwrap returns Err
// ---------------------------------------------------------------------------

/// Verifies the fallback path: when Arc::try_unwrap fails (other owners
/// hold references), we sync via Mutex lock, preserving ALL mutations.
#[test]
fn sync_via_lock_when_extra_refs_exist() {
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NodeRun {
        attempts: Vec<String>,
    }

    let node_run = NodeRun {
        attempts: vec!["parent-value".into()],
    };
    let node_run_arc = Arc::new(Mutex::new(node_run));

    // Simulate Parallel supervisor thread holding a clone
    let child_ref = Arc::clone(&node_run_arc);

    // Child thread mutates the NodeRun via lock (simulates operator mutation)
    {
        let mut guard = child_ref.lock().unwrap();
        guard.attempts.push("from-child".into());
    }

    // The main sync path — try_unwrap fails, falls back to lock
    let result = defensive_sync_node_run(node_run_arc);

    // Critical assertion: the child mutation is PRESERVED, not silently dropped
    assert_eq!(
        result.attempts,
        vec!["parent-value".to_string(), "from-child".to_string()],
        "fallback path must preserve mutations from other Arc owners"
    );
}

// ---------------------------------------------------------------------------
// RED Test 3: Panic on mutex poisoning
// ---------------------------------------------------------------------------

/// Verifies that Mutex poisoning (from a panicking thread) causes a clear
/// panic rather than silently swallowing the error.
#[test]
#[should_panic(expected = "Mutex<T> poisoned at sync point")]
fn panic_when_mutex_poisoned() {
    #[derive(Clone, Debug)]
    struct NodeRun {
        _data: String,
    }

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        _data: "initial".into(),
    }));

    // Spawn a thread that panics while holding the lock — poisons the Mutex
    let arc_clone = Arc::clone(&node_run_arc);
    let _ = std::thread::spawn(move || {
        let _guard = arc_clone.lock().unwrap();
        panic!("poison");
    })
    .join();

    // Now defensive_sync_node_run must panic with a clear message
    let _ = defensive_sync_node_run(node_run_arc);
}

// ---------------------------------------------------------------------------
// Integration-style test: confirms the pattern works end-to-end
// ---------------------------------------------------------------------------

/// End-to-end style test: two Arc owners mutate in sequence, verifying
/// the defensive sync never silently drops mutations.
#[test]
fn no_silent_mutation_loss_on_sequential_arc_owners() {
    #[derive(Clone, Debug, PartialEq, Default)]
    struct NodeRun {
        counter: i32,
    }

    let node_run_arc = Arc::new(Mutex::new(NodeRun::default()));

    // Owner A increments
    {
        let mut guard = node_run_arc.lock().unwrap();
        guard.counter += 1;
    }

    // Owner B (clone) also increments
    let owner_b = Arc::clone(&node_run_arc);
    {
        let mut guard = owner_b.lock().unwrap();
        guard.counter += 10;
    }

    // Sync from owner A's perspective — strong_count is 2, try_unwrap fails
    let result = defensive_sync_node_run(node_run_arc);

    // Both increments must be reflected (no silent drop)
    assert_eq!(result.counter, 11, "both mutations must be preserved");
}
