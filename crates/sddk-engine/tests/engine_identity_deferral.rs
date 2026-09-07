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

/// Scenario: legacy constructors are deprecated
/// WHEN `new` and `new_with_event_store` are compiled
/// THEN both emit #[warn(deprecated)] when invoked
/// AND the test grep `legacy_constructors_are_deprecated` passes
#[test]
fn legacy_constructors_are_deprecated() {
    let source_path = workflow_runtime_path();
    let content =
        std::fs::read_to_string(&source_path).expect("failed to read workflow_runtime.rs");

    let lines: Vec<&str> = content.lines().collect();

    // Check that `pub fn new` has #[deprecated]
    let mut new_has_deprecated = false;
    let mut new_with_event_store_has_deprecated = false;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Look for `pub fn new(` within the next ~20 lines of a `#[deprecated]` attribute
        if line.contains("#[deprecated") {
            // Check the next 20 lines for `pub fn new` or `pub fn new_with_event_store`
            for offset in 1..=20 {
                let idx = i + offset;
                if idx < lines.len() {
                    let following = lines[idx];
                    if following.contains("pub fn new(") {
                        new_has_deprecated = true;
                    }
                    if following.contains("pub fn new_with_event_store(") {
                        new_with_event_store_has_deprecated = true;
                    }
                }
            }
        }
        i += 1;
    }

    assert!(
        new_has_deprecated,
        "pub fn new must have #[deprecated] attribute"
    );
    assert!(
        new_with_event_store_has_deprecated,
        "pub fn new_with_event_store must have #[deprecated] attribute"
    );
}

/// Scenario: engine identity path is Uuid-free
/// WHEN `crates/sddk-engine/src/workflow_runtime.rs` is greped by `Uuid::new_v4`
/// THEN 0 occurrences remain (all 6 annotated occurrences have been removed)
/// AND the test grep `engine_identity_path_is_uuid_free` passes
#[test]
fn engine_identity_path_is_uuid_free() {
    let source_path = workflow_runtime_path();
    let content =
        std::fs::read_to_string(&source_path).expect("failed to read workflow_runtime.rs");

    let lines: Vec<&str> = content.lines().collect();

    let mut uuid_v4_count = 0;
    for line in lines {
        if line.contains("Uuid::new_v4()") {
            uuid_v4_count += 1;
        }
    }

    assert_eq!(
        uuid_v4_count, 0,
        "workflow_runtime.rs must have 0 Uuid::new_v4() occurrences (found {}), \
         all must be replaced with RunId::derive(...)",
        uuid_v4_count
    );
}
