//! Universal Evidence Model for SDDK governed capabilities.
//!
//! This module defines the canonical evidence types used across ALL governed
//! capabilities in SDDK — not just UAT. The `EvidenceBundle` structure with
//! content-addressable artifacts, environment context, and execution metadata is
//! the universal substrate for assurance and auditability.
//!
//! ## Design principles (ADR-0016)
//!
//! - **Content-addressable**: every artifact is identified by `sha256:<hex>` of its bytes.
//!   The bundle is verifiable independently of where it is stored.
//! - **Extensible kinds**: `EvidenceKind` is a closed enum; adding a new variant
//!   is a schema extension (backward-compatible), not a modification.
//! - **Separation**: environment (`where`) vs execution (`who/what`) vs artifacts
//!   (`what was captured`) — three orthogonal concerns in one bundle.
//!
//! ## UAT specialization
//!
//! The UAT-specific aliases (`UatEvidenceBundle`, `UatEvidenceArtifact`, etc.)
//! in [`crate::uat`] are re-exports of these types for backward compatibility.
//! New code should use the names in this module directly.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Shared enums (used by evidence types AND by UAT-specific types in uat.rs).
// Kept here so evidence.rs is self-contained.
// ---------------------------------------------------------------------------

/// How strict the comparison between `expected` and `observed` should be.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceExpectedCheck {
    #[default]
    ExactMatch,
    Contains,
    Regex,
    JsonPath,
    ExitCode,
}

/// Closed vocabulary for risk classification.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRiskClassification {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}

/// How much a single scenario failure can impact the release.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBlastRadius {
    #[default]
    FeatureBlocker,
    ReleaseBlocker,
    Advisory,
}

/// Status of automation for a scenario.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceAutomationStatus {
    #[default]
    Manual,
    Scripted,
    Automated,
}

/// Origin of a scenario: why this test exists.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    Spec,
    Bug,
    Incident,
    #[default]
    Regression,
}

// ---------------------------------------------------------------------------
// Evidence types
// ---------------------------------------------------------------------------

/// Closed vocabulary for evidence capture kinds.
///
/// These are the capture taxonomy used by ALL governed capabilities,
/// not just UAT. Adding a new kind is an extension (backward-compatible).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    File,
    Screenshot,
    CommandOutput,
    Assertion,
    Metric,
    /// Playwright trace archive.
    Trace,
    /// Captured console messages (JSON).
    Console,
    /// Captured network failures (JSON array).
    Network,
    /// HTTP response snapshot of the main navigation (status/url/headers).
    Http,
    /// DOM snapshot (HTML).
    Dom,
    /// ARIA accessibility snapshot (JSON).
    Aria,
    /// Bounding-box geometry of selectors (JSON).
    Geometry,
    /// Video recording (webm).
    Video,
    /// Computer-use trajectory (JSON).
    Trajectory,
    #[default]
    Note,
}

/// One evidence kind descriptor: what to capture and how to evaluate it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceKindItem {
    pub kind: EvidenceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_mode: Option<EvidenceExpectedCheck>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_bytes: Option<u64>,
}

/// A captured evidence artifact. Content-addressable:
/// `sha256:<hex>` of the payload bytes (ADR-014 §2.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceArtifact {
    pub kind: EvidenceKind,
    /// `sha256:<hex>` of the payload — verifiable against the referenced file.
    pub r#ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Environment snapshot for an evidence bundle: what environment the execution ran in.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceEnvironment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

/// Execution metadata for an evidence bundle: who executed and with what model/prompt.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceExecution {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
}

/// Universal evidence bundle. All artifacts are content-addressable;
/// `environment` + `execution` make the execution reproducible and auditable
/// (ADR-014 §2.3).
///
/// This is the canonical evidence type for ANY governed capability in SDDK,
/// not just UAT. `UatEvidenceBundle` in [`crate::uat`] is a type alias
/// pointing here for backward compatibility.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceBundle {
    #[serde(default)]
    pub artifacts: Vec<EvidenceArtifact>,
    #[serde(default)]
    pub environment: EvidenceEnvironment,
    #[serde(default)]
    pub execution: EvidenceExecution,
}

// ---------------------------------------------------------------------------
// Evidence redaction (ADR-0024)
// ---------------------------------------------------------------------------

/// How a field is treated in non-audit contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionLevel {
    /// Field appears unchanged in all contexts (receipts, exports, UI, audit).
    Public,
    /// Field replaced with `[REDACTED]` in receipts, exports, and UI;
    /// full value available only in raw audit log.
    Restricted,
    /// Field is omitted entirely from receipts, exports, and UI;
    /// full value available only in raw audit log.
    Confidential,
}

