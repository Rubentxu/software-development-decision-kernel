//! `sddk dev reconcile` — Authoritative IDE reconciliation.
//!
//! Reconciles drift between `assets/agent-models.yaml` + `agents/*.md`
//! and the per-IDE configs (opencode, zcode, claude, codex).
//!
//! Dry-run by default; `--apply` mutations are atomic.

// Re-export reconcile types for use by the command and tests
pub use crate::dev::editor_adapters::reconcile::{
    FieldDiff, ReconcileAdapter, ReconcileContext, ReconcileReport,
};

use crate::dev::agent_models::AgentModelsConfig;
use crate::dev::editor_adapters::reconcile::reconcilers_for;
use crate::dev::editor_adapters::{EditorDirs, load_agent_sources, renames_builder};
use crate::dev::paths::resolve_active_framework_root;
use crate::{CliEnvironment, CommandOutput, OutputFormat};
use clap::Args;
use std::path::PathBuf;

/// Reconciliation arguments.
#[derive(Debug, Clone, Args)]
pub struct ReconcileArgs {
    /// Framework root (default: active bundle).
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Target editor(s).
    #[arg(long, value_enum, default_value_t = crate::dev::LinkEditor::All)]
    pub editor: crate::dev::LinkEditor,

    /// Actually mutate editor configs (dry-run is default).
    #[arg(long, short)]
    pub apply: bool,

    /// Check mode: exit 1 if drift detected, 0 if clean.
    #[arg(long)]
    pub check: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Override the OpenCode config dir.
    #[arg(long)]
    pub opencode_dir: Option<PathBuf>,

    /// Override the ZCode dir.
    #[arg(long)]
    pub zcode_dir: Option<PathBuf>,

    /// Override the Claude Code dir.
    #[arg(long)]
    pub claude_dir: Option<PathBuf>,

    /// Override the Codex dir.
    #[arg(long)]
    pub codex_dir: Option<PathBuf>,
}

/// Run `sddk dev reconcile`.
pub fn run_dev_reconcile(args: ReconcileArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;

    let result = (|| -> anyhow::Result<Vec<ReconcileReport>> {
        // Resolve root
        let root = if let Some(ref r) = args.root {
            std::fs::canonicalize(r)?
        } else {
            resolve_active_framework_root(environment)?
        };

        // Load agent sources from bundle
        let agents = load_agent_sources(&root);

        // Load agent-models.yaml (absence is not an error)
        let models_path = root.join("assets").join("agent-models.yaml");
        let models =
            AgentModelsConfig::from_file(&models_path).map_err(|e| anyhow::anyhow!("{}", e))?;

        // Build editor dirs
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        let opencode_dir = args
            .opencode_dir
            .unwrap_or_else(|| home.join(".config/opencode"));
        let zcode_dir = args.zcode_dir.unwrap_or_else(|| home.join(".zcode"));
        let claude_dir = args.claude_dir.unwrap_or_else(|| home.join(".claude"));
        let codex_dir = args.codex_dir.unwrap_or_else(|| home.join(".codex"));

        let dirs = EditorDirs {
            opencode: opencode_dir,
            zcode: zcode_dir,
            claude: claude_dir,
            codex: codex_dir,
        };

        // Build reconciliation context
        let renames = renames_builder(&agents);
        let ctx = ReconcileContext {
            root: &root,
            agents: &agents,
            models: models.as_ref(),
            renames: &renames,
        };

        // Run reconciliation for each selected editor
        let mut reports = Vec::new();
        for adapter in reconcilers_for(args.editor, &dirs) {
            let report = adapter.reconcile(&ctx, args.apply);
            reports.push(report);
        }

        Ok(reports)
    })();

    // Render output
    let has_drift = result
        .as_ref()
        .map(|r| {
            r.iter()
                .any(|rep| rep.agents_changed > 0 || rep.agents_pruned > 0)
        })
        .unwrap_or(false);

    let mut output = render_reconcile_result(&result, format);

    // Exit codes per spec: --check controls exit code, not --apply
    if args.check {
        output.status = if has_drift { 1 } else { 0 };
    } else {
        // --apply or dry-run: exit 0 on success, 1 on errors
        let has_errors = result
            .as_ref()
            .map(|r| r.iter().any(|rep| !rep.errors.is_empty()))
            .unwrap_or(true);
        output.status = if has_errors { 1 } else { 0 };
    }

    output
}

/// JSON output wrapper for reconcile results (BUG-F003 fix).
/// Adds top-level summary fields required by REQ-11.
#[derive(serde::Serialize)]
struct ReconcileOutputJson {
    schema_version: u32,
    cycle: String,
    editors: Vec<ReconcileReport>,
    added: usize,
    reconciled: usize,
    unchanged: usize,
    pruned: usize,
    skipped: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diffs: Vec<FieldDiff>,
    errors: Vec<String>,
}

fn render_reconcile_result(
    result: &anyhow::Result<Vec<ReconcileReport>>,
    format: OutputFormat,
) -> CommandOutput {
    match format {
        OutputFormat::Json => {
            let json = match result {
                Ok(reports) => {
                    // Aggregate summary counts across all editor reports
                    let total_added: usize = reports
                        .iter()
                        .map(|r| {
                            r.agents_total.saturating_sub(
                                r.agents_changed + r.agents_pruned + r.agents_skipped,
                            )
                        })
                        .sum();
                    let total_reconciled: usize = reports.iter().map(|r| r.agents_changed).sum();
                    let total_unchanged: usize = reports
                        .iter()
                        .map(|r| r.agents_total.saturating_sub(r.agents_changed))
                        .sum();
                    let total_pruned: usize = reports.iter().map(|r| r.agents_pruned).sum();
                    let total_skipped: usize = reports.iter().map(|r| r.agents_skipped).sum();
                    let all_errors: Vec<String> =
                        reports.iter().flat_map(|r| r.errors.clone()).collect();

                    let output = ReconcileOutputJson {
                        schema_version: 1,
                        cycle:
                            "p-52b95ef55999f9de/kernel-cycle-29-cli-authoritative-reconciliation"
                                .to_owned(),
                        editors: reports.clone(),
                        added: total_added,
                        reconciled: total_reconciled,
                        unchanged: total_unchanged,
                        pruned: total_pruned,
                        skipped: total_skipped,
                        diffs: Vec::new(), // ReconcileReport does not store per-agent diffs
                        errors: all_errors,
                    };
                    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "[]".to_owned())
                }
                Err(e) => serde_json::to_string_pretty(&serde_json::json!({
                    "error": e.to_string()
                }))
                .unwrap_or_else(|_| "{}".to_owned()),
            };
            CommandOutput {
                status: 0,
                stdout: json,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut text = String::new();
            match result {
                Ok(reports) => {
                    for report in reports {
                        text.push_str(&render_report_text(report));
                    }
                }
                Err(e) => {
                    text.push_str(&format!("error: {}\n", e));
                }
            }
            CommandOutput {
                status: 0,
                stdout: text,
                stderr: String::new(),
            }
        }
    }
}

fn render_report_text(report: &ReconcileReport) -> String {
    let mut text = String::new();
    text.push_str(&format!("editor: {}\n", report.editor));
    text.push_str(&format!(
        "  agents: {} total, {} changed, {} pruned, {} skipped\n",
        report.agents_total, report.agents_changed, report.agents_pruned, report.agents_skipped
    ));
    for error in &report.errors {
        text.push_str(&format!("  error: {}\n", error));
    }
    text
}

#[cfg(test)]
#[path = "tests/reconcile_tests.rs"]
mod reconcile_tests;
