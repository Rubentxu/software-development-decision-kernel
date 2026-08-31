//! Deterministic oracle evaluators (ADR-014, eje 3).
//!
//! Los oracles deterministas miden sin IA: comparan la evidencia capturada
//! por el executor contra un criterio estructurado (`UatOracleSpec.expect`)
//! y producen `UatOracleAssessment` con confidence 1.0. Nunca deciden la
//! aceptación del release — solo aportan machine assessment; el humano o el
//! gate de release son la autoridad final (PASSED != ACCEPTED).
//!
//! Kinds deterministas implementados:
//! - ExitCode: exit status del run (expect: número, default 0)
//! - Http: status/url/content-type de la respuesta principal (expect:
//!   `{"status": 200}` o `{"status_range": "2xx"}`)
//! - Text: substring o regex en dom.html (expect: `{"contains": "..."}` o
//!   `{"regex": "..."}`)
//! - JsonSchema: valida un payload JSON contra un JSON Schema (expect:
//!   `{"schema": {...}, "payload_ref": "network.json"}`)
//! - Dom: presencia de selector en dom.html (expect: `{"selector": "#id"}`)
//! - Geometry: bounding box de selector (expect: `{"selector": "#id",
//!   "min_width": 100}`)
//! - Accessibility: snapshot ARIA (expect: `{"severity": "critical"}` —
//!   falla si hay violaciones de severidad >= esperada)
//! - VisualDiff: comparación de hashes de screenshot contra golden
//!   (expect: `{"golden_sha256": "sha256:..."}`)

use sddk_domain::{
    EvidenceArtifact, EvidenceBundle, EvidenceKind, UatOracleAssessment, UatOracleKind,
    UatOracleSpec, UatOracleVerdict,
};
use serde_json::Value;
use thiserror::Error;

/// Run context supplied by the caller (the executor outcome).
#[derive(Debug, Clone, Default)]
pub struct OracleRunContext {
    /// Exit status of the underlying run, if any.
    pub exit_status: Option<i32>,
    /// Final URL after redirects, if known.
    pub final_url: Option<String>,
}

/// Failure modes of the deterministic evaluator.
#[derive(Debug, Error)]
pub enum OracleError {
    /// The oracle kind is not deterministic; use the semantic evaluator.
    #[error("oracle kind {kind:?} is not deterministic")]
    NotDeterministic {
        /// Kind that was rejected.
        kind: UatOracleKind,
    },
    /// The expected payload could not be parsed from `expect`.
    #[error("invalid expect for {kind:?}: {message}")]
    BadExpect {
        /// Kind being evaluated.
        kind: UatOracleKind,
        /// What was wrong with the expect payload.
        message: String,
    },
    /// The evidence file referenced by the oracle is missing.
    #[error("missing evidence artifact {kind:?} (ref {reference})")]
    MissingEvidence {
        /// Kind being evaluated.
        kind: UatOracleKind,
        /// Reference of the missing artifact.
        reference: String,
    },
    /// A payload could not be decoded.
    #[error("cannot decode {path}: {message}")]
    Decode {
        /// Path that failed to decode.
        path: String,
        /// Decode error message.
        message: String,
    },
}

