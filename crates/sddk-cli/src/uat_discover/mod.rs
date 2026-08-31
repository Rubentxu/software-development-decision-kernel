//! E14.4 — Test Discovery Agent: parse Fara computer-use outputs into AAM.
//!
//! Parses `computer_use.mjs` outputs: `trajectory.json`, `summary.json`, and
//! screenshots into a non-empty `ActualApplicationModel` (AAM) YAML.

pub(crate) mod aam;
mod parser;
pub mod run;
#[cfg(test)]
mod tests;

pub use aam::{AamFlow, AamModel, AamPage, AamScenarioCandidate};

/// Outcome of a discovery run: AAM model plus optional warning message.
/// Warning is present when Fara is unreachable but we still produce a fallback AAM.
#[derive(Debug, Clone)]
pub struct DiscoveryOutcome {
    pub aam: AamModel,
    pub warning: Option<String>,
}

/// Render discovery output to stdout and write AAM YAML to disk.
/// Returns the stdout message (with optional WARN line) on success.
pub fn render_discovery_output(
    outcome: &DiscoveryOutcome,
    goals: &[String],
    output_path: &std::path::Path,
) -> anyhow::Result<String> {
    let yaml = serde_saphyr::to_string(&outcome.aam)
        .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
    std::fs::write(output_path, &yaml).map_err(|e| anyhow::anyhow!("write failed: {e}"))?;

    let pages = outcome.aam.pages.len();
    let flows = outcome.aam.flows.len();
    let scenarios = outcome.aam.scenario_candidates.len();
    let screenshots = outcome.aam.screenshots.len();
    let urls = outcome.aam.urls.len();

    let mut msg = String::new();
    if let Some(ref warn) = outcome.warning {
        msg.push_str(&format!("WARN: {}\n", warn));
    }
    msg.push_str(&format!(
        "uat discover: {} goals, {} pages, {} flows, {} scenarios, {} screenshots, {} urls\n  AAM: {}",
        goals.len(),
        pages,
        flows,
        scenarios,
        screenshots,
        urls,
        output_path.display()
    ));
    Ok(msg)
}

/// Build arguments for the `computer_use.mjs` node script.
/// Returns args in order: [--url, app_url, --goal, goal, --output, output, --fara-url, fara_url, --max-steps, budget].
pub fn computer_use_command_args(
    app_url: &str,
    goal: &str,
    output: &str,
    fara_url: &str,
    budget: u32,
) -> Vec<String> {
    vec![
        "--url".into(),
        app_url.into(),
        "--goal".into(),
        goal.into(),
        "--output".into(),
        output.into(),
        "--fara-url".into(),
        fara_url.into(),
        "--max-steps".into(),
        budget.to_string(),
    ]
}

/// Validate that a ParseResult has non-empty pages or flows.
/// Returns Ok if the artifact is usable, Err with the specified message otherwise.
pub fn validate_successful_artifacts(result: &parser::ParseResult) -> anyhow::Result<()> {
    if result.pages.is_empty() && result.flows.is_empty() {
        anyhow::bail!("empty AAM after successful run");
    }
    Ok(())
}

/// Merge multiple ParseResult into one combined result.
pub fn merge_successful_runs(results: Vec<parser::ParseResult>) -> parser::ParseResult {
    let mut pages: Vec<AamPage> = Vec::new();
    let mut flows: Vec<AamFlow> = Vec::new();
    let mut screenshots: Vec<String> = Vec::new();
    let mut urls: Vec<String> = Vec::new();

    for result in results {
        pages.extend(result.pages);
        flows.extend(result.flows);
        screenshots.extend(result.screenshots);
        urls.extend(result.urls);
    }

    parser::ParseResult {
        pages,
        flows,
        screenshots,
        urls,
    }
}

