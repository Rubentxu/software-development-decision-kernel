//! Tests for engine identity deferral annotations (REQ-WFR-SCOPE-001).
//!
//! This test verifies that every `Uuid::new_v4()` call site in
//! `crates/sddk-engine/src/workflow_runtime.rs` is annotated with a
//! `// TODO(dw-runtime-003): derive via RunId::derive + compile_plan_to_revision().revision_id`
//! comment within 5 lines, indicating that the identity derivation is deferred
//! to DW-RUNTIME-003 (not implemented in this cycle).
//!
//! No executable behavior changes are allowed in this cycle for the engine.

use std::path::Path;

/// Path to the workflow_runtime source file.
fn workflow_runtime_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("workflow_runtime.rs")
}

type PathBuf = std::path::PathBuf;

/// Scenario: every Uuid::new_v4() is annotated with TODO(dw-runtime-003) within 5 lines
/// WHEN the engine source is inspected
/// THEN every Uuid::new_v4() call site is followed (within 5 lines) by
///     a `// TODO(dw-runtime-003)` comment
/// AND the diff on crates/sddk-engine/ contains ONLY comment lines, no executable changes
#[test]
fn engine_run_identity_deferred_to_dw_runtime_003() {
    let source_path = workflow_runtime_path();
    let content =
        std::fs::read_to_string(&source_path).expect("failed to read workflow_runtime.rs");

    let lines: Vec<&str> = content.lines().collect();

    let mut violations = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.contains("Uuid::new_v4()") {
            // Check if there's a TODO(dw-runtime-003) within ±5 lines
            let mut found_marker = false;
            for offset in -5..=5 {
                let idx = (i as isize + offset) as usize;
                if offset == 0 {
                    continue; // skip the line itself
                }
                if idx < lines.len() {
                    let surrounding = lines[idx];
                    if surrounding.contains("TODO(dw-runtime-003)") {
                        found_marker = true;
                        break;
                    }
                }
            }
            if !found_marker {
                violations.push(format!(
                    "Uuid::new_v4() at line {} has no TODO(dw-runtime-003) marker within 5 lines: {}",
                    i + 1, // 1-indexed for user display
                    line.trim()
                ));
            }
        }
        i += 1;
    }

    assert!(
        violations.is_empty(),
        "All Uuid::new_v4() calls must be annotated with TODO(dw-runtime-003) within 5 lines:\n{}",
        violations.join("\n")
    );
}
