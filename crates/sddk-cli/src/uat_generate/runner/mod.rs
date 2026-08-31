//! E14.5 — Pipeline runner for the generate command.
//!
//! Stages: optional discover → plan → enrich → quality → interactive approval → validate.
//! Propagates status/error non-zero on any failure (no continue after failure).
//!
//! Atomic write: plan is ONLY written after ALL stages pass.
//! No intermediate files are written. On error, no output file exists.

pub mod approval;
pub mod stages;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use crate::uat_common::io::ApprovalIo;

pub use approval::stage_approval;
pub use stages::{
    render_pipeline_output, run_discover, run_quality_stage, stage_enrich, stage_validate,
    stage_write,
};

/// Pipeline errors.
#[derive(Debug)]
pub enum PipelineError {
    ValidationFailed(String),
    PlanningFailed(String),
    QualityFailed(String),
    SchemaValidationFailed(String),
    ApprovalRejected,
    ApprovalEditRequested,
    DiscoveryFailed(String),
    IoError(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::ValidationFailed(msg) => write!(f, "validation failed: {}", msg),
            PipelineError::PlanningFailed(msg) => write!(f, "planning failed: {}", msg),
            PipelineError::QualityFailed(msg) => write!(f, "quality gate failed: {}", msg),
            PipelineError::SchemaValidationFailed(msg) => {
                write!(f, "schema validation failed: {}", msg)
            }
            PipelineError::ApprovalRejected => write!(f, "approval rejected"),
            PipelineError::ApprovalEditRequested => {
                write!(f, "approval edit requested (not supported)")
            }
            PipelineError::DiscoveryFailed(msg) => write!(f, "discovery failed: {}", msg),
            PipelineError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

/// Stage output with path and status.
#[derive(Debug, Clone)]
pub struct StageOutput {
    pub stage: &'static str,
    pub path: PathBuf,
    pub tag: String,
    pub message: String,
}

/// Pipeline configuration.
pub struct PipelineConfig {
    pub release: String,
    pub requirements: Option<PathBuf>,
    pub changelog: Option<PathBuf>,
    pub last_plan: Option<PathBuf>,
    pub discover: bool,
    pub app_url: Option<String>,
    pub interactive: bool,
    pub output: Option<PathBuf>,
    pub approval_io: Option<Box<dyn ApprovalIo>>,
    pub force_quality_failure: bool,
}

impl PipelineConfig {
    /// Default output path for the plan.
    pub fn output_path(&self) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("uat-plan-{}.yaml", self.release)))
    }
}

/// Run the full generate pipeline.
/// Returns StageOutput for each completed stage on success,
/// or PipelineError on any failure (no partial output).
///
/// Atomicity: NO file writes until after ALL gates pass.
/// On any error, no output file exists.
pub fn run_pipeline(config: PipelineConfig) -> Result<Vec<StageOutput>, PipelineError> {
    let mut stages = Vec::new();

    let release = config.release.clone();
    let requirements = config.requirements.clone();
    let changelog = config.changelog.clone();
    let last_plan = config.last_plan.clone();
    let discover = config.discover;
    let app_url = config.app_url.clone();
    let interactive = config.interactive;
    let output_path = config.output_path();
    let approval_io = config.approval_io;

    // ── Stage 0: Validate inputs ──────────────────────────────────────────────
    super::validator::validate_inputs(&requirements, &changelog, &last_plan, discover, &app_url)
        .map_err(|e| PipelineError::ValidationFailed(format!("{:?}", e)))?;

    // ── Stage 1: Optional Discover ────────────────────────────────────────────
    let aam_scenario_candidates = if discover {
        let url = app_url.as_ref().unwrap();
        match run_discover(url) {
            Ok(candidates) => {
                stages.push(StageOutput {
                    stage: "discover",
                    path: PathBuf::from("N/A"),
                    tag: "discovered".to_string(),

                    message: format!("discover: {} scenario candidates", candidates.len()),
                });
                candidates
            }
            Err(e) => return Err(PipelineError::DiscoveryFailed(e)),
        }
    } else {
        stages.push(StageOutput {
            stage: "discover",
            path: PathBuf::from("N/A"),
            tag: "skipped".to_string(),
            message: "discover: skipped (no --discover)".to_string(),
        });
        Vec::new()
    };

    // ── Stage 2: Plan ────────────────────────────────────────────────────────
    let plan_output: super::planner::PlanOutput = super::planner::build_plan(
        &release,
        &requirements,
        &changelog,
        &last_plan,
        &aam_scenario_candidates,
    )
    .map_err(|e| PipelineError::PlanningFailed(format!("{:?}", e)))?;

    stages.push(StageOutput {
        stage: "plan",
        path: PathBuf::from("N/A"),
        tag: "planned".to_string(),
        message: format!(
            "plan: {} features, {} scenarios",
            plan_output.plan.features.len(),
            plan_output
                .plan
                .features
                .iter()
                .map(|f| f.scenarios.len())
                .sum::<usize>()
        ),
    });

    // ── Stage 3: Enrich ──────────────────────────────────────────────────────
    let mut enriched_plan = plan_output.plan;
    stage_enrich(&mut enriched_plan);

    stages.push(StageOutput {
        stage: "enrich",
        path: PathBuf::from("N/A"),
        tag: "enriched".to_string(),
        message: "enrich: forms + provenance set".to_string(),
    });

    // ── Stage 4: Quality ─────────────────────────────────────────────────────
    let quality_report = run_quality_stage(&enriched_plan, config.force_quality_failure);

    if !quality_report.summary.pass {
        return Err(PipelineError::QualityFailed(format!(
            "{} blockers found",
            quality_report.summary.blockers
        )));
    }

    stages.push(StageOutput {
        stage: "quality",
        path: PathBuf::from("N/A"),
        tag: "quality_pass".to_string(),
        message: format!(
            "quality: {} smells ({} blockers, {} warnings) — PASS",
            quality_report.summary.total,
            quality_report.summary.blockers,
            quality_report.summary.warnings
        ),
    });

    // ── Stage 5: Approval ───────────────────────────────────────────────────
    let _decision = stage_approval(&mut enriched_plan, interactive, approval_io)?;

    if interactive {
        stages.push(StageOutput {
            stage: "approval",
            path: PathBuf::from("N/A"),
            tag: "approved".to_string(),
            message: "approval: approved".to_string(),
        });
    } else {
        stages.push(StageOutput {
            stage: "approval",
            path: PathBuf::from("N/A"),
            tag: "auto_skip".to_string(),
            message: "approval: auto mode — no human approval recorded".to_string(),
        });
    }

    // ── Stage 6: Validate ────────────────────────────────────────────────────
    let total_scenarios = stage_validate(&enriched_plan)?;

    stages.push(StageOutput {
        stage: "validate",
        path: PathBuf::from("N/A"),
        tag: "validated".to_string(),
        message: format!(
            "validate: {} features, {} scenarios — OK",
            enriched_plan.features.len(),
            total_scenarios
        ),
    });

    // ── ATOMIC WRITE ─────────────────────────────────────────────────────────
    stage_write(&enriched_plan, &output_path)?;

    stages.push(StageOutput {
        stage: "write",
        path: output_path.clone(),
        tag: "written".to_string(),
        message: format!("written: {}", output_path.display()),
    });

    Ok(stages)
}
