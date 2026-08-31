//! Staleness and impact queries over the reactive graph (SPEC-012, Phase 6).

use clap::{Args, Subcommand};
use sddk_domain::{GraphState, GraphStore, StalenessResult, all_staleness};
use serde::Serialize;

use crate::cycle::RuntimeArgs;
use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};

#[derive(Debug, Subcommand)]
pub(crate) enum StaleCommand {
    /// List entities that are not fresh, with causal paths.
    List(StaleListArgs),
    /// Show the impact scope of an entity (BFS over outgoing relations).
    Impact(StaleImpactArgs),
    /// Evaluate the release staleness gate (fail-closed for critical entities).
    Gate(StaleGateArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct StaleGateArgs {
    /// Comma-separated entity keys to treat as critical acceptance/evidence
    /// (e.g. `acceptance:login-flow,evidence:uat-report`).
    #[arg(long)]
    pub(crate) critical: String,
    /// Severity for non-critical stale entities: `fail` or `warn`.
    #[arg(long, default_value = "warn")]
    pub(crate) advisory: String,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct StaleListArgs {
    /// Only show this entity (`kind:id`).
    #[arg(long)]
    pub(crate) entity: Option<String>,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct StaleImpactArgs {
    /// Entity to start from (`kind:id`).
    #[arg(long)]
    pub(crate) entity: String,
    /// Maximum BFS depth.
    #[arg(long, default_value_t = 3)]
    pub(crate) max_depth: u32,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_stale(command: StaleCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        StaleCommand::List(args) => run_stale_list(args, environment),
        StaleCommand::Impact(args) => run_stale_impact(args, environment),
        StaleCommand::Gate(args) => run_stale_gate(args, environment),
    }
}

/// Serializes a staleness state in snake_case (matches the serde representation).
pub(crate) fn serde_state_name(state: sddk_domain::StalenessState) -> String {
    match state {
        sddk_domain::StalenessState::Fresh => "fresh".into(),
        sddk_domain::StalenessState::PossiblyStale => "possibly_stale".into(),
        sddk_domain::StalenessState::Stale => "stale".into(),
        sddk_domain::StalenessState::Invalidated => "invalidated".into(),
        sddk_domain::StalenessState::Unknown => "unknown".into(),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct StaleGateOutput {
    passed: bool,
    critical_failures: Vec<StaleEntityOutput>,
    advisory_stale: Vec<StaleEntityOutput>,
}

fn run_stale_gate(args: StaleGateArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<StaleGateOutput> {
        let (state, _stream) = load_graph(&args.runtime, environment)?;
        let critical: Vec<String> = args
            .critical
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let advisory_fail = args.advisory == "fail";

        let mut critical_failures = Vec::new();
        let mut advisory_stale = Vec::new();
        for entity in &critical {
            let result = sddk_domain::derive_staleness(&state, entity);
            let state_str = serde_state_name(result.state);
            let output = StaleEntityOutput {
                entity: entity.clone(),
                state: state_str.clone(),
                causal_path: result.causal_path,
            };
            // Fail-closed: critical entities that are stale/invalidated fail.
            if state_str == "stale" || state_str == "invalidated" {
                critical_failures.push(output);
            } else if state_str == "possibly_stale" {
                advisory_stale.push(output);
            }
        }
        // Advisory severity: possibly_stale non-critical also fails when policy says so.
        if advisory_fail {
            for (entity, result) in all_staleness(&state) {
                let state_str = serde_state_name(result.state);
                if state_str == "possibly_stale" && !critical.contains(&entity) {
                    advisory_stale.push(StaleEntityOutput {
                        entity,
                        state: state_str,
                        causal_path: result.causal_path,
                    });
                }
            }
        }
        Ok(StaleGateOutput {
            passed: critical_failures.is_empty() && (!advisory_fail || advisory_stale.is_empty()),
            critical_failures,
            advisory_stale,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, stale_gate_text);
            if !output.passed {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

fn stale_gate_text(output: &StaleGateOutput) -> String {
    let mut text = format!("passed: {}\n", output.passed);
    if !output.critical_failures.is_empty() {
        text.push_str("critical failures:\n");
        for entity in &output.critical_failures {
            text.push_str(&format!(
                "  {} ({}) path={}\n",
                entity.entity,
                entity.state,
                entity.causal_path.join(",")
            ));
        }
    }
    if !output.advisory_stale.is_empty() {
        text.push_str("advisory stale:\n");
        for entity in &output.advisory_stale {
            text.push_str(&format!("  {} ({})\n", entity.entity, entity.state));
        }
    }
    text
}

fn load_graph(
    args: &RuntimeArgs,
    environment: &CliEnvironment,
) -> anyhow::Result<(GraphState, String)> {
    let context = crate::cycle::RuntimeContext::open(args, environment, false)?;
    let ledger_dir = context
        .paths
        .ledger
        .parent()
        .ok_or_else(|| anyhow::anyhow!("ledger path has no parent"))?
        .to_path_buf();
    let stream = format!("project:{}", context.identity.project_id);
    let mut store = sddk_storage::graph_store::SqliteGraphStore::open(&ledger_dir)?;
    let state = match store.load_state()? {
        Some(state) => state,
        None => store.rebuild_from_ledger(&stream)?,
    };
    Ok((state, stream))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct StaleListOutput {
    entities: Vec<StaleEntityOutput>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct StaleEntityOutput {
    entity: String,
    state: String,
    causal_path: Vec<String>,
}

fn run_stale_list(args: StaleListArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<StaleListOutput> {
        let (state, _stream) = load_graph(&args.runtime, environment)?;
        let mut entities: Vec<StaleEntityOutput> = match &args.entity {
            Some(entity) => {
                let result = sddk_domain::derive_staleness(&state, entity);
                vec![StaleEntityOutput {
                    entity: entity.clone(),
                    state: serde_state_name(result.state),
                    causal_path: result.causal_path,
                }]
            }
            None => all_staleness(&state)
                .into_iter()
                .map(|(entity, result)| StaleEntityOutput {
                    entity,
                    state: serde_state_name(result.state),
                    causal_path: result.causal_path,
                })
                .collect(),
        };
        entities.sort_by(|a, b| a.entity.cmp(&b.entity));
        Ok(StaleListOutput { entities })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, stale_list_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct StaleImpactOutput {
    entity: String,
    reachable: Vec<String>,
    depth: u32,
}

fn run_stale_impact(args: StaleImpactArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<StaleImpactOutput> {
        let (state, _stream) = load_graph(&args.runtime, environment)?;
        // BFS over ALL outgoing relations from the entity.
        let mut reachable = Vec::new();
        let mut frontier = vec![args.entity.clone()];
        let mut seen = std::collections::BTreeSet::new();
        seen.insert(args.entity.clone());
        for _depth in 0..args.max_depth {
            let mut next = Vec::new();
            for current in &frontier {
                for edge in state.edges.iter().filter(|e| e.from == *current) {
                    if seen.insert(edge.to.clone()) {
                        reachable.push(edge.to.clone());
                        next.push(edge.to.clone());
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        reachable.sort();
        Ok(StaleImpactOutput {
            entity: args.entity.clone(),
            reachable,
            depth: args.max_depth,
        })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, stale_impact_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

fn stale_list_text(output: &StaleListOutput) -> String {
    if output.entities.is_empty() {
        return "no entities with verification provenance\n".to_string();
    }
    let mut text = String::from("entity\tstate\tcausal_path\n");
    for entity in &output.entities {
        let path = if entity.causal_path.is_empty() {
            "-".to_string()
        } else {
            entity.causal_path.join(",")
        };
        text.push_str(&format!("{}\t{}\t{}\n", entity.entity, entity.state, path));
    }
    text
}

fn stale_impact_text(output: &StaleImpactOutput) -> String {
    if output.reachable.is_empty() {
        return format!("no reachable nodes from {}\n", output.entity);
    }
    let mut text = format!("impact of {} (depth {}):\n", output.entity, output.depth);
    for node in &output.reachable {
        text.push_str(&format!("  {node}\n"));
    }
    text
}

// Re-export for graph_cmd's why-stale.
pub(crate) fn derive_for_entity(state: &GraphState, entity: &str) -> StalenessResult {
    sddk_domain::derive_staleness(state, entity)
}

pub(crate) fn load_graph_state(
    args: &RuntimeArgs,
    environment: &CliEnvironment,
) -> anyhow::Result<GraphState> {
    load_graph(args, environment).map(|(state, _)| state)
}
