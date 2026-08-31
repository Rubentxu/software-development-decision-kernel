//! Moldable explorer CLI (SPEC-013, Phase 8).
//!
//! Renders task-specific views (graph, timeline, verification, ...) as
//! self-contained HTML from the reactive graph — same entity, many views.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_domain::{find_builtin_view, render_view_model};
use serde::Serialize;

use crate::cycle::RuntimeArgs;
use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};

/// Embedded explorer template (fallback when the assets bundle is absent,
/// e.g. tests or a bare binary).
const EMBEDDED_TEMPLATE: &str = include_str!("../../../assets/explorer/template.html");

#[derive(Debug, Subcommand)]
pub(crate) enum ExploreCommand {
    /// Render a task-specific view as self-contained HTML.
    Render(ExploreRenderArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ExploreRenderArgs {
    /// View id: overview | graph | timeline | verification | evidence | release.
    #[arg(long)]
    pub(crate) view: String,
    /// Focus entity (`kind:id`).
    #[arg(long)]
    pub(crate) entity: Option<String>,
    /// Output file (default: stdout).
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_explore(command: ExploreCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        ExploreCommand::Render(args) => run_explore_render(args, environment),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ExploreOutput {
    view: String,
    entity: Option<String>,
    out: String,
    html_bytes: usize,
}

fn run_explore_render(args: ExploreRenderArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ExploreOutput> {
        let descriptor = find_builtin_view(&args.view)
            .ok_or_else(|| anyhow::anyhow!("unknown view '{}' (try: overview, graph, timeline, verification, evidence, release)", args.view))?;

        let state = crate::stale_cmd::load_graph_state(&args.runtime, environment)?;
        let model = render_view_model(&state, &descriptor, args.entity.as_deref());

        // Prefer the installed assets template; fall back to the embedded one
        // (tests, bare binary, or missing bundle).
        let assets = crate::dev::paths::resolve_assets_dir(environment)
            .ok()
            .flatten();
        let template = match assets {
            Some(dir) => {
                let template_path = dir.join("explorer").join("template.html");
                std::fs::read_to_string(&template_path)
                    .unwrap_or_else(|_| EMBEDDED_TEMPLATE.to_string())
            }
            None => EMBEDDED_TEMPLATE.to_string(),
        };

        let model_json = serde_json::to_string(&model)?;
        let html = template
            .replace("{{TITLE}}", &descriptor.title)
            .replace("{{VIEWMODEL_JSON}}", &model_json);

        let out = args.out.clone().unwrap_or_else(|| PathBuf::from("-"));
        if out.as_os_str() == "-" {
            print!("{html}");
        } else {
            std::fs::write(&out, &html)
                .map_err(|e| anyhow::anyhow!("write {}: {e}", out.display()))?;
        }

        Ok(ExploreOutput {
            view: args.view.clone(),
            entity: args.entity.clone(),
            out: out.display().to_string(),
            html_bytes: html.len(),
        })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, explore_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

fn explore_text(output: &ExploreOutput) -> String {
    format!(
        "view: {}\nentity: {}\nout: {}\nhtml_bytes: {}\n",
        output.view,
        output.entity.as_deref().unwrap_or("-"),
        output.out,
        output.html_bytes
    )
}