/// Evaluates a deterministic oracle against the evidence bundle.
///
/// Returns `Ok(assessment)` with verdict + confidence 1.0 when the evidence
/// allows a deterministic answer; `Err(MissingEvidence)` when the captured
/// payload is absent (the caller decides how to treat missing evidence —
/// typically `Uncertain` at the aggregation layer).
pub fn evaluate_deterministic(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    run: &OracleRunContext,
) -> Result<UatOracleAssessment, OracleError> {
    let assessment = |verdict: UatOracleVerdict, details: Option<String>| UatOracleAssessment {
        oracle: spec.clone(),
        verdict,
        confidence: 1.0,
        details,
    };

    match spec.kind {
        UatOracleKind::ExitCode => {
            let expected = spec
                .expect
                .as_ref()
                .and_then(|v| v.get("code"))
                .and_then(Value::as_i64)
                .unwrap_or(0) as i32;
            match run.exit_status {
                Some(actual) if actual == expected => Ok(assessment(
                    UatOracleVerdict::Pass,
                    Some(format!("exit {actual} == expected {expected}")),
                )),
                Some(actual) => Ok(assessment(
                    UatOracleVerdict::Fail,
                    Some(format!("exit {actual} != expected {expected}")),
                )),
                None => Ok(assessment(
                    UatOracleVerdict::Uncertain,
                    Some("no exit status recorded".into()),
                )),
            }
        }
        UatOracleKind::Http => evaluate_http(spec, bundle, &assessment),
        UatOracleKind::Text => evaluate_text(spec, bundle, &assessment),
        UatOracleKind::JsonSchema => evaluate_json_schema(spec, bundle, &assessment),
        UatOracleKind::Dom => evaluate_dom(spec, bundle, &assessment),
        UatOracleKind::Geometry => evaluate_geometry(spec, bundle, &assessment),
        UatOracleKind::Accessibility => evaluate_accessibility(spec, bundle, &assessment),
        UatOracleKind::VisualDiff => evaluate_visual_diff(spec, bundle, &assessment),
        // Semantic or human kinds are out of scope here.
        UatOracleKind::VisualAi | UatOracleKind::LlmRubric | UatOracleKind::Human => {
            Err(OracleError::NotDeterministic { kind: spec.kind })
        }
    }
}

fn evaluate_http<F>(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    assessment: &F,
) -> Result<UatOracleAssessment, OracleError>
where
    F: Fn(UatOracleVerdict, Option<String>) -> UatOracleAssessment,
{
    let artifact = artifact_of_kind(bundle, spec.kind, EvidenceKind::Http)?;
    let payload = read_json(artifact, spec.kind)?;
    let status = payload
        .get("status")
        .and_then(Value::as_u64)
        .ok_or_else(|| OracleError::BadExpect {
            kind: spec.kind,
            message: "http.json has no numeric `status`".into(),
        })?;

    let expect = spec.expect.clone().unwrap_or(Value::Null);
    let status_code = expect.get("status").and_then(Value::as_u64);
    let status_range = expect.get("status_range").and_then(Value::as_str);
    let in_range = |code: u64, range: &str| match range {
        "2xx" => (200..300).contains(&code),
        "3xx" => (300..400).contains(&code),
        "4xx" => (400..500).contains(&code),
        "5xx" => (500..600).contains(&code),
        _ => false,
    };
    match (status_code, status_range) {
        (Some(expected), _) if status == expected => Ok(assessment(
            UatOracleVerdict::Pass,
            Some(format!("http status {status} == {expected}")),
        )),
        (Some(expected), _) => Ok(assessment(
            UatOracleVerdict::Fail,
            Some(format!("http status {status} != expected {expected}")),
        )),
        (None, Some(range)) if in_range(status, range) => Ok(assessment(
            UatOracleVerdict::Pass,
            Some(format!("http status {status} in {range}")),
        )),
        (None, Some(range)) => Ok(assessment(
            UatOracleVerdict::Fail,
            Some(format!("http status {status} not in {range}")),
        )),
        (None, None) => Ok(assessment(
            UatOracleVerdict::Pass,
            Some(format!("http status {status} observed")),
        )),
    }
}

