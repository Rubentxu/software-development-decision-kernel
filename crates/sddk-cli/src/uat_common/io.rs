//! Injectable Approval IO — enables testing the generate pipeline's approval gate.

#![allow(dead_code)]

/// Verdict of the human approval gate.
#[derive(Debug, Clone)]
pub enum ApprovalVerdict {
    Approve,
    Reject,
    Edit,
}

/// Recorded approval decision.
#[derive(Debug, Clone)]
pub struct ApprovalDecision {
    pub verdict: ApprovalVerdict,
    /// Internal ID of the approver (e.g. "T-0001").
    pub id: String,
    /// Display name of the approver.
    pub display: String,
    /// RFC3339 timestamp of the decision.
    pub at: String,
}

impl ApprovalDecision {
    /// Create an approval decision.
    pub fn new(verdict: ApprovalVerdict, id: String, display: String) -> Self {
        Self {
            verdict,
            id,
            display,
            at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .expect("RFC 3339 formatting cannot fail"),
        }
    }
}

/// Minimal summary of a draft plan for the approval prompt.
#[derive(Debug, Clone)]
pub struct UatPlanSummary {
    pub release: String,
    pub feature_count: usize,
    pub scenario_count: usize,
    pub output_path: String,
}

impl From<&sddk_domain::UatPlan> for UatPlanSummary {
    fn from(plan: &sddk_domain::UatPlan) -> Self {
        let scenario_count = plan.features.iter().map(|f| f.scenarios.len()).sum();
        Self {
            release: plan.release.candidate.clone(),
            feature_count: plan.features.len(),
            scenario_count,
            output_path: String::new(),
        }
    }
}

/// Trait for injectable approval IO.
/// Production: reads from stdin; Tests: scripted decisions.
pub trait ApprovalIo: Send {
    /// Prompt the human for a decision on the draft plan.
    fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision>;

    /// Record the decision in the plan provenance.
    fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()>;
}

/// CI/default: auto-approve.
#[derive(Debug, Clone)]
pub struct AutoApproveIo {
    pub by: String,
}

impl AutoApproveIo {
    pub fn new(by: impl Into<String>) -> Self {
        Self { by: by.into() }
    }
}

impl ApprovalIo for AutoApproveIo {
    fn prompt(&mut self, _draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
        Ok(ApprovalDecision::new(
            ApprovalVerdict::Approve,
            "ci-bot".into(),
            self.by.clone(),
        ))
    }

    fn record(&mut self, _decision: &ApprovalDecision) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Interactive: reads from stdin.
#[derive(Debug)]
pub struct StdioApprovalIo {
    pub reader: std::sync::Mutex<std::io::Stdin>,
}

impl Default for StdioApprovalIo {
    fn default() -> Self {
        Self {
            reader: std::sync::Mutex::new(std::io::stdin()),
        }
    }
}

impl ApprovalIo for StdioApprovalIo {
    fn prompt(&mut self, draft: &UatPlanSummary) -> anyhow::Result<ApprovalDecision> {
        println!("═══════════════════════════════════════════════════════════");
        println!("  UAT Pipeline — Approval Gate");
        println!("═══════════════════════════════════════════════════════════");
        println!("  Release: {}", draft.release);
        println!(
            "  Features: {}  |  Scenarios: {}",
            draft.feature_count, draft.scenario_count
        );
        println!("───────────────────────────────────────────────────────────");
        println!("  [A]probar   [R]echazar   [E]ditar");
        println!("═══════════════════════════════════════════════════════════");
        print!("  Your decision [A/r/e]: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut line = String::new();
        self.reader
            .lock()
            .map_err(|_| anyhow::anyhow!("stdin lock poisoned"))?
            .read_line(&mut line)?;

        let line = line.trim().to_lowercase();
        let (verdict, id, display) = match line.as_str() {
            "a" | "" => (
                ApprovalVerdict::Approve,
                "T-human".to_string(),
                "Human Reviewer".to_string(),
            ),
            "r" => (
                ApprovalVerdict::Reject,
                "T-human".to_string(),
                "Human Reviewer".to_string(),
            ),
            "e" => (
                ApprovalVerdict::Edit,
                "T-human".to_string(),
                "Human Reviewer".to_string(),
            ),
            other => anyhow::bail!("invalid choice: {other}"),
        };

        Ok(ApprovalDecision::new(verdict, id, display))
    }

    fn record(&mut self, decision: &ApprovalDecision) -> anyhow::Result<()> {
        println!(
            "  Approved by: {} ({}) at {}",
            decision.display, decision.id, decision.at
        );
        Ok(())
    }
}
