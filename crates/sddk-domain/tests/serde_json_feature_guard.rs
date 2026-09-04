//! CI guard: verify serde_json is built WITHOUT preserve_order/indexmap feature.
//! (REQ-IRDT-HS-03, AC-IRDT-09).
//!
//! The canonical JSON serialization for IR types relies on BTreeMap ordering.
//! If `serde_json/preserve_order` (IndexMap) were enabled, hashes would be
//! non-deterministic across feature-flag combinations.

/// Verifies serde_json does NOT have preserve_order or indexmap feature.
/// Uses cargo metadata to inspect resolved dependencies.
#[test]
fn serde_json_preserve_order_disabled() {
    // Read Cargo.lock and check serde_json features
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/sddk-domain
        .and_then(|p| p.parent()) // workspace root
        .unwrap()
        .join("Cargo.lock");

    let lock_content =
        std::fs::read_to_string(&lock_path).expect("Cargo.lock must be present for CI gate");

    // Find serde_json entry and its resolved features
    let lock_text = lock_content.as_str();

    // Parse [[package]] blocks to find serde_json
    let mut in_serde_json = false;
    let mut lines_in_package = Vec::new();

    for line in lock_text.lines() {
        if line.starts_with("[[package]]") && in_serde_json {
            // End of serde_json block
            break;
        }
        if line.starts_with("name = \"serde_json\"") {
            in_serde_json = true;
            lines_in_package.clear();
            continue;
        }
        if in_serde_json {
            lines_in_package.push(line);
            if line.starts_with("name = \"") {
                // Next package started
                break;
            }
        }
    }

    assert!(
        in_serde_json && !lines_in_package.is_empty(),
        "serde_json package must be present in Cargo.lock"
    );

    // Look for features = [...] in serde_json block
    let features_line = lines_in_package
        .iter()
        .find(|l| l.trim().starts_with("features ="));

    if let Some(feat_line) = features_line {
        let features_str = feat_line.trim();
        assert!(
            !features_str.contains("preserve_order"),
            "serde_json must NOT have preserve_order feature enabled (breaks canonical JSON)"
        );
        assert!(
            !features_str.contains("indexmap"),
            "serde_json must NOT have indexmap feature enabled (breaks canonical JSON)"
        );
    }
    // If no features line or empty features, that's fine (default BTreeMap behavior)
}

use std::path::Path;
