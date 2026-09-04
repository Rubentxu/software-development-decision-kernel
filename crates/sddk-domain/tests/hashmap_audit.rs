//! CI lint: walk IR canonical forms and assert NO HashMap field participates.
//! (REQ-IRDT-HS-05, AC-IRDT-10).
//!
//! This test walks `crates/sddk-domain/src/*.rs` and checks that no struct
//! which derives `Serialize, Deserialize` AND has a `compute_content_hash` or
//! `plan_identity` method contains a `HashMap` field.

use std::path::Path;

/// Returns all `.rs` source files under src/ (not tests/).
fn domain_source_files() -> Vec<std::path::PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&src) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files
}

/// Checks if a file is an IR canonical form file.
/// IR canonical forms are files that define types with compute_content_hash or
/// plan_identity methods AND derive Serialize + Deserialize.
fn is_ir_canonical_form(path: &Path) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Check if the file defines types that:
    // 1. Have compute_content_hash or plan_identity method
    // 2. Derive Serialize AND Deserialize
    let has_hash_method = content.contains("fn compute_content_hash")
        || content.contains("fn plan_identity");
    let has_serde_derives = content.contains("Serialize") && content.contains("Deserialize");

    has_hash_method && has_serde_derives
}

/// Returns true if the given source file contains a `HashMap` field declaration
/// (type path containing `HashMap`).
fn file_has_hashmap_field(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("//!") {
            continue;
        }
        // Look for HashMap<...> type annotations in struct fields
        // Pattern: field_name: HashMap<...> or field_name: std::collections::HashMap<...>
        if trimmed.contains("HashMap<") && trimmed.contains(':') {
            // Basic heuristic: field declaration with HashMap type
            return Some(line.to_string());
        }
    }
    None
}

#[test]
fn no_hashmap_in_ir_canonical_forms() {
    let mut violations = Vec::new();
    for file_path in domain_source_files() {
        // Only check IR canonical form files
        if !is_ir_canonical_form(&file_path) {
            continue;
        }

        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Some(line) = file_has_hashmap_field(&file_path) {
            violations.push(format!(
                "{}: {}",
                file_name,
                line
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "HashMap fields found in IR canonical form files (must use BTreeMap instead):\n{}",
        violations.join("\n")
    );
}
