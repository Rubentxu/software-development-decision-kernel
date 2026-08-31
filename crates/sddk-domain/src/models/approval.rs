//! Approval decision records.
use serde::{Deserialize, Serialize};

/// Outcome of a human approval decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Granted,
    Denied,
}

/// Input data for recording an approval decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApprovalReceiptInput {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: String,
    pub capability: String,
    pub request_hash: String,
    pub decision: ApprovalDecision,
    pub actor: String,
    pub reason: String,
    pub requested_at: String,
    pub decided_at: String,
}

/// A persisted human approval receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApprovalReceipt {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: String,
    pub capability: String,
    pub request_hash: String,
    pub decision: ApprovalDecision,
    pub actor: String,
    pub reason: String,
    pub requested_at: String,
    pub decided_at: String,
    pub requested_event_id: String,
    pub decision_event_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_decision_granted_roundtrip() {
        let decision = ApprovalDecision::Granted;
        let json = serde_json::to_string(&decision).unwrap();
        assert_eq!(json, "\"granted\"");
        let roundtrip: ApprovalDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, ApprovalDecision::Granted);
    }

    #[test]
    fn test_approval_decision_denied_roundtrip() {
        let decision = ApprovalDecision::Denied;
        let json = serde_json::to_string(&decision).unwrap();
        assert_eq!(json, "\"denied\"");
        let roundtrip: ApprovalDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, ApprovalDecision::Denied);
    }

    #[test]
    fn test_approval_receipt_input_roundtrip() {
        let input = ApprovalReceiptInput {
            receipt_id: "ar-1".into(),
            project_id: "p-1".into(),
            cycle_id: "c-1".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:abcd1234".into(),
            decision: ApprovalDecision::Granted,
            actor: "alice".into(),
            reason: "ok, reversible via reflog".into(),
            requested_at: "2026-08-18T10:00:00Z".into(),
            decided_at: "2026-08-18T10:05:00Z".into(),
        };
        let json = serde_json::to_string(&input).unwrap();
        let roundtrip: ApprovalReceiptInput = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, input);
    }

    #[test]
    fn test_approval_receipt_roundtrip() {
        let receipt = ApprovalReceipt {
            receipt_id: "ar-1".into(),
            project_id: "p-1".into(),
            cycle_id: "c-1".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:abcd1234".into(),
            decision: ApprovalDecision::Granted,
            actor: "alice".into(),
            reason: "ok, reversible via reflog".into(),
            requested_at: "2026-08-18T10:00:00Z".into(),
            decided_at: "2026-08-18T10:05:00Z".into(),
            requested_event_id: "approval-cap-git-delete_branch-abcd1234-requested".into(),
            decision_event_id: "approval-cap-git-delete_branch-abcd1234-granted".into(),
        };
        let json = serde_json::to_string(&receipt).unwrap();
        let roundtrip: ApprovalReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, receipt);
    }
}
