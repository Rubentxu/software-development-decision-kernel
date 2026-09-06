//! CI lint: compile path must not contain wall-clock, RNG, or HashMap types.
//!
//! Covers I-3 (pure computation, BTreeMap only) and I-4 (no SystemTime).
//!
//! This is the same discipline already used for `HashMap` prohibition
//! (`tests/hashmap_audit.rs`).

use std::path::Path;

/// Returns the path to the execution_graph_compiler source file.
fn compiler_source_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("execution_graph_compiler.rs")
}

/// Checks that `path` does NOT contain any of the forbidden time/RNG symbols.
fn check_no_forbidden_symbols(path: &Path) -> Result<(), String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

    let forbidden = [
        "std::time::",
        "SystemTime",
        "Instant",
        "time::OffsetDateTime",
        "rand::",
        "getrandom",
        "// TODO",
        "// FIXME",
        "// HACK",
    ];

    let mut failures = Vec::new();
    for (lineno, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("//!") {
            continue;
        }
        for symbol in &forbidden {
            if line.contains(symbol) {
                failures.push(format!(
                    "line {}: contains forbidden symbol '{}'",
                    lineno + 1,
                    symbol
                ));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

/// Verifies compile path has no wall-clock or RNG symbols (I-3, I-4).
#[test]
fn compile_path_has_no_wallclock() {
    let path = compiler_source_path();
    let result = check_no_forbidden_symbols(&path);
    assert!(
        result.is_ok(),
        "compile path must not use wall-clock/RNG: {}",
        result.unwrap_err()
    );
}

/// Verifies compile path uses BTreeMap (not HashMap) — covered by hashmap_audit.
#[test]
fn compile_path_uses_btreemap_only() {
    let path = compiler_source_path();
    let content = std::fs::read_to_string(&path).expect("must read source");
    // If HashMap< appears in a non-comment line, it must be in a test-only file
    // The execution_graph_compiler.rs itself must not use HashMap
    for (lineno, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("//!") {
            continue;
        }
        if line.contains("HashMap<") {
            panic!(
                "compile path at line {} contains HashMap (use BTreeMap instead): {}",
                lineno + 1,
                line
            );
        }
    }
}