/// Parse a Fara run directory into an AAM model.
/// Returns non-empty pages/flows/scenario_candidates if successful artifacts exist.
/// On Fara unreachable, returns a fallback AAM with a warning (no error).
pub fn discover(args: &crate::uat::DiscoverArgs) -> anyhow::Result<DiscoveryOutcome> {
    let fara_url = args
        .fara_url
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:8082".into());

    // Create temp dir for all goal runs
    let temp_base = std::env::temp_dir().join(format!("uat-discovery-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_base).map_err(|e| anyhow::anyhow!("mkdir temp: {e}"))?;

    let mut fara_reachable = false;
    let mut app_reachable = false;
    let mut warning: Option<String> = None;

    // Check Fara health
    match ureq::get(&format!("{}/health", fara_url)).call() {
        Ok(r) if r.status() == 200 => fara_reachable = true,
        _ => {}
    }

    if !fara_reachable {
        // Fara unreachable → explicit fallback AAM with fara_version=unreachable
        warning = Some(format!("Fara not reachable at {}", fara_url));
        return Ok(DiscoveryOutcome {
            aam: AamModel::fallback(&args.app_url, &args.entry),
            warning,
        });
    }

    // Check app health
    match ureq::get(&args.app_url).call() {
        Ok(r) if r.status() == 200 => app_reachable = true,
        _ => {}
    }

    if !app_reachable {
        anyhow::bail!("App unreachable at {}", args.app_url);
    }

    // Run Fara for each goal and collect results
    let mut all_results: Vec<parser::ParseResult> = Vec::new();

    for (i, goal) in args.goals.iter().enumerate() {
        let run_dir = temp_base.join(format!("run-{:03}", i));
        std::fs::create_dir_all(&run_dir).map_err(|e| anyhow::anyhow!("mkdir run dir: {e}"))?;

        let cmd_args = computer_use_command_args(
            &args.app_url,
            goal,
            run_dir.to_str().unwrap(),
            &fara_url,
            args.budget,
        );

        let status = std::process::Command::new("node")
            .arg("assets/uat-driver/computer_use.mjs")
            .args(&cmd_args)
            .current_dir(std::env::current_dir().unwrap())
            .status()
            .map_err(|e| anyhow::anyhow!("computer_use.mjs: {e}"))?;

        if !status.success() {
            anyhow::bail!("computer_use.mjs exited with {}", status);
        }

        // Parse this run's artifacts
        let result = parser::parse_run_dir(&run_dir, goal, i)?;
        all_results.push(result);
    }

    // Merge results
    let merged = merge_successful_runs(all_results);

    // Validate non-empty artifacts
    validate_successful_artifacts(&merged)?;

    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC 3339 formatting cannot fail");
    let scenario_candidates = if !merged.flows.is_empty() {
        merged
            .flows
            .iter()
            .enumerate()
            .map(|(i, flow)| AamScenarioCandidate::from_flow(flow, i, &args.app_url))
            .collect()
    } else {
        Vec::new()
    };

    Ok(DiscoveryOutcome {
        aam: AamModel {
            schema_version: 1,
            model: "uat-discovery".into(),
            generated_by: "uat-discovery".into(),
            generated_at: now.clone(),
            app: aam::AamApp {
                name: "Discovered App".into(),
                version: "unknown".into(),
                base_url: args.app_url.clone(),
                explored_at: now.clone(),
                exploration_budget: args.budget,
                fara_version: if fara_reachable {
                    "reachable".into()
                } else {
                    "unreachable".into()
                },
                fara_url: fara_url.clone(),
            },
            pages: merged.pages,
            flows: merged.flows,
            scenario_candidates,
            screenshots: merged.screenshots,
            urls: merged.urls,
            provenance: aam::AamProvenance {
                generated_by: Some("uat-discovery".into()),
                author: None,
                created_at: Some(now.clone()),
                last_modified_at: None,
                origin: Some("discovered".into()),
                origin_ref: None,
                modified_by: None,
                linked_defect: None,
                repro_command: None,
                tags: vec!["discovered".into()],
                confidence: None,
                human_reviewed: false,
                fallback: None,
            },
        },
        warning,
    })
}
