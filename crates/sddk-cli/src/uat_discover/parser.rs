//! Parser for computer_use.mjs outputs: trajectory.json, summary.json, screenshots.
//!
//! Uses tolerant deserialization with `#[serde(default)]` and `serde_json::Value`
//! to handle varying action/result formats.

use std::collections::HashSet;
use std::path::Path;

use crate::uat_discover::aam::{AamFlow, AamFlowStep, AamPage};
use serde::Deserialize;

/// Result of parsing a run directory.
#[derive(Debug, Default)]
pub struct ParseResult {
    pub pages: Vec<AamPage>,
    pub flows: Vec<AamFlow>,
    pub screenshots: Vec<String>,
    pub urls: Vec<String>,
}

/// A trajectory entry from computer_use.mjs trajectory.json.
/// Tolerant: uses serde_json::Value for action/result which vary in format.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryEntry {
    /// Step number
    #[serde(default)]
    pub step: u32,
    /// Screenshot filename (e.g., "screenshot-01.png" or "screenshot.png")
    #[serde(default, deserialize_with = "opt_string_from_str")]
    pub screenshot: Option<String>,
    /// Raw decision/thinking from the agent
    #[serde(default)]
    pub raw_decision: Option<String>,
    /// Action taken — varies in format, so Value
    #[serde(default)]
    pub action: serde_json::Value,
    /// Result of the action — varies in format, so Value
    #[serde(default)]
    pub result: serde_json::Value,
}

/// A summary.json entry from computer_use.mjs.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct SummaryJson {
    pub goal: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub steps_taken: u32,
    #[serde(default)]
    pub max_steps: u32,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub final_url: Option<String>,
    #[serde(default)]
    pub page_title: Option<String>,
    #[serde(default)]
    pub fara_url: Option<String>,
}

