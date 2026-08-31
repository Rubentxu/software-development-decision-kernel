//! Conversion of legacy unstructured agent output into structured results.

use serde_json::{Map, Value};
use thiserror::Error;

use crate::cycle::{AgentResult, AgentVerdict, ArtifactRef, Phase, ProposedRelation};

/// Errors emitted while converting legacy agent output.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LegacyError {
    /// The legacy payload is not a JSON object.
    #[error("legacy agent output must be a JSON object")]
    NotAnObject,
    /// A phase name could not be parsed.
    #[error("unknown phase: {phase}")]
    UnknownPhase {
        /// Unparseable phase name.
        phase: String,
    },
    /// A verdict value could not be mapped.
    #[error("unknown verdict: {verdict}")]
    UnknownVerdict {
        /// Unparseable verdict value.
        verdict: String,
    },
}

/// One structured conversion result with non-verifiable field warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyConversion {
    /// Structured agent result.
    pub result: AgentResult,
    /// Fields that could not be verified from the legacy payload.
    pub warnings: Vec<String>,
}

/// Converts free-form legacy JSON into a structured [`AgentResult`].
///
/// Known legacy field spellings are mapped tolerantly; anything unrecognized
/// produces a warning instead of failing. `schema_version`, `agent`,
/// `cycle_id`, `phase`, `verdict`, and `summary` are always synthesized.
pub fn convert_legacy_map(
    agent: &str,
    cycle_id: &str,
    phase: &Phase,
    value: &Value,
) -> Result<LegacyConversion, LegacyError> {
    let object = value.as_object().ok_or(LegacyError::NotAnObject)?;
    let mut warnings = Vec::new();

    let verdict = string_field(object, &["verdict", "status", "result_type"])
        .map(|verdict| parse_verdict(&verdict))
        .transpose()?
        .unwrap_or(AgentVerdict::Completed);
    if verdict == AgentVerdict::Completed
        && !object.contains_key("verdict")
        && !object.contains_key("status")
    {
        warnings.push("verdict missing; assumed completed".to_owned());
    }

    let summary = string_field(object, &["summary", "message", "output"])
        .unwrap_or_else(|| "(legacy output without summary)".to_owned());
    if !object.contains_key("summary") {
        warnings.push("summary synthesized from legacy message/output field".to_owned());
    }

    let mut artifacts = Vec::new();
    if let Some(Value::Array(items)) = object.get("artifacts") {
        for (index, item) in items.iter().enumerate() {
            match item {
                Value::String(path) => {
                    artifacts.push(ArtifactRef::new("artifact", path.clone()));
                }
                Value::Object(map) => {
                    let kind = map
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("artifact")
                        .to_owned();
                    let path = map
                        .get("path")
                        .or_else(|| map.get("file"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if path.is_empty() {
                        warnings.push(format!("artifact[{index}] has no path"));
                    }
                    artifacts.push(ArtifactRef::new(kind, path));
                }
                _ => warnings.push(format!("artifact[{index}] is not a string or object")),
            }
        }
    }

    let mut proposed_relations = Vec::new();
    if let Some(Value::Array(items)) = object
        .get("relations")
        .or_else(|| object.get("proposed_relations"))
    {
        for item in items {
            if let Value::Object(map) = item {
                let relation = ProposedRelation {
                    source: map
                        .get("source")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    relation_type: map
                        .get("relation_type")
                        .or_else(|| map.get("type"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    target: map
                        .get("target")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                };
                proposed_relations.push(relation);
            }
        }
    }

    for key in object.keys() {
        if ![
            "verdict",
            "status",
            "result_type",
            "summary",
            "message",
            "output",
            "artifacts",
            "relations",
            "proposed_relations",
            "evidence",
            "risks",
            "schema_version",
        ]
        .contains(&key.as_str())
        {
            warnings.push(format!("unrecognized legacy field: {key}"));
        }
    }

    let evidence = string_array(object, "evidence");
    let risks = string_array(object, "risks");

    Ok(LegacyConversion {
        result: AgentResult {
            schema_version: 1,
            agent: agent.to_owned(),
            cycle_id: cycle_id.to_owned(),
            phase: *phase,
            verdict,
            summary,
            artifacts,
            proposed_relations,
            evidence,
            risks,
            requested_capabilities: Vec::new(),
        },
        warnings,
    })
}

/// Converts a legacy text blob, treating the first non-empty line as the summary.
pub fn convert_legacy_text(
    agent: &str,
    cycle_id: &str,
    phase: &Phase,
    text: &str,
) -> LegacyConversion {
    let summary = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_owned();
    LegacyConversion {
        result: AgentResult {
            schema_version: 1,
            agent: agent.to_owned(),
            cycle_id: cycle_id.to_owned(),
            phase: *phase,
            verdict: AgentVerdict::Completed,
            summary,
            artifacts: Vec::new(),
            proposed_relations: Vec::new(),
            evidence: Vec::new(),
            risks: Vec::new(),
            requested_capabilities: Vec::new(),
        },
        warnings: vec![
            "text output is not structured; summary and verdict are unverifiable".to_owned(),
        ],
    }
}

fn string_field(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
}

fn string_array(object: &Map<String, Value>, key: &str) -> Vec<String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_verdict(value: &str) -> Result<AgentVerdict, LegacyError> {
    match value.to_ascii_lowercase().as_str() {
        "completed" | "success" | "succeeded" | "ok" | "pass" => Ok(AgentVerdict::Completed),
        "blocked" => Ok(AgentVerdict::Blocked),
        "needs_input" | "needs-input" | "needs_inputs" | "awaiting_input" => {
            Ok(AgentVerdict::NeedsInput)
        }
        "failed" | "failure" | "error" | "fail" => Ok(AgentVerdict::Failed),
        other => Err(LegacyError::UnknownVerdict {
            verdict: other.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use crate::cycle::Phase;
    use serde_json::json;

    use super::{LegacyError, convert_legacy_map, convert_legacy_text};

    #[test]
    fn converts_loose_map_with_warnings() {
        let conversion = convert_legacy_map(
            "explorer",
            "cycle-1",
            &Phase::Explore,
            &json!({
                "status": "success",
                "message": "explored",
                "artifacts": ["artifacts/report.md", {"kind": "map", "file": "x.md"}],
                "unknown_thing": 42,
            }),
        )
        .unwrap();
        let result = conversion.result;
        assert_eq!(result.agent, "explorer");
        assert_eq!(result.phase, Phase::Explore);
        assert_eq!(result.summary, "explored");
        assert_eq!(result.artifacts.len(), 2);
        assert!(
            conversion
                .warnings
                .iter()
                .any(|warning| warning.contains("unknown_thing"))
        );
        assert!(
            !conversion
                .warnings
                .iter()
                .any(|warning| warning.contains("verdict"))
        );
    }

    #[test]
    fn rejects_unknown_verdict_and_non_object() {
        assert_eq!(
            convert_legacy_map("a", "c", &Phase::Build, &json!({"verdict": "maybe"})),
            Err(LegacyError::UnknownVerdict {
                verdict: "maybe".into()
            })
        );
        assert_eq!(
            convert_legacy_map("a", "c", &Phase::Build, &json!([1, 2])),
            Err(LegacyError::NotAnObject)
        );
    }

    #[test]
    fn text_conversion_flags_unverifiable_fields() {
        let conversion =
            convert_legacy_text("explorer", "cycle-1", &Phase::Explore, "First line\nmore");
        assert_eq!(conversion.result.summary, "First line");
        assert_eq!(conversion.warnings.len(), 1);
        assert!(conversion.warnings[0].contains("unverifiable"));
    }
}
