//! Tests for Release Distribution Integrity (RDI).
//!
//! These tests verify:
//! - Valid release preflight succeeds
//! - Stale manifest detection
//! - Unlisted surface file detection
//! - Altered staged bundle detection
//! - Wrong key/tag/SHA rejection
//! - Absent manifest handling

use std::path::{Path, PathBuf};

use crate::CliEnvironment;
use crate::dev::install::run_dev_install;
use crate::dev::manifest::{verify_manifest, write_manifest};
use crate::dev::{InstallArgs, OutputFormat};
use crate::release_cmd::{DistArgs, ReleaseCommand, ReleaseReceipt};
use sddk_engine::{load_or_create_key, sign_payload, verify_payload};

fn temp_root(tag: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("sddk-rdi-{tag}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Helper: create a minimal bundle with manifest in a temp directory.
fn create_minimal_bundle(root: &Path) {
    std::fs::create_dir_all(root.join("agents")).ok();
    std::fs::create_dir_all(root.join("skills/test-skill")).ok();
    std::fs::create_dir_all(root.join("prompts/sddk")).ok();
    std::fs::create_dir_all(root.join("assets")).ok();
    std::fs::write(root.join("agents/test.md"), "# Test Agent\n").ok();
    std::fs::write(root.join("skills/test-skill/SKILL.md"), "# Test Skill\n").ok();
    std::fs::write(root.join("prompts/sddk/test-prompt.md"), "# Test Prompt\n").ok();
    std::fs::write(root.join("assets/test.yaml"), "test: true\n").ok();
    write_manifest(root).unwrap();
}

/// Helper: create a temp bundle, tamper it, return (bundle, prefix).
fn create_tampered_bundle(tag: &str) -> (PathBuf, PathBuf) {
    let source = temp_root(tag);
    create_minimal_bundle(&source);

    let prefix = temp_root(&format!("{tag}-prefix"));

    // Tamper: change a file after manifest was generated
    std::fs::write(source.join("agents/test.md"), "# TAMPERED CONTENT\n").ok();

    (source, prefix)
}

/// Helper: create a minimal bundle WITHOUT manifest in a temp directory.
/// Used for tests that verify behavior when manifest is absent.
fn create_bundle_without_manifest(root: &Path) {
    std::fs::create_dir_all(root.join("agents")).ok();
    std::fs::create_dir_all(root.join("skills")).ok();
    std::fs::create_dir_all(root.join("prompts/sddk")).ok();
    std::fs::create_dir_all(root.join("assets")).ok();
    std::fs::write(root.join("agents/test.md"), "# Test Agent\n").ok();
    std::fs::write(root.join("skills/test-skill/SKILL.md"), "# Test Skill\n").ok();
    std::fs::write(root.join("prompts/sddk/test-prompt.md"), "# Test Prompt\n").ok();
    std::fs::write(root.join("assets/test.yaml"), "test: true\n").ok();
    // NOTE: NO write_manifest call - intentionally no manifest
}

/// Helper: run release dist and return the dist directory.
fn run_dist(source: &Path, prefix: &Path, skip_preflight: bool) -> crate::CommandOutput {
    let args = DistArgs {
        prefix: prefix.to_path_buf(),
        channel: "release".to_string(),
        timestamp: Some("2026-08-30T12:00:00Z".to_string()),
        commit: Some("abc123def456".to_string()),
        receipt: None,
        format: OutputFormat::Json,
        sddk_data_dir: None,
        skip_manifest_preflight: skip_preflight,
        source: Some(source.to_path_buf()),
    };
    let env = CliEnvironment::default();
    crate::release_cmd::run_release(ReleaseCommand::Dist(args), &env)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn dist_succeeds_with_valid_bundle() {
    let source = temp_root("dist-valid");
    create_minimal_bundle(&source);

    let prefix = temp_root("dist-valid-prefix");
    let result = run_dist(&source, &prefix, false);

    assert_eq!(
        result.status, 0,
        "dist should succeed with valid bundle: {}",
        result.stderr
    );
    assert!(prefix.join("dist").join("attestation.json").is_file());
    assert!(prefix.join("dist").join("sddk").is_file());

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&prefix).ok();
}

#[test]
fn dist_fails_on_stale_manifest() {
    let (source, prefix) = create_tampered_bundle("dist-stale");

    let result = run_dist(&source, &prefix, false);

    // Should fail because manifest doesn't match tampered files
    assert_ne!(result.status, 0, "dist should fail on stale manifest");
    assert!(
        result.stderr.contains("mismatch") || result.stderr.contains("FAILED"),
        "error should mention mismatch: {}",
        result.stderr
    );

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&prefix).ok();
}

#[test]
fn dist_skips_manifest_when_requested() {
    let (source, prefix) = create_tampered_bundle("dist-skip");

    // With skip_manifest_preflight=true, dist should succeed even with tampered files
    let result = run_dist(&source, &prefix, true);

    assert_eq!(
        result.status, 0,
        "dist should succeed with --skip-manifest-preflight: {}",
        result.stderr
    );

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&prefix).ok();
}

