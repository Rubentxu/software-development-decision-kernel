//! Probes for R2: partial / corrupt CAS objects must NOT be silently accepted.
//!
//! Scenarios:
//! - `a5_2_r2_truncated_blob` - bytes truncated after `put`; `get` must reject.
//! - `a5_2_r2_wrong_content` - replace the bytes at the path with content whose
//!   hash differs from the requested key; `get` must return `HashMismatch`
//!   rather than the substituted bytes.
//! - `a5_2_r2_unreadable_path` - replace the file path with a directory; `get`
//!   must surface a typed storage error and not panic.
//! - `a5_2_r2_other_valid_blobs_survive` - a corruption localised to one blob
//!   must not affect reads of other valid blobs in the same store.
//!
//! See `crates/sddk-storage/src/cas.rs::FilesystemCas::get` and
//! `docs/history/legacy-packages/architecture-a5-a6/architecture-a5/A5-2-PLAN.md` (F4).

use sddk_domain::ports::{CasError, CasPort};
use sddk_storage::cas::FilesystemCas;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper: locate the on-disk file backing a `sha256:<hex>` reference.
fn cas_path(root: &Path, hash: &str) -> std::path::PathBuf {
    let hex = hash.strip_prefix("sha256:").unwrap_or(hash);
    let (p1, p2) = (&hex[0..2], &hex[2..4]);
    root.join(p1).join(p2).join(hex)
}

#[test]
fn a5_2_r2_truncated_blob_get_rejects_with_mismatch() {
    let td = TempDir::new().expect("tempdir");
    let cas = FilesystemCas::new(td.path()).expect("cas init");
    let content = b"hello, world - content addressed, please store me.";
    let hash = cas.put(content).expect("put ok");

    // Truncate the on-disk blob.
    let path = cas_path(td.path(), &hash);
    let original_len = fs::metadata(&path).expect("metadata").len();
    assert!(
        original_len > 4,
        "baseline: content is large enough to truncate"
    );
    let truncated = &content[..original_len as usize / 2];
    fs::write(&path, truncated).expect("truncate");

    // The get MUST surface HashMismatch rather than returning the truncated
    // bytes silently.
    let err = cas
        .get(&hash)
        .expect_err("get must reject the truncated blob");
    assert!(
        matches!(err, CasError::HashMismatch { .. }),
        "expected HashMismatch on truncated blob, got: {err:?}"
    );
}

#[test]
fn a5_2_r2_wrong_content_get_rejects_with_mismatch() {
    let td = TempDir::new().expect("tempdir");
    let cas = FilesystemCas::new(td.path()).expect("cas init");
    let content = b"original content alpha-1";
    let hash = cas.put(content).expect("put ok");

    // Replace the bytes at the path with completely different bytes.
    fs::write(cas_path(td.path(), &hash), b"unrelated").expect("rewrite");

    let err = cas
        .get(&hash)
        .expect_err("get must reject substituted bytes");
    assert!(
        matches!(err, CasError::HashMismatch { .. }),
        "expected HashMismatch on substituted bytes, got: {err:?}"
    );
}

#[test]
fn a5_2_r2_unreadable_path_returns_typed_storage_error() {
    let td = TempDir::new().expect("tempdir");
    let cas = FilesystemCas::new(td.path()).expect("cas init");
    let content = b"will be replaced with a directory";
    let hash = cas.put(content).expect("put ok");

    // Replace the file with a directory at the path.
    let path = cas_path(td.path(), &hash);
    fs::remove_file(&path).expect("remove file");
    fs::create_dir(&path).expect("create dir in place of file");

    let err = cas
        .get(&hash)
        .expect_err("get must reject unreadable CAS path");
    // Either a HashMismatch (we read the empty dir as bytes) or a typed
    // Storage error is acceptable, but it MUST NOT silently return the
    // dir's listing as a Vec<u8>.
    match err {
        CasError::HashMismatch { .. } | CasError::Storage(_) => {}
        other => panic!("expected HashMismatch or Storage error, got: {other:?}"),
    }
}

#[test]
fn a5_2_r2_corruption_localised_other_valid_blobs_survive() {
    let td = TempDir::new().expect("tempdir");
    let cas = FilesystemCas::new(td.path()).expect("cas init");

    let good_a = cas.put(b"good content alpha").expect("put a");
    let bad_b = cas
        .put(b"good content beta - to be corrupted")
        .expect("put b");
    let good_c = cas.put(b"good content gamma").expect("put c");

    // Corrupt only the second blob.
    fs::write(cas_path(td.path(), &bad_b), b"corrupt").expect("rewrite b");

    // The two good blobs must remain readable, and the bad one must reject.
    assert_eq!(cas.get(&good_a).expect("a ok"), b"good content alpha");
    assert_eq!(cas.get(&good_c).expect("c ok"), b"good content gamma");
    let err = cas.get(&bad_b).expect_err("b must reject");
    assert!(matches!(err, CasError::HashMismatch { .. }), "got: {err:?}");
}
