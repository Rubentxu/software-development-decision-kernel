//! Semantic oracle evaluator (ADR-014, F9) — visual_ai / llm_rubric.
//!
//! Los oracles deterministas (F3) miden sin IA; los semánticos usan un
//! VLM/LLM local (Fara / llama.cpp) para evaluar evidencia contra una
//! rúbrica. Producen `UatOracleAssessment` con confidence < 1.0 — son
//! assessment preliminar, NUNCA la autoridad de aceptación (REQ-RF-023).

use std::path::{Path, PathBuf};

use sddk_domain::{UatOracleAssessment, UatOracleKind, UatOracleSpec, UatOracleVerdict};
use thiserror::Error;

/// What a semantic oracle run should assess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticOracleSpec {
    /// Oracle kind: VisualAi or LlmRubric.
    pub kind: UatOracleKind,
    /// Evidence input: screenshot path (visual_ai) or text file (llm_rubric).
    pub evidence_path: PathBuf,
    /// Rubric JSON file (array of criteria strings or {criteria: [...]}).
    pub rubric_path: PathBuf,
    /// Fara/llama.cpp OpenAI-compatible base URL.
    pub fara_url: String,
    /// Where assessment.json is written.
    pub output_dir: PathBuf,
    /// Wall-clock timeout per Fara call, in milliseconds.
    pub timeout_ms: u64,
}

/// Result of a semantic oracle run.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticOracleOutcome {
    /// Parsed assessment.
    pub assessment: UatOracleAssessment,
    /// Raw model output (first 2000 chars) for audit.
    pub raw_output: String,
}

/// Failures while running the semantic oracle harness.
#[derive(Debug, Error)]
pub enum SemanticOracleError {
    /// The harness exited non-zero or could not be spawned.
    #[error("semantic oracle {kind:?} failed: {message}")]
    Run {
        /// Kind being evaluated.
        kind: UatOracleKind,
        /// Failure message.
        message: String,
    },
    /// The evidence or rubric path does not exist.
    #[error("missing input for {kind:?}: {path}")]
    MissingInput {
        /// Kind being evaluated.
        kind: UatOracleKind,
        /// Path that is missing.
        path: String,
    },
    /// The output dir could not be prepared.
    #[error("cannot prepare output dir {path}: {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

fn semantic_env(fara_url: &str) -> std::collections::BTreeMap<String, String> {
    let mut env = std::collections::BTreeMap::new();
    for key in ["PATH", "HOME", "NODE_PATH", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            env.insert(key.to_owned(), value.to_string_lossy().into_owned());
        }
    }
    env.insert("FARA_URL".to_owned(), fara_url.to_owned());
    env
}