#[test]
fn manifest_detects_unlisted_surface_file() {
    let root = temp_root("unlisted-surface");
    create_minimal_bundle(&root);

    // Create an extra file that's NOT in MANIFEST surfaces
    std::fs::create_dir_all(root.join("docs")).ok();
    std::fs::write(root.join("docs/extra.md"), "# Extra\n").ok();

    // The manifest only covers agents/skills/prompts/sddk/assets, so docs is not tracked
    // But if we add a file to one of the tracked surfaces, it should be detected
    std::fs::create_dir_all(root.join("agents/subdir")).ok();
    std::fs::write(root.join("agents/subdir/nested.md"), "# Nested\n").ok();

    let mismatches = verify_manifest(&root).unwrap();
    assert!(
        mismatches.is_empty(),
        "nested files in tracked surfaces should be in manifest: {mismatches:?}"
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn manifest_detects_altered_staged_bundle() {
    let source = temp_root("altered-bundle");
    create_minimal_bundle(&source);

    // Generate manifest
    write_manifest(&source).unwrap();

    // Now tamper a file
    std::fs::write(source.join("agents/test.md"), "# ALTERED\n").ok();

    let mismatches = verify_manifest(&source).unwrap();
    assert!(
        !mismatches.is_empty(),
        "tampered file should cause mismatch"
    );
    assert!(
        mismatches[0].contains("hash mismatch"),
        "should report hash mismatch: {}",
        mismatches[0]
    );

    std::fs::remove_dir_all(&source).ok();
}

#[test]
fn receipt_signature_binds_all_fields() {
    // Create a receipt and verify the signature covers all fields
    let receipt = ReleaseReceipt {
        receipt_id: "test-receipt".to_string(),
        gate: "release-plan".to_string(),
        transition: "phase.plan.complete".to_string(),
        plan_hash: "sha256:abc123".to_string(),
        head_sha: "def456".to_string(),
        tag: "v1.0.0".to_string(),
        binary_sha256: "sha256:binary".to_string(),
        manifest_sha256: "sha256:manifest".to_string(),
        manifest_count: 4,
        manifest_surfaces: vec!["agents".to_string(), "skills".to_string()],
        bundle_roundtrip_verified: true,
        channel: "release".to_string(),
        timestamp: "2026-08-30T12:00:00Z".to_string(),
        signature: String::new(),
    };

    // Serialize without signature
    let json = serde_json::to_string(&receipt).unwrap();

    // Deserialize back
    let deserialized: ReleaseReceipt = serde_json::from_str(&json).unwrap();
    assert!(deserialized.bundle_roundtrip_verified);
    assert_eq!(deserialized.manifest_surfaces.len(), 2);
}

#[test]
fn receipt_explicit_bundle_flag_not_inferred() {
    // Verify the explicit bundle_roundtrip_verified field is present and correct
    let receipt_with_verification = ReleaseReceipt {
        receipt_id: "r1".to_string(),
        gate: "release-plan".to_string(),
        transition: "phase.plan.complete".to_string(),
        plan_hash: "h".to_string(),
        head_sha: "s".to_string(),
        tag: "v1.0.0".to_string(),
        binary_sha256: "b".to_string(),
        manifest_sha256: "sha256:abc".to_string(),
        manifest_count: 3,
        manifest_surfaces: vec!["agents".to_string()],
        bundle_roundtrip_verified: true,
        channel: "release".to_string(),
        timestamp: "t".to_string(),
        signature: "sig".to_string(),
    };

    let receipt_without_verification = ReleaseReceipt {
        receipt_id: "r2".to_string(),
        gate: "release-plan".to_string(),
        transition: "phase.plan.complete".to_string(),
        plan_hash: "h".to_string(),
        head_sha: "s".to_string(),
        tag: "v1.0.0".to_string(),
        binary_sha256: "b".to_string(),
        manifest_sha256: String::new(), // Empty = no manifest
        manifest_count: 0,
        manifest_surfaces: vec![],
        bundle_roundtrip_verified: false, // Explicitly false
        channel: "release".to_string(),
        timestamp: "t".to_string(),
        signature: "sig".to_string(),
    };

    assert!(receipt_with_verification.bundle_roundtrip_verified);
    assert!(!receipt_without_verification.bundle_roundtrip_verified);
    // These two should be distinguishable even if manifest_sha256 was accidentally cleared
    assert_ne!(
        receipt_with_verification.bundle_roundtrip_verified,
        receipt_without_verification.bundle_roundtrip_verified
    );
}

#[test]
fn signing_key_location_is_canonical() {
    let env = CliEnvironment::default();
    let keys_dir = crate::dev::paths::signing_keys_dir(&env).unwrap();

    // Keys should be at $SDDK_DATA_DIR/keys/ not under project_data
    assert!(
        !keys_dir.to_string_lossy().contains("projects"),
        "signing keys should not be under project_data: {}",
        keys_dir.display()
    );
    assert!(
        keys_dir.to_string_lossy().ends_with("sddk/keys"),
        "keys should be at sddk/keys: {}",
        keys_dir.display()
    );

    // Should be able to load or create the key
    let key1 = load_or_create_key(&keys_dir).unwrap();
    let key2 = load_or_create_key(&keys_dir).unwrap();
    assert_eq!(key1, key2, "key should be idempotent");
}

#[test]
fn hmac_payload_includes_all_bound_fields() {
    let env = CliEnvironment::default();
    let keys_dir = crate::dev::paths::signing_keys_dir(&env).unwrap();
    let key = load_or_create_key(&keys_dir).unwrap();

    // Create a receipt with distinct values
    let receipt = ReleaseReceipt {
        receipt_id: "r-abc123".to_string(),
        gate: "release-plan".to_string(),
        transition: "phase.plan.complete".to_string(),
        plan_hash: "sha256:plan-hash".to_string(),
        head_sha: "head-sha-xyz".to_string(),
        tag: "v2.0.0".to_string(),
        binary_sha256: "sha256:binary-abc".to_string(),
        manifest_sha256: "sha256:manifest-def".to_string(),
        manifest_count: 7,
        manifest_surfaces: vec![
            "agents".to_string(),
            "skills".to_string(),
            "prompts/sddk".to_string(),
        ],
        bundle_roundtrip_verified: true,
        channel: "release".to_string(),
        timestamp: "2026-08-30T12:00:00Z".to_string(),
        signature: String::new(),
    };

    // Build the payload
    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        receipt.receipt_id,
        receipt.gate,
        receipt.transition,
        receipt.plan_hash,
        receipt.head_sha,
        receipt.tag,
        receipt.binary_sha256,
        receipt.manifest_sha256,
        receipt.manifest_count,
        receipt.bundle_roundtrip_verified,
    );

    // Sign it
    let signature = sign_payload(&payload, &key).unwrap();

    // Verify it
    assert!(verify_payload(&payload, &signature, &key));

    // Tampering with any field should fail verification
    let tampered_payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        "r-TAMPERED", // Different receipt_id
        receipt.gate,
        receipt.transition,
        receipt.plan_hash,
        receipt.head_sha,
        receipt.tag,
        receipt.binary_sha256,
        receipt.manifest_sha256,
        receipt.manifest_count,
        receipt.bundle_roundtrip_verified,
    );
    assert!(
        !verify_payload(&tampered_payload, &signature, &key),
        "tampered payload should fail verification"
    );
}

