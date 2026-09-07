//! Determinism audit: workflow_run.rs must not use non-deterministic primitives.
//! (REQ-WFR-ID-001)
//!
//! This test walks `crates/sddk-domain/src/workflow_run.rs` and asserts it does NOT reference:
//! - `Uuid::new_v4` (random UUID generation)
//! - `SystemTime` (wall-clock time)
//! - `Instant` (monotonic time)
//! - `HashMap` (non-deterministic iteration order)
//!
//! Pattern modelled on `hashmap_audit.rs`.

use std::path::Path;

fn workflow_run_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("workflow_run.rs")
}

type PathBuf = std::path::PathBuf;

/// Checks if a line contains a forbidden non-deterministic symbol.
/// Skips comments and string literals.
fn line_has_forbidden_symbol(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Skip comment-only lines
    if trimmed.starts_with("//") || trimmed.starts_with("//!") {
        return None;
    }

    let forbidden = [
        ("Uuid::new_v4", "Uuid::new_v4"),
        ("SystemTime", "SystemTime"),
        ("Instant", "Instant"),
        ("HashMap<", "HashMap"),
    ];

    for (symbol, display) in &forbidden {
        if trimmed.contains(symbol) {
            return Some(display.to_string());
        }
    }
    None
}

/// Scans a file for forbidden symbols, returning violations.
fn scan_file_for_forbidden(path: &Path, relative_name: &str) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            panic!("failed to read {}: {}", relative_name, e);
        }
    };

    let mut violations = Vec::new();
    for (line_idx, line) in content.lines().enumerate() {
        if let Some(forbidden) = line_has_forbidden_symbol(line) {
            violations.push(format!(
                "{}:{}: {} — contains {}",
                relative_name,
                line_idx + 1,
                line.trim(),
                forbidden
            ));
        }
    }
    violations
}

/// Scenario: run persistence path is deterministic
/// WHEN crates/sddk-domain/src/workflow_run.rs (RunId::derive) is inspected
/// THEN it contains no Uuid::new_v4, SystemTime, Instant, or HashMap
#[test]
fn run_persistence_path_is_deterministic() {
    let path = workflow_run_path();
    let violations = scan_file_for_forbidden(&path, "workflow_run.rs");

    assert!(
        violations.is_empty(),
        "workflow_run.rs persistence path must not use non-deterministic primitives:\n{}",
        violations.join("\n")
    );
}
