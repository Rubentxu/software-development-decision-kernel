//! Quality report types for the 13-smell detector.

use serde::Serialize;

/// Verdict threshold for quality gate.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum QualityThreshold {
    Blocker,
    Warning,
}

/// Quality report produced by the 13-smell detector.
#[derive(Debug, Serialize)]
pub struct QualityReport {
    pub schema_version: u32,
    pub analyzer: String,
    pub model: String,
    pub analyzed_at: String,
    pub plan_ref: String,
    pub smells: Vec<QualitySmell>,
    pub summary: QualitySummary,
    pub verdict: String,
    pub threshold_applied: String,
}

/// A single smell detected in a form item or scenario.
#[derive(Debug, Serialize)]
pub struct QualitySmell {
    pub id: String,
    pub smell_id: String,
    pub severity: String,
    pub location: SmellLocation,
    pub snippet: Option<String>,
    pub suggestion: String,
    pub auto_fixable: bool,
}

/// Location of a detected smell.
#[derive(Debug, Serialize)]
pub struct SmellLocation {
    pub feature_id: String,
    pub scenario_id: String,
    pub item_id: Option<String>,
    pub field: Option<String>,
}

/// Summary counts for a quality report.
#[derive(Debug, Serialize)]
pub struct QualitySummary {
    pub total: u32,
    pub blockers: u32,
    pub errors: u32,
    pub warnings: u32,
    pub suggestions: u32,
    pub pass: bool,
}