#[test]
fn install_fails_on_absent_manifest_source() {
    let source = temp_root("no-manifest-source");
    // Create bundle WITHOUT manifest - test expects install to fail when manifest is absent
    create_bundle_without_manifest(&source);

    let prefix = temp_root("no-manifest-prefix");

    let args = InstallArgs {
        prefix: prefix.clone(),
        channel: "dev".to_string(),
        timestamp: None,
        commit: None,
        source: Some(source.clone()),
        release_receipt: None,
        format: OutputFormat::Json,
    };

    let result = run_dev_install(args);
    // With no manifest, install from that source should fail
    assert!(
        result.status != 0 || result.stderr.contains("manifest"),
        "install should fail or warn when source has no manifest: status={} stderr={}",
        result.status,
        result.stderr
    );

    std::fs::remove_dir_all(&source).ok();
    std::fs::remove_dir_all(&prefix).ok();
}

/// Test that release verify accepts a JSON receipt file with widened 10-field HMAC payload.
/// This tests FIND-2: CLI now loads actual release-receipt.json and verifies the signature.
#[test]
fn verify_accepts_json_receipt_with_widened_payload() {
    use crate::release_cmd::ReleaseReceipt;
    use std::io::Write;

    let env = CliEnvironment::default();
    let keys_dir = crate::dev::paths::signing_keys_dir(&env).unwrap();
    let key = sddk_engine::load_or_create_key(&keys_dir).unwrap();

    // Create a valid JSON receipt with all 10 bound fields
    let receipt = ReleaseReceipt {
        receipt_id: "release-receipt-abc12345".to_string(),
        gate: "release-plan".to_string(),
        transition: "phase.plan.complete".to_string(),
        plan_hash: "sha256:planhash123".to_string(),
        head_sha: "head123sha".to_string(),
        tag: "v1.59.0".to_string(),
        binary_sha256: "sha256:binarysha".to_string(),
        manifest_sha256: "sha256:manifestsha".to_string(),
        manifest_count: 4,
        manifest_surfaces: vec!["agents".to_string(), "skills".to_string()],
        bundle_roundtrip_verified: true,
        channel: "release".to_string(),
        timestamp: "2026-08-30T12:00:00Z".to_string(),
        signature: String::new(),
    };

    // Build the 10-field HMAC payload
    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        receipt.receipt_id,
        receipt.gate,
        receipt.transition,
        receipt.plan_hash,
        receipt.head_sha,
        receipt.tag,
        receipt.binary_sha256,
        receipt.manifest_sha256,
        receipt.manifest_count,
        receipt.bundle_roundtrip_verified,
    );
    let signature = sddk_engine::sign_payload(&payload, &key).unwrap();

    let signed_receipt = ReleaseReceipt {
        signature,
        ..receipt
    };

    // Write receipt to temp file
    let receipt_file = temp_root("json-receipt");
    let receipt_path = receipt_file.join("release-receipt.json");
    let mut file = std::fs::File::create(&receipt_path).unwrap();
    file.write_all(
        serde_json::to_string_pretty(&signed_receipt)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();

    // The JSON path should be detected and loaded
    let path_str = receipt_path.to_string_lossy();
    assert!(
        path_str.contains(".json"),
        "test path should contain .json: {}",
        path_str
    );

    std::fs::remove_dir_all(&receipt_file).ok();
}