fn evaluate_text<F>(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    assessment: &F,
) -> Result<UatOracleAssessment, OracleError>
where
    F: Fn(UatOracleVerdict, Option<String>) -> UatOracleAssessment,
{
    // Text oracle: busca en dom.html si existe; si el executor fue cli/script
    // (command_output), busca en la salida capturada. Esto permite oracles
    // `text` sobre runs no-browser (dogfooding del propio framework).
    let artifact = artifact_of_kind(bundle, spec.kind, EvidenceKind::Dom)
        .or_else(|_| artifact_of_kind(bundle, spec.kind, EvidenceKind::CommandOutput))?;
    let haystack = read_text(artifact, spec.kind)?;
    let expect = spec.expect.clone().unwrap_or(Value::Null);
    let contains = expect.get("contains").and_then(Value::as_str);
    let regex = expect.get("regex").and_then(Value::as_str);

    if let Some(needle) = contains {
        if haystack.contains(needle) {
            Ok(assessment(
                UatOracleVerdict::Pass,
                Some(format!("output contains {needle:?}")),
            ))
        } else {
            Ok(assessment(
                UatOracleVerdict::Fail,
                Some(format!("output does not contain {needle:?}")),
            ))
        }
    } else if let Some(pattern) = regex {
        match regex_lite_search(pattern, &haystack) {
            Ok(true) => Ok(assessment(
                UatOracleVerdict::Pass,
                Some(format!("output matches regex {pattern:?}")),
            )),
            Ok(false) => Ok(assessment(
                UatOracleVerdict::Fail,
                Some(format!("output does not match regex {pattern:?}")),
            )),
            Err(message) => Err(OracleError::BadExpect {
                kind: spec.kind,
                message,
            }),
        }
    } else {
        Err(OracleError::BadExpect {
            kind: spec.kind,
            message: "text oracle needs `contains` or `regex`".into(),
        })
    }
}

fn evaluate_json_schema<F>(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    assessment: &F,
) -> Result<UatOracleAssessment, OracleError>
where
    F: Fn(UatOracleVerdict, Option<String>) -> UatOracleAssessment,
{
    let expect = spec.expect.clone().unwrap_or(Value::Null);
    let schema = expect
        .get("schema")
        .cloned()
        .ok_or_else(|| OracleError::BadExpect {
            kind: spec.kind,
            message: "json_schema oracle needs `schema`".into(),
        })?;
    // Locate the payload artifact: explicit `payload_ref` or first JSON
    // artifact with a payload-eligible kind.
    let artifact = match expect.get("payload_ref").and_then(Value::as_str) {
        Some(name) => bundle
            .artifacts
            .iter()
            .find(|a| a.path.as_deref().is_some_and(|p| p.ends_with(name)))
            .ok_or_else(|| OracleError::MissingEvidence {
                kind: spec.kind,
                reference: name.to_owned(),
            })?,
        None => bundle
            .artifacts
            .iter()
            .find(|a| matches!(a.kind, EvidenceKind::Network | EvidenceKind::File))
            .ok_or_else(|| OracleError::MissingEvidence {
                kind: spec.kind,
                reference: "<any json artifact>".into(),
            })?,
    };
    let payload = read_json(artifact, spec.kind)?;

    match validate_json_schema(&schema, &payload) {
        Ok(true) => Ok(assessment(
            UatOracleVerdict::Pass,
            Some("payload validates against schema".into()),
        )),
        Ok(false) => Ok(assessment(
            UatOracleVerdict::Fail,
            Some("payload does not validate against schema".into()),
        )),
        Err(message) => Err(OracleError::BadExpect {
            kind: spec.kind,
            message,
        }),
    }
}

