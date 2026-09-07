//! Determinism audit: graph_store.rs run-persistence functions must not use non-deterministic primitives
//! for identity inputs.
//! (REQ-WFR-ID-001, REQ-WFR-PERSIST-001)
//!
//! This test walks the RUN-PERSISTENCE FUNCTIONS in `crates/sddk-storage/src/graph_store.rs`
//! (record_run, record_graph_revision_for_run, record_node_run_for_run, record_attempt,
//! load_run, load_node_run, list_attempts, latest_attempt, stream_node_runs,
//! latest_workflow_run_state, load_revision) and asserts they do NOT reference:
//! - `Uuid::new_v4` (random UUID generation — identity must be deterministic)
//! - `HashMap` (non-deterministic iteration order — use BTreeMap)
//!
//! SystemTime and Instant are NOT flagged here because lifecycle event timestamps
//! (occurred_at) REQUIRE wall-clock time per REQ-WFR-EV-001 (RFC 3339, non-decreasing).
//! The spec's intent is identity derivation determinism, not timestamp exclusion.

use std::path::Path;

fn graph_store_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("graph_store.rs")
}

type PathBuf = std::path::PathBuf;

/// Returns the line indices of the run-persistence function bodies in graph_store.rs.
/// We identify them by their fn signature lines.
fn run_persistence_function_lines(content: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let function_names = [
        "fn record_run",
        "fn record_graph_revision_for_run",
        "fn record_node_run_for_run",
        "fn record_node_run(",
        "fn record_attempt",
        "fn load_run",
        "fn load_node_run",
        "fn list_attempts",
        "fn latest_attempt",
        "fn stream_node_runs",
        "fn latest_workflow_run_state",
        "fn load_revision",
    ];

    let mut result = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        for fname in &function_names {
            if line.contains(fname) {
                result.push((idx, line.trim().to_string()));
                break;
            }
        }
    }
    result
}

/// Checks if a line contains a forbidden identity-determinism symbol.
/// Skips comments and string literals.
fn line_has_forbidden_symbol(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Skip comment-only lines
    if trimmed.starts_with("//") || trimmed.starts_with("//!") {
        return None;
    }

    let forbidden = [
        ("Uuid::new_v4", "Uuid::new_v4"),
        ("HashMap<", "HashMap"),
    ];

    for (symbol, display) in &forbidden {
        if trimmed.contains(symbol) {
            return Some(display.to_string());
        }
    }
    None
}

/// Scans run-persistence function bodies for forbidden symbols.
fn scan_run_persistence_functions(content: &str) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let fn_lines = run_persistence_function_lines(content);
    let mut violations = Vec::new();

    for (fn_start_idx, fn_signature) in fn_lines {
        // Scan from fn signature until we hit the next fn at the same or lower indentation
        // (heuristic: next line that starts a new fn at column 0 or less indentation)
        let fn_indent = lines[fn_start_idx]
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();

        let mut i = fn_start_idx + 1;
        while i < lines.len() {
            let line = lines[i];
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();

            // If we hit a line at or before the fn's indent level that's a new fn, stop
            if indent <= fn_indent && lines[i].trim().starts_with("fn ") {
                break;
            }

            if let Some(forbidden) = line_has_forbidden_symbol(line) {
                violations.push(format!(
                    "{} — line {}: {} — contains {}",
                    fn_signature,
                    i + 1,
                    line.trim(),
                    forbidden
                ));
            }
            i += 1;
        }
    }

    violations
}

/// Scenario: graph_store run-persistence functions are deterministic for identity
/// WHEN crates/sddk-storage/src/graph_store.rs run-persistence functions are inspected
/// THEN they contain no Uuid::new_v4 or HashMap (SystemTime is allowed for occurred_at)
#[test]
fn graph_store_run_persistence_deterministic() {
    let path = graph_store_path();
    let content = std::fs::read_to_string(&path)
        .expect("failed to read graph_store.rs");
    let violations = scan_run_persistence_functions(&content);

    assert!(
        violations.is_empty(),
        "graph_store.rs run-persistence functions must not use non-deterministic identity primitives:\n{}",
        violations.join("\n")
    );
}
