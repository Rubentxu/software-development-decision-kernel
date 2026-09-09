//! `sddk target` — list and resolve Target/Task DAGs (M6.2).
//!
//! The command surface is intentionally narrow this cycle:
//! - `sddk target list` — enumerate all registered targets.
//! - `sddk target resolve <name>` — resolve a target to its DAG.
//!
//! Both produce JSON or Text output. The CLI delegates to the engine
//! `TargetRegistry` (which carries the built-in targets). No DAG
//! execution happens here; that lands in a later cycle.

#![allow(missing_docs)]

use clap::Subcommand;
use serde::Serialize;

use crate::{CliEnvironment, CommandOutput, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum TargetCommand {
    /// List all registered targets (built-in + user-declared).
    List {
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Resolve a target by name to its topologically-ordered DAG.
    Resolve {
        /// Target name (e.g. status, run, ship, recover).
        #[arg(long)]
        name: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
}

#[derive(Debug, Serialize)]
pub struct TargetListEntry {
    pub name: String,
    pub about: String,
    pub task_count: usize,
}

#[derive(Debug, Serialize)]
pub struct TargetListReport {
    pub kind: &'static str,
    pub count: usize,
    pub targets: Vec<TargetListEntry>,
}

/// Run the `target` subcommand dispatcher.
pub(crate) fn run_target(command: TargetCommand, environment: &CliEnvironment) -> CommandOutput {
    let _ = environment;
    let registry = sddk_engine::target_task::registry::TargetRegistry::with_builtins();
    match command {
        TargetCommand::List { format } => render_target_list(&registry, format),
        TargetCommand::Resolve { name, format } => render_target_resolve(&registry, &name, format),
    }
}

fn render_target_list(
    registry: &sddk_engine::target_task::registry::TargetRegistry,
    format: OutputFormat,
) -> CommandOutput {
    let targets: Vec<TargetListEntry> = registry
        .names()
        .map(|name| {
            let t = registry.get(name).expect("registered");
            TargetListEntry {
                name: t.name.clone(),
                about: t.about.clone(),
                task_count: t.tasks.len(),
            }
        })
        .collect();
    let count = targets.len();
    match format {
        OutputFormat::Json => {
            let report = TargetListReport {
                kind: "target.list",
                count,
                targets,
            };
            let stdout = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into());
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str("Registered targets:\n");
            for t in &targets {
                out.push_str(&format!(
                    "  {:<10} {} ({} tasks)\n",
                    t.name, t.about, t.task_count
                ));
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}

fn render_target_resolve(
    registry: &sddk_engine::target_task::registry::TargetRegistry,
    name: &str,
    format: OutputFormat,
) -> CommandOutput {
    let resolution = registry.resolve(name);
    let has_dag = resolution.dag.is_some();
    match format {
        OutputFormat::Json => {
            let stdout = serde_json::to_string_pretty(&resolution).unwrap_or_else(|_| "{}".into());
            CommandOutput {
                status: if has_dag { 0 } else { 1 },
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str(&format!("Target: {}\n", resolution.target));
            out.push_str(&format!("Validation: {}\n", resolution.validation));
            if let Some(dag) = resolution.dag {
                out.push_str(&format!("Tasks ({}):\n", dag.order.len()));
                for id in &dag.order {
                    let task = dag.by_id.get(id).expect("task in dag");
                    out.push_str(&format!(
                        "  {:<22} side_effect={:?} authority={:?}\n",
                        id, task.side_effect_class, task.authority_requirement
                    ));
                }
            }
            CommandOutput {
                status: if has_dag { 0 } else { 1 },
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> sddk_engine::target_task::registry::TargetRegistry {
        sddk_engine::target_task::registry::TargetRegistry::with_builtins()
    }

    #[test]
    fn list_reports_builtin_count() {
        let r = registry();
        let out = render_target_list(&r, OutputFormat::Json);
        assert_eq!(out.status, 0);
        // JSON report contains all four built-in names.
        for name in ["status", "run", "ship", "recover"] {
            assert!(out.stdout.contains(name), "missing `{name}`");
        }
    }

    #[test]
    fn resolve_known_target_returns_dag_in_text() {
        let r = registry();
        let out = render_target_resolve(&r, "status", OutputFormat::Text);
        assert_eq!(out.status, 0);
        assert!(out.stdout.contains("context.resolve"));
        assert!(out.stdout.contains("render"));
    }

    #[test]
    fn resolve_unknown_target_returns_error_in_text() {
        let r = registry();
        let out = render_target_resolve(&r, "nope", OutputFormat::Text);
        assert_eq!(out.status, 1);
        assert!(out.stdout.contains("unknown target"));
    }
}