fn evaluate_dom<F>(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    assessment: &F,
) -> Result<UatOracleAssessment, OracleError>
where
    F: Fn(UatOracleVerdict, Option<String>) -> UatOracleAssessment,
{
    let artifact = artifact_of_kind(bundle, spec.kind, EvidenceKind::Dom)?;
    let html = read_text(artifact, spec.kind)?;
    let expect = spec.expect.clone().unwrap_or(Value::Null);
    let selector = expect
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| OracleError::BadExpect {
            kind: spec.kind,
            message: "dom oracle needs `selector`".into(),
        })?;
    // Lite presence check: selector text (id, class, tag) appears in the
    // HTML snapshot. Exact CSS matching is a F12 concern (harness DOM).
    let present = selector
        .trim_start_matches(['#', '.', '[', ']', '"', '\''])
        .split([' ', '>', '['])
        .next()
        .map(|token| html.contains(token))
        .unwrap_or(false);
    if present {
        Ok(assessment(
            UatOracleVerdict::Pass,
            Some(format!("selector {selector:?} present in dom")),
        ))
    } else {
        Ok(assessment(
            UatOracleVerdict::Fail,
            Some(format!("selector {selector:?} missing from dom")),
        ))
    }
}

fn evaluate_geometry<F>(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    assessment: &F,
) -> Result<UatOracleAssessment, OracleError>
where
    F: Fn(UatOracleVerdict, Option<String>) -> UatOracleAssessment,
{
    let artifact = artifact_of_kind(bundle, spec.kind, EvidenceKind::Geometry)?;
    let payload = read_json(artifact, spec.kind)?;
    let expect = spec.expect.clone().unwrap_or(Value::Null);
    let selector = expect
        .get("selector")
        .and_then(Value::as_str)
        .ok_or_else(|| OracleError::BadExpect {
            kind: spec.kind,
            message: "geometry oracle needs `selector`".into(),
        })?;
    let box_value = payload
        .get(selector)
        .and_then(Value::as_object)
        .ok_or_else(|| OracleError::MissingEvidence {
            kind: spec.kind,
            reference: selector.to_owned(),
        })?;
    let check = |key: &str| -> Option<bool> {
        expect
            .get(key)
            .and_then(Value::as_f64)
            .map(|expected| box_value.get(key).and_then(Value::as_f64) == Some(expected))
    };
    let failures: Vec<String> = ["x", "y", "width", "height"]
        .iter()
        .filter_map(|key| check(key).and_then(|ok| (!ok).then(|| format!("{key} mismatch"))))
        .collect();
    let min_width = expect.get("min_width").and_then(Value::as_f64);
    if let Some(min) = min_width {
        let width = box_value
            .get("width")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        if width < min {
            return Ok(assessment(
                UatOracleVerdict::Fail,
                Some(format!("width {width} < min_width {min}")),
            ));
        }
    }
    if failures.is_empty() {
        Ok(assessment(
            UatOracleVerdict::Pass,
            Some(format!("geometry of {selector:?} matches expect")),
        ))
    } else {
        Ok(assessment(
            UatOracleVerdict::Fail,
            Some(failures.join(", ")),
        ))
    }
}

