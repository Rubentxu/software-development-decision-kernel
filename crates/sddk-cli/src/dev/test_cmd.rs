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

/// A parsed cargo test message from --message-format=json.
#[derive(Debug, serde::Deserialize)]
struct CargoMessage {
    reason: Option<String>,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    #[serde(default)]
    target: CargoTarget,
    result: Option<CargoTestResult>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct CargoTarget {
    name: Option<String>,
    src_path: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CargoTestResult {
    ok: bool,
    #[serde(default)]
    n: usize,
}

/// Run `cargo test --workspace --message-format=json --no-run` and parse the output
/// to count total passing tests and distinct test binaries.
fn count_workspace_tests(root: &PathBuf) -> Result<CountWorkspaceOutput, String> {
    let output = Command::new("cargo")
        .args(["test", "--workspace", "--message-format=json", "--no-run"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("failed to spawn cargo test: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "cargo test --no-run failed with exit {}",
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut total_workspace_tests: usize = 0;
    let mut test_binaries = std::collections::HashSet::new();

    for line in stdout.lines() {
        // Parse JSON messages from cargo test --message-format=json
        if let Ok(msg) = serde_json::from_str::<CargoMessage>(line) {
            // Only process test result messages
            let reason = msg.reason.or(msg.msg_type);
            if reason.as_deref() != Some("test") {
                continue;
            }

            if let Some(result) = msg.result {
                total_workspace_tests += result.n;
                if let Some(name) = msg.target.name {
                    test_binaries.insert(name);
                }
            }
        }
    }

    Ok(CountWorkspaceOutput {
        total_workspace_tests,
        test_binaries: test_binaries.len(),
    })
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
