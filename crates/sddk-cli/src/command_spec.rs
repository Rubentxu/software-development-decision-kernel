//! Typed `CommandSpec` metadata for CLI introspection (M6.1).
//!
//! Agent consumers can enumerate commands via `sddk introspect commands --format json`
//! or get a single command's spec via `sddk --command-spec <name>`.
//!
//! The spec is hand-curated from the `Command` enum in `lib.rs` to ensure
//! machine-readable consistency without runtime clap introspection.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at module scope.
//!
#![allow(missing_docs)]

use clap::Subcommand;
use serde::Serialize;

use crate::{CliEnvironment, CommandOutput, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum IntrospectCommand {
    /// List all available commands and their typed specs.
    Commands {
        /// Restrict to a single command name. If absent, lists all.
        #[arg(long)]
        command: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    Flag,
    Option,
    Positional,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    String,
    Int,
    Bool,
    Path,
    Enum(Vec<String>),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ArgSpec {
    pub name: String,
    pub kind: ArgKind,
    pub value_type: ValueType,
    pub required: bool,
    pub default: Option<String>,
    pub help: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormatKind {
    Text,
    Json,
    Both,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommandSpec {
    pub name: String,
    pub about: String,
    pub output_format: OutputFormatKind,
    pub args: Vec<ArgSpec>,
    pub has_subcommands: bool,
    pub cycle_phase: Option<String>,
    pub spec_ref: Option<String>,
}

/// Return the canonical CommandSpec list for the entire CLI surface.
pub fn all_command_specs() -> Vec<CommandSpec> {
    vec![
        spec(
            "version",
            "Show the resolved framework version",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "project",
            "Resolve deterministic project and workspace identity",
            OutputFormatKind::Both,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "adopt",
            "Plan, apply, inspect, or repair project adoption",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "lint",
            "Validate repository contracts and generated workflow documentation",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "generate",
            "Generate deterministic repository documentation",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "cycle",
            "Plan and apply workflow cycles under the local authority",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-005"),
        ),
        spec(
            "ledger",
            "Verify the causal ledger and list its events",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-008"),
        ),
        spec(
            "capability",
            "Plan and execute typed capabilities under the default-deny policy",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-008"),
        ),
        spec(
            "git",
            "Run typed local Git operations with verified postconditions",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "artifact",
            "Store and verify content-addressed artifacts",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "permission",
            "Check agent phase and capability permissions",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "validate",
            "Validate structured results against canonical schemas",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "release",
            "Plan, apply, and ship local Git releases",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "vault",
            "Index, validate, search, and export knowledge vaults",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "knowledge",
            "Resolve the canonical knowledge vault path and profile",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "dev",
            "Developer tooling: doctor, gates, and atomic install/verify",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "pack",
            "Validate declarative pack manifests",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "graph",
            "Query, inspect, and rebuild the reactive knowledge graph",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "metrics",
            "Record, aggregate, and tune cycle telemetry metrics",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "analytics",
            "Report, trend, and bottleneck analytics from cycle metrics",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "telemetry",
            "Central telemetry control plane",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "uat",
            "User Acceptance Testing: data-driven YAML plans rendered to dashboards",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "approval",
            "Manage human approval decisions for governed capabilities",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            Some("arch-spec-008"),
        ),
        spec(
            "rules",
            "Architecture-rule registry: evaluate rules against baseline JSON (SDDK2-003)",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "stale",
            "Staleness and impact queries over the reactive graph (SPEC-012)",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "fork",
            "Fork, replay, diff and promote controlled experiments (SPEC-009)",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "memory",
            "Decision memory: status, log, tree, show, diff, merge-base, reflog, audit (SPEC-004)",
            OutputFormatKind::Both,
            vec![],
            true,
            None,
            Some("arch-spec-004"),
        ),
        spec(
            "explore",
            "Render task-specific views over the reactive graph (SPEC-013)",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            Some("arch-spec-013"),
        ),
        spec(
            "completion",
            "Generate or install shell completion scripts",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "debt",
            "Debt management: report generation, INC listing, INC backfill, gate evaluation",
            OutputFormatKind::Text,
            vec![],
            false,
            None,
            None,
        ),
        spec(
            "status",
            "Show the current cycle snapshot and lease",
            OutputFormatKind::Both,
            vec![arg(
                "cycle",
                ArgKind::Option,
                ValueType::String,
                false,
                None,
                "Cycle identifier",
            )],
            false,
            Some("status"),
            None,
        ),
        spec(
            "plan",
            "Plan and apply workflow cycles under the local authority",
            OutputFormatKind::Both,
            vec![arg(
                "workitem",
                ArgKind::Option,
                ValueType::String,
                false,
                None,
                "DEPRECATED: use sddk plan workitem",
            )],
            true,
            Some("plan"),
            None,
        ),
        spec(
            "run",
            "Execute a capability",
            OutputFormatKind::Both,
            vec![],
            false,
            Some("run"),
            Some("arch-spec-008"),
        ),
        spec(
            "ship",
            "Plan a release (dry-run)",
            OutputFormatKind::Both,
            vec![],
            false,
            Some("ship"),
            None,
        ),
        spec(
            "recover",
            "Rebuild a cycle from ledger events (dry-run)",
            OutputFormatKind::Both,
            vec![arg(
                "dry_run",
                ArgKind::Flag,
                ValueType::Bool,
                false,
                Some("false".into()),
                "Dry-run",
            )],
            false,
            Some("recover"),
            None,
        ),
        spec(
            "change",
            "Create a planning work item in a cycle (M6.1)",
            OutputFormatKind::Both,
            vec![
                arg(
                    "cycle_id",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Cycle identifier",
                ),
                arg(
                    "title",
                    ArgKind::Option,
                    ValueType::String,
                    true,
                    None,
                    "Work item title",
                ),
                arg(
                    "description",
                    ArgKind::Option,
                    ValueType::String,
                    true,
                    None,
                    "Work item description",
                ),
                arg(
                    "actor",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Actor id",
                ),
            ],
            false,
            Some("plan"),
            Some("arch-spec-006"),
        ),
        spec(
            "verify",
            "Verify ledger continuity and capability policy snapshot (M6.1)",
            OutputFormatKind::Both,
            vec![],
            false,
            Some("verify"),
            Some("arch-spec-008"),
        ),
        spec(
            "audit",
            "Audit memory reflog plus ledger events (M6.1)",
            OutputFormatKind::Both,
            vec![
                arg(
                    "since",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Optional RFC3339 filter (currently a no-op)",
                ),
                arg(
                    "format",
                    ArgKind::Option,
                    ValueType::Enum(vec!["text".into(), "json".into()]),
                    false,
                    Some("text".into()),
                    "Output format",
                ),
            ],
            false,
            Some("audit"),
            Some("arch-spec-004"),
        ),
        spec(
            "config",
            "Inspect configuration precedence chain (M6.1)",
            OutputFormatKind::Both,
            vec![arg(
                "format",
                ArgKind::Option,
                ValueType::Enum(vec!["text".into(), "json".into()]),
                false,
                Some("text".into()),
                "Output format",
            )],
            true,
            None,
            None,
        ),
        spec(
            "docs",
            "Generate rendered documentation from a chapter contract",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "inventory",
            "Query, inspect, and rebuild the reactive knowledge graph",
            OutputFormatKind::Text,
            vec![],
            true,
            None,
            None,
        ),
        spec(
            "introspect",
            "Enumerate commands and their typed specs (M6.1)",
            OutputFormatKind::Json,
            vec![
                arg(
                    "command",
                    ArgKind::Option,
                    ValueType::String,
                    false,
                    None,
                    "Single command name; if absent, lists all",
                ),
                arg(
                    "format",
                    ArgKind::Option,
                    ValueType::Enum(vec!["text".into(), "json".into()]),
                    false,
                    Some("json".into()),
                    "Output format",
                ),
            ],
            true,
            None,
            None,
        ),
    ]
}

fn spec(
    name: &str,
    about: &str,
    output_format: OutputFormatKind,
    args: Vec<ArgSpec>,
    has_subcommands: bool,
    cycle_phase: Option<&str>,
    spec_ref: Option<&str>,
) -> CommandSpec {
    CommandSpec {
        name: name.to_string(),
        about: about.to_string(),
        output_format,
        args,
        has_subcommands,
        cycle_phase: cycle_phase.map(String::from),
        spec_ref: spec_ref.map(String::from),
    }
}

fn arg(
    name: &str,
    kind: ArgKind,
    value_type: ValueType,
    required: bool,
    default: Option<String>,
    help: &str,
) -> ArgSpec {
    ArgSpec {
        name: name.to_string(),
        kind,
        value_type,
        required,
        default,
        help: help.to_string(),
    }
}

/// Render the full command spec list (or a single spec) as a `CommandOutput`.
pub fn render_command_spec(
    name: Option<&str>,
    format: OutputFormat,
    _environment: &CliEnvironment,
) -> CommandOutput {
    let specs = all_command_specs();
    let payload = match name {
        Some(n) => {
            let found: Vec<&CommandSpec> = specs.iter().filter(|s| s.name == n).collect();
            if found.is_empty() {
                serde_json::json!({
                    "kind": "command_spec",
                    "error": format!("unknown command: {n}"),
                    "available": specs.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
                })
            } else {
                serde_json::json!({
                    "kind": "command_spec",
                    "spec": found[0],
                })
            }
        }
        None => serde_json::json!({
            "kind": "command_specs",
            "specs": specs,
        }),
    };
    match format {
        OutputFormat::Json => {
            let stdout = serde_json::to_string_pretty(&payload).unwrap_or_default();
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let stdout = match name {
                Some(n) => {
                    let mut out = format!("command spec for {n}:\n");
                    if let Some(s) = specs.iter().find(|s| s.name == n) {
                        out.push_str(&format!("  about: {}\n", s.about));
                        out.push_str(&format!("  output_format: {:?}\n", s.output_format));
                        out.push_str(&format!("  has_subcommands: {}\n", s.has_subcommands));
                        if let Some(p) = &s.cycle_phase {
                            out.push_str(&format!("  cycle_phase: {p}\n"));
                        }
                    } else {
                        out.push_str(&format!("  (unknown command: {n})\n"));
                    }
                    out
                }
                None => {
                    let mut out = String::from("available commands:\n");
                    for s in &specs {
                        out.push_str(&format!("  {:<14} {}\n", s.name, s.about));
                    }
                    out
                }
            };
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
    }
}

/// Run the `introspect` subcommand dispatcher.
pub(crate) fn run_introspect(
    command: IntrospectCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        IntrospectCommand::Commands { command, format } => {
            render_command_spec(command.as_deref(), format, environment)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_command_specs_includes_change_verify_audit() {
        let names: Vec<String> = all_command_specs().into_iter().map(|s| s.name).collect();
        assert!(names.iter().any(|n| n == "change"));
        assert!(names.iter().any(|n| n == "verify"));
        assert!(names.iter().any(|n| n == "audit"));
        assert!(names.iter().any(|n| n == "config"));
        assert!(names.iter().any(|n| n == "introspect"));
    }

    #[test]
    fn render_all_in_json_is_valid() {
        let env = CliEnvironment::default();
        let out = render_command_spec(None, OutputFormat::Json, &env);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert_eq!(parsed["kind"], "command_specs");
        let arr = parsed["specs"].as_array().unwrap();
        assert!(arr.len() >= 40);
    }

    #[test]
    fn render_single_known_command() {
        let env = CliEnvironment::default();
        let out = render_command_spec(Some("memory"), OutputFormat::Json, &env);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert_eq!(parsed["kind"], "command_spec");
        assert_eq!(parsed["spec"]["name"], "memory");
        assert_eq!(parsed["spec"]["spec_ref"], "arch-spec-004");
    }

    #[test]
    fn render_single_unknown_command_reports_available() {
        let env = CliEnvironment::default();
        let out = render_command_spec(Some("does_not_exist"), OutputFormat::Json, &env);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert!(parsed["error"].as_str().unwrap().contains("does_not_exist"));
        let available = parsed["available"].as_array().unwrap();
        assert!(available.len() >= 40);
    }

    #[test]
    fn change_spec_marks_required_args() {
        let s = all_command_specs()
            .into_iter()
            .find(|s| s.name == "change")
            .expect("change spec");
        let required: Vec<&str> = s
            .args
            .iter()
            .filter(|a| a.required)
            .map(|a| a.name.as_str())
            .collect();
        assert!(required.contains(&"title"));
        assert!(required.contains(&"description"));
    }
}
