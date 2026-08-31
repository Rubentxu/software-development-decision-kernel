//! E14.5 — Pipeline stage functions.
//!
//! Individual pure functions for each pipeline stage.

use sddk_domain::UatPlan;

/// Run the quality stage: detect smells in the plan.
pub fn run_quality_stage(
    plan: &UatPlan,
    force_quality_failure: bool,
) -> crate::uat_quality::report::QualityReport {
    if force_quality_failure {
        crate::uat_quality::report::QualityReport {
            schema_version: 1,
            analyzer: "pipeline-test-injection".to_string(),
            model: "test-v1".to_string(),
            analyzed_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .expect("RFC 3339 formatting cannot fail"),
            plan_ref: String::new(),
            smells: vec![crate::uat_quality::report::QualitySmell {
                id: "TEST-001".to_string(),
                smell_id: "injected-blocker".to_string(),
                severity: "BLOCKER".to_string(),
                location: crate::uat_quality::report::SmellLocation {
                    feature_id: "F-01".to_string(),
                    scenario_id: "S-001".to_string(),
                    item_id: None,
                    field: None,
                },
                snippet: Some("test snippet".to_string()),
                suggestion: "Remove test injection".to_string(),
                auto_fixable: false,
            }],
            summary: crate::uat_quality::report::QualitySummary {
                total: 1,
                blockers: 1,
                errors: 0,
                warnings: 0,
                suggestions: 0,
                pass: false,
            },
            verdict: "NEEDS_REVISION".to_string(),
            threshold_applied: "BLOCKER".to_string(),
        }
    } else {
        crate::uat_quality::detect_13_smells(
            plan,
            crate::uat_quality::report::QualityThreshold::Blocker,
        )
    }
}

/// Run discovery and return scenario candidates.
pub fn run_discover(
    app_url: &str,
) -> Result<Vec<crate::uat_discover::AamScenarioCandidate>, String> {
    let fara_url = std::env::var("FARA_URL")
        .ok()
        .unwrap_or_else(|| "http://127.0.0.1:8082".to_string());

    let budget: u32 = std::env::var("FARA_BUDGET")
        .ok()
        .and_then(|b| b.parse().ok())
        .unwrap_or(50);

    let goals: Vec<String> = std::env::var("FARA_GOALS")
        .ok()
        .map(|g| g.split(',').map(String::from).collect())
        .unwrap_or_else(|| vec!["Explore the main functionality".to_string()]);

    let args = crate::uat::DiscoverArgs {
        app_url: app_url.to_string(),
        entry: "/".to_string(),
        goals,
        budget,
        fara_url: Some(fara_url),
        output: None,
        format: crate::OutputFormat::Text,
    };

    let outcome =
        crate::uat_discover::discover(&args).map_err(|e| format!("discovery failed: {}", e))?;

    Ok(outcome.aam.scenario_candidates)
}

/// Enrich stage: add forms and provenance to scenarios in-place.
pub fn stage_enrich(plan: &mut UatPlan) {
    for feature in &mut plan.features {
        for scenario in &mut feature.scenarios {
            crate::uat_enrich::enrich_scenario(scenario);
        }
    }
}

/// Validate stage: check plan has features/scenarios and form DSL is valid.
/// Returns total_scenarios on success.
pub fn stage_validate(plan: &UatPlan) -> Result<usize, crate::uat_generate::runner::PipelineError> {
    if plan.features.is_empty() {
        return Err(crate::uat_generate::runner::PipelineError::PlanningFailed(
            "final plan has no features".to_string(),
        ));
    }
    let total_scenarios: usize = plan.features.iter().map(|f| f.scenarios.len()).sum();
    if total_scenarios == 0 {
        return Err(crate::uat_generate::runner::PipelineError::PlanningFailed(
            "final plan has no scenarios".to_string(),
        ));
    }

    let mut dsl_errors: Vec<String> = Vec::new();
    for feature in &plan.features {
        for scenario in &feature.scenarios {
            if let Some(form) = &scenario.form {
                for error in sddk_domain::validate_form_dsl(form) {
                    dsl_errors.push(format!("{}: {}", scenario.id, error));
                }
            }
        }
    }
    if !dsl_errors.is_empty() {
        return Err(
            crate::uat_generate::runner::PipelineError::SchemaValidationFailed(format!(
                "form DSL validation failed:\n  {}",
                dsl_errors.join("\n  ")
            )),
        );
    }

    Ok(total_scenarios)
}

/// Write stage: atomically write the plan to disk.
pub fn stage_write(
    plan: &UatPlan,
    output_path: &std::path::Path,
) -> Result<(), crate::uat_generate::runner::PipelineError> {
    crate::uat_common::plan_io::atomic_write_plan(plan, output_path)
        .map_err(|e| crate::uat_generate::runner::PipelineError::IoError(e.to_string()))
}

/// Render pipeline output as string.
pub fn render_pipeline_output(
    stages: &[crate::uat_generate::runner::StageOutput],
    final_path: &std::path::Path,
) -> String {
    let mut lines = Vec::new();
    for stage in stages {
        lines.push(format!(
            "  [{}] {}: {} ({})",
            stage.stage,
            stage.tag,
            stage.message,
            stage.path.display()
        ));
    }
    lines.push(String::new());
    lines.push(format!("Pipeline complete: {}", final_path.display()));
    lines.join("\n")
}