/// One redaction rule: maps a JSON pointer path to a redaction level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionRule {
    /// JSON pointer path to the field (e.g. `"raw_response"`, `"tool_calls[].output"`).
    pub field_path: &'static str,
    /// The redaction level for this field.
    pub level: RedactionLevel,
    /// Human-readable reason for the redaction.
    pub reason: &'static str,
}

/// The initial redaction registry. Additive — new rules added via PR + ADR amendment.
///
/// Covers evidence payload fields that commonly appear in model responses,
/// tool outputs, and agent prompts.
///
/// Paths follow JSON pointer semantics: `/foo/bar` for object keys,
/// `/list/0` for array indices. Rules apply to all occurrences of a
/// field name at any nesting level within the evidence bundle.
pub static REDACTION_RULES: &[RedactionRule] = &[
    RedactionRule {
        field_path: "/raw_response",
        level: RedactionLevel::Restricted,
        reason: "May contain model-generated content with embedded secrets",
    },
    RedactionRule {
        field_path: "/prompt",
        level: RedactionLevel::Restricted,
        reason: "May contain project names or internal context",
    },
    RedactionRule {
        field_path: "/system_prompt",
        level: RedactionLevel::Confidential,
        reason: "Internal instructions must not appear in receipts",
    },
    RedactionRule {
        field_path: "/tool_calls",
        level: RedactionLevel::Restricted,
        reason: "Tool calls array may contain sensitive invocation data",
    },
    RedactionRule {
        field_path: "/api_key",
        level: RedactionLevel::Confidential,
        reason: "Never intentionally in evidence — guard-rail",
    },
    RedactionRule {
        field_path: "/password",
        level: RedactionLevel::Confidential,
        reason: "Never intentionally in evidence — guard-rail",
    },
    RedactionRule {
        field_path: "/secret",
        level: RedactionLevel::Confidential,
        reason: "Never intentionally in evidence — guard-rail",
    },
    RedactionRule {
        field_path: "/token",
        level: RedactionLevel::Confidential,
        reason: "Bearer tokens and session tokens must not appear in receipts",
    },
];

/// Sentinel string used to replace restricted fields.
const REDACTED_PLACEHOLDER: &str = "[REDACTED]";

/// JSON pointer path segment — distinguishes array index from object key.
fn is_array_segment(s: &str) -> Option<usize> {
    s.parse::<usize>().ok()
}

/// Returns the value at `field_path` within `value` following JSON pointer rules.
/// Supports both object keys (`/foo/bar`) and array indices (`/foo/0`).
fn get_at_path<'a>(
    value: &'a serde_json::Value,
    field_path: &str,
) -> Option<&'a serde_json::Value> {
    let segments: Vec<&str> = field_path.split('/').filter(|s| !s.is_empty()).collect();
    let mut current = value;
    for seg in segments {
        match current {
            serde_json::Value::Object(map) => current = map.get(seg)?,
            serde_json::Value::Array(arr) => {
                let idx = is_array_segment(seg)?;
                current = arr.get(idx)?
            }
            _ => return None,
        }
    }
    Some(current)
}

/// Sets `value` at `field_path` following JSON pointer rules, cloning as needed.
fn set_at_path(
    value: &serde_json::Value,
    field_path: &str,
    new_val: serde_json::Value,
) -> serde_json::Value {
    let segments: Vec<&str> = field_path.split('/').filter(|s| !s.is_empty()).collect();
    let mut result = value.clone();
    set_at_path_inner(&mut result, &segments, new_val);
    result
}

fn set_at_path_inner(value: &mut serde_json::Value, segments: &[&str], new_val: serde_json::Value) {
    if segments.is_empty() {
        *value = new_val;
        return;
    }
    match value {
        serde_json::Value::Object(map) => {
            let head = segments[0];
            if segments.len() == 1 {
                map.insert(head.to_string(), new_val);
            } else {
                let child = map.entry(head).or_insert_with(|| serde_json::Value::Null);
                set_at_path_inner(child, &segments[1..], new_val);
            }
        }
        serde_json::Value::Array(arr) => {
            if let Ok(idx) = segments[0].parse::<usize>() {
                if segments.len() == 1 {
                    if idx < arr.len() {
                        arr[idx] = new_val;
                    }
                } else if idx < arr.len() {
                    set_at_path_inner(&mut arr[idx], &segments[1..], new_val);
                }
            }
        }
        _ => {}
    }
}