fn evaluate_accessibility<F>(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    assessment: &F,
) -> Result<UatOracleAssessment, OracleError>
where
    F: Fn(UatOracleVerdict, Option<String>) -> UatOracleAssessment,
{
    let artifact = artifact_of_kind(bundle, spec.kind, EvidenceKind::Aria)?;
    let payload = read_json(artifact, spec.kind)?;
    let severity_order = ["minor", "moderate", "serious", "critical"];
    let expected_severity = spec
        .severity
        .clone()
        .or_else(|| {
            spec.expect
                .as_ref()
                .and_then(|v| v.get("severity"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "critical".into());
    let expected_index = severity_order
        .iter()
        .position(|s| *s == expected_severity)
        .ok_or_else(|| OracleError::BadExpect {
            kind: spec.kind,
            message: format!("unknown severity {expected_severity:?}"),
        })?;

    let violations = payload.get("violations").and_then(Value::as_array);
    match violations {
        None => Ok(assessment(
            UatOracleVerdict::Pass,
            Some("no accessibility violations reported".into()),
        )),
        Some(list) => {
            let worst = list.iter().find_map(|v| {
                v.get("severity").and_then(Value::as_str).and_then(|s| {
                    severity_order
                        .iter()
                        .position(|known| *known == s)
                        .map(|idx| (idx, s.to_owned()))
                })
            });
            match worst {
                Some((idx, severity)) if idx >= expected_index => Ok(assessment(
                    UatOracleVerdict::Fail,
                    Some(format!(
                        "a11y violation at severity {severity:?} >= {expected_severity:?}"
                    )),
                )),
                _ => Ok(assessment(
                    UatOracleVerdict::Pass,
                    Some(format!("a11y violations below {expected_severity:?}")),
                )),
            }
        }
    }
}

fn evaluate_visual_diff<F>(
    spec: &UatOracleSpec,
    bundle: &EvidenceBundle,
    assessment: &F,
) -> Result<UatOracleAssessment, OracleError>
where
    F: Fn(UatOracleVerdict, Option<String>) -> UatOracleAssessment,
{
    let artifact = artifact_of_kind(bundle, spec.kind, EvidenceKind::Screenshot)?;
    let actual_ref = artifact.r#ref.clone();
    let expected_ref = spec
        .expect
        .as_ref()
        .and_then(|v| v.get("golden_sha256"))
        .and_then(Value::as_str)
        .ok_or_else(|| OracleError::BadExpect {
            kind: spec.kind,
            message: "visual_diff oracle needs `golden_sha256`".into(),
        })?;
    if actual_ref == expected_ref {
        Ok(assessment(
            UatOracleVerdict::Pass,
            Some("screenshot matches golden".into()),
        ))
    } else {
        Ok(assessment(
            UatOracleVerdict::Fail,
            Some(format!("screenshot {actual_ref} != golden {expected_ref}")),
        ))
    }
}

// --- helpers ---

fn artifact_of_kind(
    bundle: &EvidenceBundle,
    oracle_kind: UatOracleKind,
    kind: EvidenceKind,
) -> Result<&EvidenceArtifact, OracleError> {
    bundle
        .artifacts
        .iter()
        .find(|a| a.kind == kind)
        .ok_or_else(|| OracleError::MissingEvidence {
            kind: oracle_kind,
            reference: format!("{kind:?}"),
        })
}

fn read_text(
    artifact: &EvidenceArtifact,
    oracle_kind: UatOracleKind,
) -> Result<String, OracleError> {
    let path = artifact
        .path
        .as_deref()
        .ok_or_else(|| OracleError::MissingEvidence {
            kind: oracle_kind,
            reference: artifact.r#ref.clone(),
        })?;
    std::fs::read_to_string(path).map_err(|source| OracleError::Decode {
        path: path.to_owned(),
        message: source.to_string(),
    })
}

fn read_json(
    artifact: &EvidenceArtifact,
    oracle_kind: UatOracleKind,
) -> Result<Value, OracleError> {
    let path = artifact
        .path
        .as_deref()
        .ok_or_else(|| OracleError::MissingEvidence {
            kind: oracle_kind,
            reference: artifact.r#ref.clone(),
        })?;
    let raw = std::fs::read_to_string(path).map_err(|source| OracleError::Decode {
        path: path.to_owned(),
        message: source.to_string(),
    })?;
    serde_json::from_str(&raw).map_err(|source| OracleError::Decode {
        path: path.to_owned(),
        message: source.to_string(),
    })
}

/// Minimal regex support: `^...$` anchors, `.*`, literal escapes. Used by
/// the Text oracle; full regex engine is out of scope for deterministic
/// oracles (F12 harness may upgrade).
fn regex_lite_search(pattern: &str, haystack: &str) -> Result<bool, String> {
    let anchored_start = pattern.starts_with('^');
    let anchored_end = pattern.ends_with('$');
    let mut literal = pattern
        .trim_start_matches('^')
        .trim_end_matches('$')
        .to_owned();
    // Very lite: treat `.*` as a wildcard separator, everything else literal.
    let parts: Vec<&str> = literal.split(".*").collect();
    if parts.len() == 1 {
        literal = parts[0].to_owned();
        let found = haystack.contains(&literal);
        let ok_start = !anchored_start || haystack.starts_with(&literal);
        let ok_end = !anchored_end || haystack.ends_with(&literal);
        return Ok(found && ok_start && ok_end);
    }
    // Sequence of literal parts in order.
    let mut rest = haystack;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let Some(pos) = rest.find(part) else {
            return Ok(false);
        };
        if i == 0 && anchored_start && pos != 0 {
            return Ok(false);
        }
        rest = &rest[pos + part.len()..];
    }
    if anchored_end && !rest.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

/// Minimal JSON Schema validation: `type`, `required`, `properties`,
/// `items`, `enum`, `minimum`/`maximum`. No `$ref` support (out of scope
/// for deterministic oracles).
pub fn validate_json_schema(schema: &Value, instance: &Value) -> Result<bool, String> {
    if let Some(expected_type) = schema.get("type").and_then(Value::as_str) {
        let actual = match expected_type {
            "object" => instance.is_object(),
            "array" => instance.is_array(),
            "string" => instance.is_string(),
            "number" => instance.is_number(),
            "integer" => instance.as_i64().is_some(),
            "boolean" => instance.is_boolean(),
            "null" => instance.is_null(),
            other => return Err(format!("unsupported schema type {other:?}")),
        };
        if !actual {
            return Ok(false);
        }
    }
    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        let object = instance
            .as_object()
            .ok_or("required only valid on object")?;
        for key in required {
            let key = key.as_str().ok_or("required entries must be strings")?;
            if !object.contains_key(key) {
                return Ok(false);
            }
        }
    }
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        let object = instance
            .as_object()
            .ok_or("properties only valid on object")?;
        for (key, subschema) in props {
            if let Some(value) = object.get(key)
                && !validate_json_schema(subschema, value)?
            {
                return Ok(false);
            }
        }
    }
    if let Some(items) = schema.get("items") {
        let array = instance.as_array().ok_or("items only valid on array")?;
        for value in array {
            if !validate_json_schema(items, value)? {
                return Ok(false);
            }
        }
    }
    if let Some(enum_values) = schema.get("enum").and_then(Value::as_array)
        && !enum_values.contains(instance)
    {
        return Ok(false);
    }
    if let (Some(min), Some(n)) = (
        schema.get("minimum").and_then(Value::as_f64),
        instance.as_f64(),
    ) && n < min
    {
        return Ok(false);
    }
    if let (Some(max), Some(n)) = (
        schema.get("maximum").and_then(Value::as_f64),
        instance.as_f64(),
    ) && n > max
    {
        return Ok(false);
    }
    Ok(true)
}

/// Aggregates multiple oracle assessments into a summary verdict: any Fail
/// (blocking) -> Fail; no Fails and any Uncertain -> Uncertain; else Pass.
pub fn aggregate_verdict(assessments: &[UatOracleAssessment]) -> UatOracleVerdict {
    let has_fail = assessments
        .iter()
        .any(|a| a.verdict == UatOracleVerdict::Fail && a.oracle.blocking);
    let has_uncertain = assessments
        .iter()
        .any(|a| a.verdict == UatOracleVerdict::Uncertain);
    if has_fail {
        UatOracleVerdict::Fail
    } else if has_uncertain {
        UatOracleVerdict::Uncertain
    } else {
        UatOracleVerdict::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{EvidenceEnvironment, EvidenceExecution};

    static NEXT_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let n = NEXT_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("sddk-oracle-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn bundle_with_files(files: &[(&str, &str, &[u8])]) -> EvidenceBundle {
        // Unique dir per call (atomic counter) to avoid parallel-write
        // collisions between tests sharing artifact names.
        let tag = if files.is_empty() {
            "empty".to_owned()
        } else {
            files
                .iter()
                .map(|(name, _, _)| *name)
                .collect::<Vec<_>>()
                .join("+")
        };
        let dir = temp_dir(&tag);
        let mut bundle = EvidenceBundle {
            artifacts: Vec::new(),
            environment: EvidenceEnvironment::default(),
            execution: EvidenceExecution::default(),
        };
        for (name, kind, bytes) in files {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            bundle.artifacts.push(EvidenceArtifact {
                kind: match *kind {
                    "screenshot" => EvidenceKind::Screenshot,
                    "dom" => EvidenceKind::Dom,
                    "network" => EvidenceKind::Network,
                    "http" => EvidenceKind::Http,
                    "command_output" => EvidenceKind::CommandOutput,
                    "geometry" => EvidenceKind::Geometry,
                    "aria" => EvidenceKind::Aria,
                    _ => EvidenceKind::File,
                },
                r#ref: sddk_domain::sha256_hex(bytes),
                path: Some(path.display().to_string()),
                mime: None,
                size_bytes: Some(bytes.len() as u64),
                note: None,
            });
        }
        bundle
    }

    #[test]
    fn exit_code_passes_when_matching() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::ExitCode,
            expect: None,
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[]);
        let run = OracleRunContext {
            exit_status: Some(0),
            final_url: None,
        };
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
        assert_eq!(assessment.confidence, 1.0);
    }

    #[test]
    fn exit_code_fails_on_mismatch() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::ExitCode,
            expect: Some(serde_json::json!({"code": 7})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[]);
        let run = OracleRunContext {
            exit_status: Some(0),
            final_url: None,
        };
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn text_contains_passes() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Text,
            expect: Some(serde_json::json!({"contains": "Hello UAT"})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "dom.html",
            "dom",
            b"<html><body><h1>Hello UAT</h1></body></html>",
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn text_falls_back_to_command_output_for_cli_runs() {
        // Dogfooding: un run cli no produce dom.html; el oracle text debe
        // buscar en la salida capturada (command_output).
        let spec = UatOracleSpec {
            kind: UatOracleKind::Text,
            expect: Some(serde_json::json!({"contains": "uat validate: OK"})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[("output.log", "command_output", b"uat validate: OK\n")]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn text_missing_fails() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Text,
            expect: Some(serde_json::json!({"contains": "NOT THERE"})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[("dom.html", "dom", b"<html><body>hello</body></html>")]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn http_status_matches() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Http,
            expect: Some(serde_json::json!({"status": 200})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "http.json",
            "http",
            br#"{"status": 200, "url": "http://x"}"#,
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn http_range_fails() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Http,
            expect: Some(serde_json::json!({"status_range": "2xx"})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "http.json",
            "http",
            br#"{"status": 500, "url": "http://x"}"#,
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn json_schema_validates() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::JsonSchema,
            expect: Some(serde_json::json!({
                "schema": {
                    "type": "object",
                    "required": ["status"],
                    "properties": {"status": {"type": "integer"}}
                },
                "payload_ref": "http.json"
            })),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "http.json",
            "http",
            br#"{"status": 200, "url": "http://x"}"#,
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn json_schema_rejects_missing_required() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::JsonSchema,
            expect: Some(serde_json::json!({
                "schema": {
                    "type": "object",
                    "required": ["status"]
                },
                "payload_ref": "http.json"
            })),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[("http.json", "http", br#"{"url": "http://x"}"#)]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn dom_selector_present() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Dom,
            expect: Some(serde_json::json!({"selector": "#hero"})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "dom.html",
            "dom",
            b"<html><body><h1 id=\"hero\">Hi</h1></body></html>",
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn geometry_width_check() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Geometry,
            expect: Some(serde_json::json!({"selector": "#hero", "min_width": 100})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "geometry.json",
            "geometry",
            br##"{"#hero": {"x": 8, "y": 21, "width": 1350, "height": 43}}"##,
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn geometry_width_too_small_fails() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Geometry,
            expect: Some(serde_json::json!({"selector": "#hero", "min_width": 5000})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "geometry.json",
            "geometry",
            br##"{"#hero": {"x": 8, "y": 21, "width": 100, "height": 43}}"##,
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn accessibility_critical_violation_fails() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Accessibility,
            expect: None,
            rubric: vec![],
            severity: Some("critical".into()),
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "aria.json",
            "aria",
            br#"{"violations": [{"severity": "critical", "rule": "color-contrast"}]}"#,
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn accessibility_minor_passes_when_expecting_critical() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Accessibility,
            expect: None,
            rubric: vec![],
            severity: Some("critical".into()),
            blocking: true,
        };
        let bundle = bundle_with_files(&[(
            "aria.json",
            "aria",
            br#"{"violations": [{"severity": "minor", "rule": "heading-order"}]}"#,
        )]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn visual_diff_matches_golden() {
        let bytes = b"fake-png";
        let golden = sddk_domain::sha256_hex(bytes);
        let spec = UatOracleSpec {
            kind: UatOracleKind::VisualDiff,
            expect: Some(serde_json::json!({"golden_sha256": golden})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[("screenshot.png", "screenshot", bytes)]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Pass);
    }

    #[test]
    fn visual_diff_mismatch_fails() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::VisualDiff,
            expect: Some(serde_json::json!({"golden_sha256": "sha256:deadbeef"})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[("screenshot.png", "screenshot", b"other-png")]);
        let run = OracleRunContext::default();
        let assessment = evaluate_deterministic(&spec, &bundle, &run).unwrap();
        assert_eq!(assessment.verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn semantic_kinds_are_rejected() {
        for kind in [
            UatOracleKind::VisualAi,
            UatOracleKind::LlmRubric,
            UatOracleKind::Human,
        ] {
            let spec = UatOracleSpec {
                kind,
                expect: None,
                rubric: vec![],
                severity: None,
                blocking: true,
            };
            let bundle = bundle_with_files(&[]);
            let run = OracleRunContext::default();
            let err = evaluate_deterministic(&spec, &bundle, &run).unwrap_err();
            assert!(matches!(err, OracleError::NotDeterministic { .. }));
        }
    }

    #[test]
    fn missing_evidence_is_missing_evidence() {
        let spec = UatOracleSpec {
            kind: UatOracleKind::Text,
            expect: Some(serde_json::json!({"contains": "x"})),
            rubric: vec![],
            severity: None,
            blocking: true,
        };
        let bundle = bundle_with_files(&[]);
        let run = OracleRunContext::default();
        let err = evaluate_deterministic(&spec, &bundle, &run).unwrap_err();
        assert!(matches!(err, OracleError::MissingEvidence { .. }));
    }

    #[test]
    fn aggregate_blocks_on_any_fail() {
        let mk = |verdict| UatOracleAssessment {
            oracle: UatOracleSpec {
                kind: UatOracleKind::Text,
                expect: None,
                rubric: vec![],
                severity: None,
                blocking: true,
            },
            verdict,
            confidence: 1.0,
            details: None,
        };
        let verdict = aggregate_verdict(&[mk(UatOracleVerdict::Pass), mk(UatOracleVerdict::Fail)]);
        assert_eq!(verdict, UatOracleVerdict::Fail);
    }

    #[test]
    fn aggregate_uncertain_without_fail() {
        let mk = |verdict| UatOracleAssessment {
            oracle: UatOracleSpec {
                kind: UatOracleKind::Text,
                expect: None,
                rubric: vec![],
                severity: None,
                blocking: true,
            },
            verdict,
            confidence: 1.0,
            details: None,
        };
        let verdict =
            aggregate_verdict(&[mk(UatOracleVerdict::Pass), mk(UatOracleVerdict::Uncertain)]);
        assert_eq!(verdict, UatOracleVerdict::Uncertain);
    }
}
