//! AIW-S8 X06 — Storage schema versioning integration tests.
//!
//! A CLI built against an older schema version must fail closed against
//! data written by a newer schema, never interpret it falsely.

use sddk_storage::{
    COMPILED_SCHEMA_VERSION, MIN_SUPPORTED_SCHEMA_VERSION, SchemaCompatibility, Storage,
    assert_compatible, check_compatibility,
};

#[test]
fn fresh_storage_reports_exact_compatibility() {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = Storage::open(dir.path().join("ledger.sqlite")).expect("open");
    let compat = check_compatibility(&storage).expect("check");
    assert!(matches!(compat, SchemaCompatibility::Exact));
}

#[test]
fn assert_compatible_succeeds_on_fresh_storage() {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = Storage::open(dir.path().join("ledger.sqlite")).expect("open");
    assert!(assert_compatible(&storage).is_ok());
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn compiled_version_is_current_and_minimum_is_sane() {
    assert!(COMPILED_SCHEMA_VERSION >= 1);
    assert!(MIN_SUPPORTED_SCHEMA_VERSION >= 1);
    assert!(MIN_SUPPORTED_SCHEMA_VERSION <= COMPILED_SCHEMA_VERSION);
}

#[test]
fn schema_version_matches_latest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = Storage::open(dir.path().join("ledger.sqlite")).expect("open");
    let version = storage.schema_version().expect("version");
    assert_eq!(version, COMPILED_SCHEMA_VERSION);
}