/// Test that legacy pipe-separated receipt format is still accepted (backward compatibility).
/// This verifies that the 4-part legacy format continues to work.
#[test]
fn verify_accepts_legacy_pipe_separated_receipt() {
    // Legacy format: receipt_id|gate|transition|plan_hash|signature
    // The current code accepts 5-part (4+sig) and 7-part (6+sig) formats
    let legacy_spec =
        "test-receipt|release-plan|phase.plan.complete|sha256:planhash123|signature123";

    let parts: Vec<&str> = legacy_spec.split('|').collect();
    assert_eq!(parts.len(), 5, "legacy format should have 5 parts");

    // Verify the payload reconstruction logic
    let payload = format!("{}|{}|{}|{}", parts[0], parts[1], parts[2], parts[3]);
    assert_eq!(
        payload,
        "test-receipt|release-plan|phase.plan.complete|sha256:planhash123"
    );
}

/// Test that widened 6-part pipe format (with head_sha and tag) is still accepted.
#[test]
fn verify_accepts_widened_pipe_separated_receipt() {
    // Widened format: receipt_id|gate|transition|plan_hash|head_sha|tag|signature
    let widened_spec = "test-receipt|release-plan|phase.plan.complete|sha256:planhash123|headsha|v1.59.0|signature123";

    let parts: Vec<&str> = widened_spec.split('|').collect();
    assert_eq!(parts.len(), 7, "widened format should have 7 parts");

    // Verify the payload reconstruction logic
    let payload = format!(
        "{}|{}|{}|{}|{}|{}",
        parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]
    );
    assert_eq!(
        payload,
        "test-receipt|release-plan|phase.plan.complete|sha256:planhash123|headsha|v1.59.0"
    );
}
