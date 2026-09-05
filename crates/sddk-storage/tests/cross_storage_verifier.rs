//! Cross-storage verifier integration tests.
//!
//! Cross-storage drift detection is tested at the domain layer using
//! FakePlanningGraphRead (see planning_cross_storage.rs). Those tests cover
//! aligned/mismatched CAS roots, v1 chain backward compat, and strict mode.
//!
//! These integration tests verify the handle_id and cas_root_id stability
//! using the public Storage API.

use sddk_storage::Storage;

/// Scenario: handle_id is stable and non-empty
#[test]
fn storage_handle_id_is_stable() {
    let storage = Storage::open_in_memory().unwrap();
    let handle = storage.handle_id();
    assert!(!handle.is_empty(), "handle_id should be non-empty");
    assert_eq!(handle, storage.handle_id(), "handle_id should be stable");
}

/// Scenario: cas_root_id is stable and deterministic for a given Storage instance
#[test]
fn storage_cas_root_id_is_stable() {
    let storage = Storage::open_in_memory().unwrap();
    let id1 = storage.cas_root_id();
    let id2 = storage.cas_root_id();
    assert_eq!(id1, id2, "cas_root_id should be stable for same instance");
    assert!(!id1.is_empty(), "cas_root_id should be non-empty");
}

/// Scenario: Different Storage::open_in_memory() instances share the same default CAS root
/// (by design — FilesystemCas::default_root() is a fixed path).
/// This is why cross-storage drift must be tested at the domain layer with FakePlanningGraphRead.
#[test]
fn storage_in_memory_shares_default_cas_root() {
    let storage1 = Storage::open_in_memory().unwrap();
    let storage2 = Storage::open_in_memory().unwrap();

    // Both instances use FilesystemCas::default_root(), so they have the same CAS root
    assert_eq!(
        storage1.cas_root_id(),
        storage2.cas_root_id(),
        "open_in_memory instances share the same default CAS root"
    );
}
