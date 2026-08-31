//! AAM (ActualApplicationModel) domain structs — tolerant deserialization.
//!
//! All structs use `#[serde(default)]` and `serde_json::Value` where action/result
//! vary to handle diverse computer_use.mjs output formats.

use serde::{Deserialize, Serialize};

/// Root AAM model — parsed from computer_use.mjs artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamModel {
    pub schema_version: u32,
    pub model: String,
    pub generated_by: String,
    pub generated_at: String,
    pub app: AamApp,
    #[serde(default)]
    pub pages: Vec<AamPage>,
    #[serde(default)]
    pub flows: Vec<AamFlow>,
    #[serde(default)]
    pub scenario_candidates: Vec<AamScenarioCandidate>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub provenance: AamProvenance,
}

impl AamModel {
    /// Creates a fallback AAM when Fara is unreachable.
    /// Sets fara_version=unreachable and provenance.fallback=no-fara.
    pub fn fallback(app_url: &str, entry: &str) -> Self {
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .expect("RFC 3339 formatting cannot fail");
        Self {
            schema_version: 1,
            model: "uat-discovery".into(),
            generated_by: "uat-discovery".into(),
            generated_at: now.clone(),
            app: AamApp {
                name: "Discovered App".into(),
                version: "unknown".into(),
                base_url: app_url.into(),
                explored_at: now.clone(),
                exploration_budget: 0,
                fara_version: "unreachable".into(),
                fara_url: "http://127.0.0.1:8082".into(),
            },
            pages: Vec::new(),
            flows: Vec::new(),
            scenario_candidates: Vec::new(),
            screenshots: Vec::new(),
            urls: vec![entry.into()],
            provenance: AamProvenance {
                generated_by: Some("uat-discovery".into()),
                author: None,
                created_at: Some(now),
                last_modified_at: None,
                origin: Some("discovered".into()),
                origin_ref: None,
                modified_by: None,
                linked_defect: None,
                repro_command: None,
                tags: vec!["discovered".into(), "fallback".into()],
                confidence: None,
                human_reviewed: false,
                fallback: Some("no-fara".into()),
            },
        }
    }
}

/// Application metadata in the AAM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamApp {
    pub name: String,
    pub version: String,
    pub base_url: String,
    pub explored_at: String,
    pub exploration_budget: u32,
    /// "reachable" or "unreachable"
    pub fara_version: String,
    pub fara_url: String,
}

/// A discovered page/screen in the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamPage {
    pub id: String,
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub semantic: String,
    #[serde(default)]
    pub url_snapshot: String,
    #[serde(default)]
    pub elements: Vec<AamElement>,
}

/// A UI element discovered on a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamElement {
    #[serde(default)]
    pub selector: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,
}

/// A flow through the application (sequence of pages with actions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamFlow {
    pub id: String,
    #[serde(default)]
    pub semantic: String,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub steps: Vec<AamFlowStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trajectory_hash: Option<String>,
}

/// One step in a flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamFlowStep {
    #[serde(default)]
    pub page: String,
    #[serde(default)]
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
}

/// A candidate scenario derived from a discovered flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamScenarioCandidate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow_ref: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default)]
    pub plain_steps: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_duration_minutes: Option<u32>,
    #[serde(default)]
    pub evidence: AamEvidence,
    #[serde(default)]
    pub provenance: AamProvenance,
}

impl AamScenarioCandidate {
    /// Creates a scenario candidate from a flow.
    pub fn from_flow(flow: &AamFlow, index: usize, _app_url: &str) -> Self {
        let plain_steps: Vec<String> = flow
            .steps
            .iter()
            .map(|s| {
                if let Some(ref sel) = s.selector {
                    format!("{} on {}", s.action, sel)
                } else {
                    s.action.clone()
                }
            })
            .collect();

        Self {
            flow_ref: Some(flow.id.clone()),
            title: if flow.semantic.is_empty() {
                format!("Discovered Flow {}", index + 1)
            } else {
                flow.semantic.clone()
            },
            priority: Some("P2".into()),
            plain_steps,
            estimated_duration_minutes: Some((flow.steps.len() as u32) * 2),
            evidence: AamEvidence {
                kinds: vec!["screenshot".into(), "trajectory".into()],
            },
            provenance: AamProvenance {
                generated_by: Some("uat-discovery".into()),
                author: None,
                created_at: Some(
                    time::OffsetDateTime::now_utc()
                        .format(&time::format_description::well_known::Rfc3339)
                        .expect("RFC 3339 formatting cannot fail"),
                ),
                last_modified_at: None,
                origin: Some("discovered".into()),
                origin_ref: None,
                modified_by: None,
                linked_defect: None,
                repro_command: None,
                tags: vec!["discovered".into()],
                confidence: Some(0.7),
                human_reviewed: false,
                fallback: None,
            },
        }
    }
}

/// Evidence spec for a scenario candidate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamEvidence {
    #[serde(default)]
    pub kinds: Vec<String>,
}

/// Provenance metadata for AAM elements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AamProvenance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked_defect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repro_command: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub human_reviewed: bool,
    /// Set to "no-fara" when Fara was unreachable
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
}