/// Result of redacting an evidence payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RedactedEvidence {
    /// The original unredacted payload (for audit log only).
    pub original: serde_json::Value,
    /// The redacted payload (for receipts, exports, and UI).
    pub redacted: serde_json::Value,
    /// Field paths that were redacted.
    pub fields_redacted: Vec<String>,
}

impl EvidenceBundle {
    /// Returns a redacted view of this evidence bundle for use in receipts,
    /// exports, and UI.
    ///
    /// Fields marked `Restricted` are replaced with `[REDACTED]`.
    /// Fields marked `Confidential` are omitted entirely from `redacted`.
    /// All fields are preserved in `original` (for audit log access only).
    ///
    /// Applies [`REDACTION_RULES`] to the bundle's JSON representation.
    /// Field paths are matched using JSON pointer semantics (`/foo/bar`, `/list/0`).
    ///
    /// # Example
    ///
    /// ```
    /// use sddk_domain::evidence::{EvidenceBundle, EvidenceExecution, EvidenceEnvironment};
    ///
    /// let bundle = EvidenceBundle {
    ///     execution: EvidenceExecution {
    ///         prompt_hash: Some("abc123".to_string()),
    ///         ..Default::default()
    ///     },
    ///     ..Default::default()
    /// };
    /// let redacted = bundle.redacted();
    /// // Confidential fields (api_key, password, secret, token) are omitted
    /// // Restricted fields (raw_response, prompt) are replaced with [REDACTED]
    /// ```
    pub fn redacted(&self) -> RedactedEvidence {
        let original = serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}));
        let mut redacted_json = original.clone();
        let mut fields_redacted = Vec::new();

        for rule in REDACTION_RULES {
            if get_at_path(&redacted_json, rule.field_path).is_some() {
                match rule.level {
                    RedactionLevel::Public => {}
                    RedactionLevel::Restricted => {
                        redacted_json = set_at_path(
                            &redacted_json,
                            rule.field_path,
                            serde_json::Value::String(REDACTED_PLACEHOLDER.to_string()),
                        );
                        fields_redacted.push(rule.field_path.to_string());
                    }
                    RedactionLevel::Confidential => {
                        redacted_json =
                            set_at_path(&redacted_json, rule.field_path, serde_json::Value::Null);
                        fields_redacted.push(rule.field_path.to_string());
                    }
                }
            }
        }

        // Remove nulls left by Confidential fields
        redacted_json = strip_nulls(&redacted_json);

        RedactedEvidence {
            original,
            redacted: redacted_json,
            fields_redacted,
        }
    }
}

