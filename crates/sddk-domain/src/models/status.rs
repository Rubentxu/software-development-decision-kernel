//! Lifecycle status of a debt finding and INC records.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingStatus {
    Open,
    InProgress,
    Deferred,
    Resolved,
    Superseded,
}

impl FindingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingStatus::Open => "open",
            FindingStatus::InProgress => "in-progress",
            FindingStatus::Deferred => "deferred",
            FindingStatus::Resolved => "resolved",
            FindingStatus::Superseded => "superseded",
        }
    }
}

/// Status of an INC record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IncStatus {
    Open,
    AcceptedRisk,
    Resolved,
}

impl IncStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncStatus::Open => "open",
            IncStatus::AcceptedRisk => "accepted-risk",
            IncStatus::Resolved => "resolved",
        }
    }
}
