//! `dev test` — test-related developer tooling.
//!
//! Exit codes:
//! - 0: success
//! - 1: test failure detected
//! - 2: invalid invocation (outside project root, etc.)

use crate::{CliEnvironment, CommandOutput, OutputFormat};
use std::path::PathBuf;
use std::process::Command;

/// Arguments for `dev test`.
#[derive(Debug, Clone, clap::Args)]
pub(super) struct TestArgs {
    #[command(subcommand)]
    pub(super) command: TestCommand,
}

/// Test subcommands.
#[derive(Debug, Clone, clap::Subcommand)]
pub(super) enum TestCommand {
    /// Count total workspace tests and distinct test binaries.
    CountWorkspace(CountWorkspaceArgs),
}

/// Arguments for `count-workspace`.
#[derive(Debug, Clone, clap::Args)]
pub(super) struct CountWorkspaceArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

/// Result of counting workspace tests.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct CountWorkspaceOutput {
    pub total_workspace_tests: usize,
    pub test_binaries: usize,
}

/// Detect if the current directory is inside an SDDK project root.
/// Walks up looking for `.sddk/` or `Cargo.toml` with workspace metadata.
fn find_project_root() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    for ancestor in current.ancestors() {
        if ancestor.join(".sddk").is_dir() {
            return Some(ancestor.to_path_buf());
        }
        let cargo_toml = ancestor.join("Cargo.toml");
        if cargo_toml.is_file() {
            // Quick check: does it contain [workspace]?
            if let Ok(content) = std::fs::read_to_string(&cargo_toml)
                && content.contains("[workspace]")
            {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

/// Parse cargo's combined stdout+stderr text output into workspace test counts.
///
/// Pure function — no I/O, no cargo invocation. Trivial to unit test. The
/// recursion-safe design lets integration tests exercise CLI mechanics
/// (help, outside-root, json flag) without re-entering `cargo test --workspace`.
///
/// Parses two kinds of lines:
/// - `test result: ok. <N> passed` — sum the captured `N` for `total_workspace_tests`.
/// - `Running <path> (target triple)` — count distinct paths for `test_binaries`.
fn parse_cargo_test_output(combined: &str) -> CountWorkspaceOutput {
    let mut total_workspace_tests: usize = 0;
    let mut test_binaries = std::collections::HashSet::new();

    // Parse "test result: ok. N passed" lines to sum total passing tests
    let result_re =
        regex::Regex::new(r"^test result:\s+ok\.\s+(\d+)\s+passed").expect("regex must be valid");

    // Parse "Running <path>" lines to count distinct test binaries.
    // Format: "     Running tests/dev_count_workspace.rs (.../dev_count_workspace-f0fde7b50d49f2b7)"
    let running_re =
        regex::Regex::new(r"^Running\s+(.+?)(?:\s+\([^)]+\))?$").expect("regex must be valid");

    for line in combined.lines() {
        if let Some(caps) = result_re.captures(line)
            && let Ok(n) = caps.get(1).unwrap().as_str().parse::<usize>()
        {
            total_workspace_tests += n;
        }
        let trimmed = line.trim_start();
        if let Some(caps) = running_re.captures(trimmed)
            && let Some(path) = caps.get(1)
        {
            test_binaries.insert(path.as_str().to_string());
        }
    }

    CountWorkspaceOutput {
        total_workspace_tests,
        test_binaries: test_binaries.len(),
    }
}

/// Run `cargo test --workspace` and parse the human-readable text output
/// to count total passing tests and distinct test binaries.
///
/// **WARNING**: This invokes `cargo test --workspace`, which re-runs the
/// test binary. Do NOT call this from inside an integration test that
/// invokes the binary in turn (infinite recursion). The CLI integration
/// tests in `tests/dev_count_workspace.rs` only exercise CLI mechanics
/// (help/format/outside-root) — parser logic is unit-tested in this file.
fn count_workspace_tests(root: &PathBuf) -> Result<CountWorkspaceOutput, String> {
    let output = Command::new("cargo")
        .args(["test", "--workspace"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to spawn cargo test: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    Ok(parse_cargo_test_output(&combined))
}

/// Dispatch `dev test count-workspace`.
fn run_count_workspace(args: CountWorkspaceArgs, root: Option<PathBuf>) -> CommandOutput {
    let root = root.or_else(find_project_root);

    let Some(root) = root else {
        return CommandOutput {
            status: 2,
            stdout: String::new(),
            stderr: "error: must run inside an SDDK project root (.sddk/ or Cargo.toml with workspace metadata)\n".to_string(),
        };
    };

    match count_workspace_tests(&root) {
        Ok(counts) => {
            let stdout = match args.format {
                OutputFormat::Text => format!(
                    "total_workspace_tests: {}\ntest_binaries: {}\n",
                    counts.total_workspace_tests, counts.test_binaries
                ),
                OutputFormat::Json => {
                    format!("{}\n", serde_json::to_string(&counts).unwrap_or_default())
                }
            };
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        Err(message) => CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("error: {message}\n"),
        },
    }
}

/// Main dispatcher for `dev test`.
pub(super) fn run_test(args: TestArgs, _environment: &CliEnvironment) -> CommandOutput {
    match args.command {
        TestCommand::CountWorkspace(count_args) => run_count_workspace(count_args, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit tests for the pure parser. Recursion-safe (no cargo invocation).
    /// Integration tests in `tests/dev_count_workspace.rs` only exercise CLI
    /// mechanics (help/format/outside-root) — they do not call
    /// `sddk dev test count-workspace` from inside the workspace because that
    /// would re-enter `cargo test --workspace` and recurse infinitely.

    #[test]
    fn parse_single_binary_summary() {
        let output = "\
            Running unittests (target/debug/deps/foo-abc123)\n\
            test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured\n\
        ";
        let counts = parse_cargo_test_output(output);
        assert_eq!(counts.total_workspace_tests, 5);
        assert_eq!(counts.test_binaries, 1);
    }

    #[test]
    fn parse_multiple_binaries_sum_totals() {
        let output = "\
            Running unittests (target/debug/deps/foo-abc123)\n\
            test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured\n\
            Running tests/integration.rs (target/debug/deps/integration-def456)\n\
            test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured\n\
            Running tests/dev_count_workspace.rs (target/debug/deps/dev_count_workspace-ghi789)\n\
            test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured\n\
        ";
        let counts = parse_cargo_test_output(output);
        assert_eq!(counts.total_workspace_tests, 19);
        assert_eq!(counts.test_binaries, 3);
    }

    #[test]
    fn parse_distinct_binaries_dedup() {
        let output = "\
            Running unittests (target/debug/deps/foo-abc123)\n\
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured\n\
            Running unittests (target/debug/deps/foo-abc123)\n\
            test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured\n        ";
        let counts = parse_cargo_test_output(output);
        assert_eq!(counts.total_workspace_tests, 3);
        assert_eq!(counts.test_binaries, 1);
    }

    #[test]
    fn parse_empty_output_returns_zero() {
        let counts = parse_cargo_test_output("");
        assert_eq!(counts.total_workspace_tests, 0);
        assert_eq!(counts.test_binaries, 0);
    }

    #[test]
    fn parse_no_running_lines_keeps_totals_only() {
        let output = "\
            warning: unused variable `x`\n\
            test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured\n\
        ";
        let counts = parse_cargo_test_output(output);
        assert_eq!(counts.total_workspace_tests, 7);
        assert_eq!(counts.test_binaries, 0);
    }

    #[test]
    fn parse_failed_lines_excluded_from_totals() {
        let output = "\
            Running tests/foo.rs (target/debug/deps/foo-abc)\n\
            test result: FAILED. 5 passed; 2 failed; 0 ignored\n\
            Running tests/bar.rs (target/debug/deps/bar-def)\n\
            test result: ok. 8 passed; 0 failed; 0 ignored\n\
        ";
        let counts = parse_cargo_test_output(output);
        // FAILED line excluded (regex requires `ok.`); only the ok line counted.
        assert_eq!(counts.total_workspace_tests, 8);
        assert_eq!(counts.test_binaries, 2);
    }

    #[test]
    fn parse_handles_leading_whitespace_on_running_lines() {
        let output = "\
               Running unittests (target/debug/deps/foo-abc)\n\
            test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured\n\
        ";
        let counts = parse_cargo_test_output(output);
        assert_eq!(counts.total_workspace_tests, 4);
        assert_eq!(counts.test_binaries, 1);
    }

    #[test]
    fn parse_ground_truth_1739_lines() {
        // Synthetic sample mirroring cargo's actual output format.
        let mut output = String::new();
        let total_binaries = 97;
        let mut total_tests = 0usize;
        for i in 0..total_binaries {
            output.push_str(&format!(
                "     Running tests/bin_{i:03}.rs (target/debug/deps/bin_{i:03}-hash)\n"
            ));
            let n = (i * 17 + 3) % 30 + 1;
            total_tests += n;
            output.push_str(&format!(
                "test result: ok. {n} passed; 0 failed; 0 ignored; 0 measured\n"
            ));
        }
        let counts = parse_cargo_test_output(&output);
        assert_eq!(counts.total_workspace_tests, total_tests);
        assert_eq!(counts.test_binaries, total_binaries);
    }
}
