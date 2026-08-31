//! Per-cycle debt report emitted by sddk-debt-verify.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DebtReport {
    pub schema_version: String,
    pub cycle_id: String,
    pub generated_at: String,
    pub findings: Vec<crate::Finding>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Finding, FindingStatus, Priority, Severity};

    fn sample() -> DebtReport {
        DebtReport {
            schema_version: "1.1.0".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            generated_at: "2026-08-21T00:00:00Z".into(),
            findings: vec![Finding {
                id: "FIND-0001".into(),
                title: "Test finding".into(),
                severity: Severity::Medium,
                priority: Priority::P2,
                status: FindingStatus::Open,
                fingerprint: "3ef321c4efe1d87e".into(),
                fingerprint_aliases: vec!["alias1".into()],
                cluster_id: "CL-01".into(),
                category: "architecture".into(),
                description: "Test".into(),
                remediation_cycle: Some("p-next".into()),
                remediation_pr: Some("https://github.com/org/repo/pull/123".into()),
                evidence_refs: Some(vec![serde_json::json!({"kind": "commit", "ref": "abc123"})]),
            }],
        }
    }

    #[test]
    fn serde_bytes_unchanged() {
        let json = serde_json::to_string(&sample()).expect("serialize");
        assert!(json.contains("\"schema_version\":\"1.1.0\""));
        assert!(json.contains("\"id\":\"FIND-0001\""));
        assert!(json.contains("\"severity\":\"medium\""));
        assert!(json.contains("\"priority\":\"P2\""));
        assert!(json.contains("\"status\":\"open\""));
        let roundtrip: DebtReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtrip.findings.len(), 1);
        assert_eq!(roundtrip.findings[0].id, "FIND-0001");
    }
}
