//! Durable cross-cycle incidence record (INC).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IncRecord {
    pub inc_id: String,
    pub finding_id: String,
    pub cycle_id: String,
    pub status: crate::IncStatus,
    pub severity: crate::Severity,
    pub priority: crate::Priority,
    pub fingerprint: String,
    pub fingerprint_aliases: Vec<String>,
    pub cluster_id: String,
    pub created_at: String,
    pub created_by: String,
    pub owner: String,
    pub inc_path: String,
    #[serde(default)]
    pub lifecycle_events: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IncStatus, Priority, Severity};

    fn sample() -> IncRecord {
        IncRecord {
            inc_id: "INC-001-3ef321c4".into(),
            finding_id: "FIND-0001".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            status: IncStatus::Open,
            severity: Severity::Medium,
            priority: Priority::P2,
            fingerprint: "3ef321c4efe1d87e".into(),
            fingerprint_aliases: vec![],
            cluster_id: "CL-01".into(),
            created_at: "2026-08-21T00:00:00Z".into(),
            created_by: "sddk".into(),
            owner: "team".into(),
            inc_path: "~/.sddk-knowledge/sddk-framework/incs/INC-001-3ef321c4.md".into(),
            lifecycle_events: vec!["created:2026-08-21T00:00:00Z".into()],
            evidence_refs: vec![],
        }
    }

    #[test]
    fn serde_bytes_unchanged() {
        let json = serde_json::to_string(&sample()).expect("serialize");
        assert!(json.contains("\"inc_id\":\"INC-001-3ef321c4\""));
        assert!(json.contains("\"finding_id\":\"FIND-0001\""));
        assert!(json.contains("\"status\":\"open\""));
        assert!(json.contains("\"severity\":\"medium\""));
        assert!(json.contains("\"priority\":\"P2\""));
        let roundtrip: IncRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtrip.inc_id, "INC-001-3ef321c4");
        assert_eq!(roundtrip.lifecycle_events.len(), 1);
    }
}
