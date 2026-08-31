//! A single debt finding within a [`DebtReport`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub severity: crate::Severity,
    pub priority: crate::Priority,
    pub status: crate::FindingStatus,
    pub fingerprint: String,
    #[serde(default)]
    pub fingerprint_aliases: Vec<String>,
    pub cluster_id: String,
    pub category: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation_cycle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation_pr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Finding {
        Finding {
            id: "FIND-0001".into(),
            title: "Sample finding".into(),
            severity: crate::Severity::High,
            priority: crate::Priority::P1,
            status: crate::FindingStatus::Open,
            fingerprint: "a1b2c3d4e5f6a7b8".into(),
            fingerprint_aliases: vec![],
            cluster_id: "test-cluster".into(),
            category: "test".into(),
            description: "sample finding".into(),
            remediation_cycle: None,
            remediation_pr: None,
            evidence_refs: None,
        }
    }

    #[test]
    fn serde_bytes_unchanged() {
        let json = serde_json::to_string(&sample()).expect("serialize");
        let pinned = "{\"id\":\"FIND-0001\",\"title\":\"Sample finding\",\"severity\":\"high\",\"priority\":\"P1\",\"status\":\"open\",\"fingerprint\":\"a1b2c3d4e5f6a7b8\",\"fingerprint_aliases\":[],\"cluster_id\":\"test-cluster\",\"category\":\"test\",\"description\":\"sample finding\"}";
        assert_eq!(json, pinned, "Finding serde bytes drifted");
    }
}
