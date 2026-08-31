//! E14.5 — Approval stage for the generate pipeline.

use crate::uat_common::io::{ApprovalDecision, ApprovalIo, ApprovalVerdict, StdioApprovalIo};
use sddk_domain::{UatPlan, UatPlanApproval};

/// Run the approval stage: prompt for or skip human approval.
/// Modifies plan in-place to set approval field if approved.
pub fn stage_approval(
    plan: &mut UatPlan,
    interactive: bool,
    approval_io: Option<Box<dyn ApprovalIo>>,
) -> Result<ApprovalDecision, crate::uat_generate::runner::PipelineError> {
    if interactive {
        let mut io: Box<dyn ApprovalIo> =
            approval_io.unwrap_or_else(|| Box::new(StdioApprovalIo::default()));

        let summary = crate::uat_common::io::UatPlanSummary::from(&*plan);
        let decision = io
            .prompt(&summary)
            .map_err(|e| crate::uat_generate::runner::PipelineError::IoError(e.to_string()))?;

        match decision.verdict {
            ApprovalVerdict::Approve => {
                io.record(&decision).map_err(|e| {
                    crate::uat_generate::runner::PipelineError::IoError(e.to_string())
                })?;

                let approval = UatPlanApproval {
                    id: decision.id.clone(),
                    display: decision.display.clone(),
                    approved_at: decision.at.clone(),
                };
                plan.approval = Some(approval);
                Ok(decision)
            }
            ApprovalVerdict::Reject => {
                Err(crate::uat_generate::runner::PipelineError::ApprovalRejected)
            }
            ApprovalVerdict::Edit => {
                Err(crate::uat_generate::runner::PipelineError::ApprovalEditRequested)
            }
        }
    } else {
        // In non-interactive mode, skip approval silently.
        // Return an auto-approved decision (not recorded since there's no IO).
        Ok(ApprovalDecision::new(
            ApprovalVerdict::Approve,
            "T-auto".to_string(),
            "Auto Mode".to_string(),
        ))
    }
}
