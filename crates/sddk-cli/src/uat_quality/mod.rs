//! E14.2 — Form Quality Agent: detects 13 test-smell categories.
//!
//! Canonical smell catalog: `agents/uat-form-quality.md` (13 smell IDs).
//! Design: ADR-019 / `design.md §Decision: 13 deterministic smells`.

mod detector;
pub mod report;
#[cfg(test)]
mod tests;

pub use crate::uat::QualityArgs;
pub use detector::detect_13_smells;
#[allow(dead_code)]
pub use report::QualityReport;

/// Run quality detection against a plan.
#[allow(dead_code)]
pub fn run(args: &QualityArgs) -> anyhow::Result<QualityReport> {
    let plan = crate::uat_common::read_plan(&args.plan)?;
    let report = detect_13_smells(&plan, args.threshold);
    Ok(report)
}