/// Parse a Fara run directory into pages, flows, screenshots, and URLs.
/// Returns non-empty collections if successful artifacts exist.
pub fn parse_run_dir(run_dir: &Path, goal: &str, run_index: usize) -> anyhow::Result<ParseResult> {
    let trajectory_path = run_dir.join("trajectory.json");
    let summary_path = run_dir.join("summary.json");

    let mut pages: Vec<AamPage> = Vec::new();
    let mut flows: Vec<AamFlow> = Vec::new();
    let mut screenshots: Vec<String> = Vec::new();
    let mut urls: Vec<String> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    // Parse summary.json for URLs and metadata
    let summary = if summary_path.exists() {
        let raw = std::fs::read_to_string(&summary_path)?;
        match serde_json::from_str::<SummaryJson>(&raw) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("WARN: failed to parse summary.json: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Extract URLs from summary
    if let Some(ref s) = summary {
        if !s.url.is_empty() && seen_urls.insert(s.url.clone()) {
            urls.push(s.url.clone());
        }
        if let Some(ref fu) = s.final_url
            && !fu.is_empty()
            && seen_urls.insert(fu.clone())
        {
            urls.push(fu.clone());
        }
    }

    // Parse trajectory.json for pages, flows, screenshots
    let trajectory: Vec<TrajectoryEntry> = if trajectory_path.exists() {
        let raw = std::fs::read_to_string(&trajectory_path)?;
        match serde_json::from_str::<Vec<TrajectoryEntry>>(&raw) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("WARN: failed to parse trajectory.json: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // Collect screenshots
    for entry in &trajectory {
        if let Some(ref ss) = entry.screenshot
            && !ss.is_empty()
        {
            screenshots.push(ss.clone());
        }
    }

    // Also check for screenshot files in the directory
    if let Ok(entries) = std::fs::read_dir(run_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("screenshot")
                && (name_str.ends_with(".png") || name_str.ends_with(".jpg"))
                && !screenshots.contains(&name_str.to_string())
            {
                screenshots.push(name_str.to_string());
            }
        }
    }

    // Build pages and flows from trajectory entries
    let mut current_flow_pages: Vec<String> = Vec::new();
    let mut current_flow_steps: Vec<AamFlowStep> = Vec::new();

    for (i, entry) in trajectory.iter().enumerate() {
        let page_id = format!("page-{}-{:03}", run_index, i);
        let url = summary
            .as_ref()
            .and_then(|s| {
                if s.url.is_empty() {
                    None
                } else {
                    Some(s.url.clone())
                }
            })
            .unwrap_or_else(|| "unknown".into());

        // Extract action string from Value
        let action_str = extract_action_string(&entry.action);
        let _result_str = extract_result_string(&entry.result);

        // Build page
        let page = AamPage {
            id: page_id.clone(),
            path: url.clone(),
            title: summary
                .as_ref()
                .and_then(|s| s.page_title.clone())
                .unwrap_or_else(|| format!("Step {}", entry.step)),
            semantic: entry.raw_decision.clone().unwrap_or_default(),
            url_snapshot: url.clone(),
            elements: Vec::new(),
        };
        pages.push(page);
        current_flow_pages.push(page_id.clone());

        // Build flow step
        let meta = extract_action_metadata(&entry.action);
        let step = AamFlowStep {
            page: page_id,
            action: action_str,
            selector: meta.selector,
            value: meta.value,
            target: meta.target,
            expected: None,
            screenshot: entry.screenshot.clone(),
        };
        current_flow_steps.push(step);

        // Update URL if we have a new one
        if let Some(ref s) = summary
            && let Some(ref fu) = s.final_url
            && !fu.is_empty()
            && seen_urls.insert(fu.clone())
        {
            urls.push(fu.clone());
        }
    }

    // Create flow if we have steps
    if !current_flow_pages.is_empty() || !current_flow_steps.is_empty() {
        let flow = AamFlow {
            id: format!("flow-{}-{:03}", sanitize_for_id(goal), run_index),
            semantic: goal.into(),
            pages: current_flow_pages,
            steps: current_flow_steps,
            trajectory_hash: None,
        };
        flows.push(flow);
    }

    Ok(ParseResult {
        pages,
        flows,
        screenshots,
        urls,
    })
}

/// Extract a string from a JSON value using tolerant fallback strategies.
fn extract_string_from_value(value: &serde_json::Value, keys: &[&str]) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(o) => {
            for key in keys {
                if let Some(v) = o.get(*key).and_then(|v| v.as_str()) {
                    return v.to_string();
                }
            }
            serde_json::to_string(value).unwrap_or_default()
        }
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Extract action string from a serde_json::Value (tolerant handling).
pub fn extract_action_string(value: &serde_json::Value) -> String {
    extract_string_from_value(value, &["type", "action", "name"])
}

/// Metadata extracted from an action object: selector, value, target.
#[derive(Debug, Default, Clone)]
pub struct ActionMetadata {
    pub selector: Option<String>,
    pub value: Option<String>,
    pub target: Option<String>,
}

/// Extract selector/value/target from an action object when present.
pub fn extract_action_metadata(value: &serde_json::Value) -> ActionMetadata {
    let mut meta = ActionMetadata::default();
    if let serde_json::Value::Object(o) = value {
        if let Some(v) = o.get("selector").and_then(|v| v.as_str()) {
            meta.selector = Some(v.to_string());
        }
        if let Some(v) = o.get("value").and_then(|v| v.as_str()) {
            meta.value = Some(v.to_string());
        }
        if let Some(v) = o.get("target").and_then(|v| v.as_str()) {
            meta.target = Some(v.to_string());
        }
    }
    meta
}

/// Extract result string from a serde_json::Value (tolerant handling).
pub fn extract_result_string(value: &serde_json::Value) -> String {
    extract_string_from_value(value, &["message", "result", "text"])
}

/// Sanitize a string for use in an ID (replace non-alphanumeric with underscores).
pub fn sanitize_for_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// Tolerant deserializer: accept string or number for optional fields.
fn opt_string_from_str<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawOptString {
        Str(String),
        Num(u32),
        Empty,
    }

    let raw = RawOptString::deserialize(deserializer)?;
    match raw {
        RawOptString::Str(s) => Ok(Some(s)),
        RawOptString::Num(n) => Ok(Some(n.to_string())),
        RawOptString::Empty => Ok(None),
    }
}