/// Executes a semantic oracle via `node <harness>`.
pub fn run_semantic_oracle(
    spec: &SemanticOracleSpec,
    harness_path: Option<&Path>,
    node_bin: Option<&str>,
) -> Result<SemanticOracleOutcome, SemanticOracleError> {
    use std::io::Read;

    if !spec.evidence_path.is_file() {
        return Err(SemanticOracleError::MissingInput {
            kind: spec.kind,
            path: spec.evidence_path.display().to_string(),
        });
    }
    if !spec.rubric_path.is_file() {
        return Err(SemanticOracleError::MissingInput {
            kind: spec.kind,
            path: spec.rubric_path.display().to_string(),
        });
    }

    let harness = harness_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::resolve_uat_driver("assess.mjs"));

    std::fs::create_dir_all(&spec.output_dir).map_err(|source| SemanticOracleError::Io {
        path: spec.output_dir.display().to_string(),
        source,
    })?;

    let kind_str = match spec.kind {
        UatOracleKind::VisualAi => "visual_ai",
        UatOracleKind::LlmRubric => "llm_rubric",
        _ => {
            return Err(SemanticOracleError::Run {
                kind: spec.kind,
                message: "kind is not semantic".into(),
            });
        }
    };
    let evidence_flag = if spec.kind == UatOracleKind::VisualAi {
        "--screenshot"
    } else {
        "--text"
    };

    let args: Vec<String> = vec![
        harness.display().to_string(),
        "--kind".into(),
        kind_str.into(),
        evidence_flag.into(),
        spec.evidence_path.display().to_string(),
        "--rubric".into(),
        spec.rubric_path.display().to_string(),
        "--output".into(),
        spec.output_dir.display().to_string(),
        "--fara-url".into(),
        spec.fara_url.clone(),
        "--timeout".into(),
        spec.timeout_ms.to_string(),
    ];
    let run_spec = crate::runner::RunSpec {
        program: node_bin.unwrap_or("node").to_owned(),
        args,
        env: semantic_env(&spec.fara_url),
        timeout_ms: spec.timeout_ms + 10_000,
        output_max_bytes: 1_048_576,
    };

    let outcome = crate::runner::run(&run_spec).map_err(|source| SemanticOracleError::Run {
        kind: spec.kind,
        message: format!("spawn failed: {source}"),
    })?;
    if outcome.exit_status != Some(0) {
        let message = if !outcome.stderr.trim().is_empty() {
            outcome.stderr.trim().to_owned()
        } else {
            outcome.stdout.trim().to_owned()
        };
        return Err(SemanticOracleError::Run {
            kind: spec.kind,
            message: if message.is_empty() {
                format!("exit {:?}", outcome.exit_status)
            } else {
                message
            },
        });
    }

    // Parse the assessment.
    let assessment_path = spec.output_dir.join("assessment.json");
    let mut raw = String::new();
    std::fs::File::open(&assessment_path)
        .map_err(|source| SemanticOracleError::Io {
            path: assessment_path.display().to_string(),
            source,
        })?
        .read_to_string(&mut raw)
        .map_err(|source| SemanticOracleError::Io {
            path: assessment_path.display().to_string(),
            source,
        })?;
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|source| SemanticOracleError::Run {
            kind: spec.kind,
            message: format!("invalid assessment.json: {source}"),
        })?;

    let verdict = match value
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("uncertain")
    {
        "pass" => UatOracleVerdict::Pass,
        "fail" => UatOracleVerdict::Fail,
        _ => UatOracleVerdict::Uncertain,
    };
    let confidence = value
        .get("confidence")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let details = value
        .get("details")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let raw_output = value
        .get("raw")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();

    Ok(SemanticOracleOutcome {
        assessment: UatOracleAssessment {
            oracle: UatOracleSpec {
                kind: spec.kind,
                expect: None,
                rubric: vec![],
                severity: None,
                blocking: true,
            },
            verdict,
            confidence,
            details,
        },
        raw_output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_evidence_is_reported_before_spawn() {
        let spec = SemanticOracleSpec {
            kind: UatOracleKind::VisualAi,
            evidence_path: PathBuf::from("/nonexistent/shot.png"),
            rubric_path: PathBuf::from("/nonexistent/rubric.json"),
            fara_url: "http://127.0.0.1:8082".into(),
            output_dir: PathBuf::from("/tmp/sddk-sem-x"),
            timeout_ms: 10_000,
        };
        let err = run_semantic_oracle(&spec, None, Some("node")).unwrap_err();
        assert!(matches!(err, SemanticOracleError::MissingInput { .. }));
    }

    #[test]
    fn non_semantic_kind_is_rejected() {
        let spec = SemanticOracleSpec {
            kind: UatOracleKind::Text,
            evidence_path: PathBuf::from("/nonexistent/x"),
            rubric_path: PathBuf::from("/nonexistent/r.json"),
            fara_url: "http://127.0.0.1:8082".into(),
            output_dir: PathBuf::from("/tmp/sddk-sem-y"),
            timeout_ms: 10_000,
        };
        let err = run_semantic_oracle(&spec, None, Some("node")).unwrap_err();
        assert!(matches!(err, SemanticOracleError::MissingInput { .. }));
    }
}