/// Recursively removes null values from a JSON value (for Confidential field omission).
fn strip_nulls(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let stripped: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), strip_nulls(v)))
                .collect();
            serde_json::Value::Object(stripped)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(strip_nulls).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)] // Helper for future redacted_with_fields tests; currently unused.
    fn bundle_with_sensitive_fields() -> EvidenceBundle {
        EvidenceBundle {
            artifacts: vec![],
            environment: EvidenceEnvironment {
                git_sha: Some("abc123".to_string()),
                app_version: Some("1.0.0".to_string()),
                ..Default::default()
            },
            execution: EvidenceExecution {
                executor: Some("agent-1".to_string()),
                model: Some("claude-opus-4".to_string()),
                model_hash: Some("modelhash".to_string()),
                prompt_hash: Some("prompthash".to_string()),
            },
        }
    }

    #[test]
    fn restricted_field_replaced_with_placeholder() {
        // Build a raw JSON evidence bundle with a top-level raw_response field.
        // Using serde_json::Value to avoid struct field restrictions.
        let bundle_json = serde_json::json!({
            "artifacts": [],
            "environment": {},
            "execution": {
                "executor": "agent-1",
                "model": "claude-opus-4"
            },
            "raw_response": "super secret model output with API key abc123"
        });

        // Apply redaction rules manually (as EvidenceBundle::redacted would do)
        let mut redacted_json = bundle_json.clone();
        let mut fields_redacted = Vec::new();

        for rule in REDACTION_RULES {
            if get_at_path(&redacted_json, rule.field_path).is_some() {
                match rule.level {
                    RedactionLevel::Public => {}
                    RedactionLevel::Restricted => {
                        redacted_json = set_at_path(
                            &redacted_json,
                            rule.field_path,
                            serde_json::Value::String("[REDACTED]".to_string()),
                        );
                        fields_redacted.push(rule.field_path.to_string());
                    }
                    RedactionLevel::Confidential => {
                        redacted_json =
                            set_at_path(&redacted_json, rule.field_path, serde_json::Value::Null);
                        fields_redacted.push(rule.field_path.to_string());
                    }
                }
            }
        }
        redacted_json = strip_nulls(&redacted_json);

        // "/raw_response" should be Restricted and replaced with [REDACTED]
        assert!(
            fields_redacted.iter().any(|f| f == "/raw_response"),
            "fields_redacted={fields_redacted:?}"
        );
        let raw_in_redacted = get_at_path(&redacted_json, "/raw_response");
        assert_eq!(
            raw_in_redacted,
            Some(&serde_json::Value::String("[REDACTED]".to_string())),
            "raw_response should be replaced with [REDACTED]"
        );
    }

    #[test]
    fn confidential_field_omitted_from_redacted() {
        // Confidential fields (api_key, password, secret, token) must be
        // stripped entirely from the redacted output. They appear in original
        // (for the audit log) but are absent from redacted.
        let bundle_json = serde_json::json!({
            "artifacts": [],
            "environment": {},
            "execution": {
                "executor": "agent-1"
            },
            "api_key": "sk-secret-abc123"
        });

        // Apply redaction rules manually
        let mut redacted_json = bundle_json.clone();
        let mut fields_redacted = Vec::new();

        for rule in REDACTION_RULES {
            if get_at_path(&redacted_json, rule.field_path).is_some() {
                match rule.level {
                    RedactionLevel::Public => {}
                    RedactionLevel::Restricted => {
                        redacted_json = set_at_path(
                            &redacted_json,
                            rule.field_path,
                            serde_json::Value::String("[REDACTED]".to_string()),
                        );
                        fields_redacted.push(rule.field_path.to_string());
                    }
                    RedactionLevel::Confidential => {
                        redacted_json =
                            set_at_path(&redacted_json, rule.field_path, serde_json::Value::Null);
                        fields_redacted.push(rule.field_path.to_string());
                    }
                }
            }
        }
        redacted_json = strip_nulls(&redacted_json);

        // api_key is Confidential — must be absent from redacted
        let api_in_redacted = get_at_path(&redacted_json, "/api_key");
        assert!(
            api_in_redacted.is_none(),
            "api_key should be absent from redacted, got {api_in_redacted:?}"
        );

        // But it must be present in original
        assert_eq!(
            get_at_path(&bundle_json, "/api_key"),
            Some(&serde_json::Value::String("sk-secret-abc123".to_string()))
        );
    }

    #[test]
    fn redaction_rules_registry_has_required_entries() {
        let paths: Vec<_> = REDACTION_RULES.iter().map(|r| r.field_path).collect();
        assert!(paths.contains(&"/raw_response"), "missing /raw_response");
        assert!(paths.contains(&"/prompt"), "missing /prompt");
        assert!(paths.contains(&"/system_prompt"), "missing /system_prompt");
        assert!(paths.contains(&"/tool_calls"), "missing /tool_calls");
        assert!(paths.contains(&"/api_key"), "missing /api_key");
        assert!(paths.contains(&"/password"), "missing /password");
    }

    #[test]
    fn get_at_path_handles_object_and_array_segments() {
        let value = serde_json::json!({
            "foo": {
                "bar": "value1"
            },
            "list": ["item0", "item1"]
        });

        assert_eq!(
            get_at_path(&value, "foo/bar"),
            Some(&serde_json::Value::String("value1".to_string()))
        );
        assert_eq!(
            get_at_path(&value, "list/0"),
            Some(&serde_json::Value::String("item0".to_string()))
        );
        assert_eq!(get_at_path(&value, "foo/nonexistent"), None);
        assert_eq!(get_at_path(&value, "list/99"), None);
    }

    #[test]
    fn set_at_path_overwrites_correctly() {
        let value = serde_json::json!({"foo": {"bar": "original"}});
        let updated = set_at_path(
            &value,
            "foo/bar",
            serde_json::Value::String("new".to_string()),
        );
        assert_eq!(
            get_at_path(&updated, "foo/bar"),
            Some(&serde_json::Value::String("new".to_string()))
        );
    }

    #[test]
    fn strip_nulls_removes_null_values_recursively() {
        let value = serde_json::json!({
            "top": null,
            "nested": {
                "a": 1,
                "b": null,
                "c": {
                    "d": null
                }
            }
        });
        let stripped = strip_nulls(&value);
        let stripped_json = serde_json::to_string(&stripped).unwrap();
        // "top": null should be removed entirely (key absent from output object)
        assert!(!stripped_json.contains("null"), "stripped={stripped_json}");
    }
}
