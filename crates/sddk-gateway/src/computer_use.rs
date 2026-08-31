//! ComputerUseExecutor — adaptador de agente autónomo (ADR-014, F8).
//!
//! Sensor + actuador, NUNCA juez: ejecuta un `goal` de forma autónoma con
//! un budget de pasos contra una URL usando el harness
//! `assets/uat-driver/computer_use.mjs`, que habla con un servidor
//! OpenAI-compatible (Fara / llama.cpp) en bucle observe→think→act.
//! La trayectoria completa (screenshots + decisiones + resultados) queda en
//! el directorio de evidencia para `EvidenceCollector`.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// What a computer-use run should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerUseSpec {
    /// Target URL to open.
    pub url: String,
    /// Goal expressed in natural language (the agent's mission).
    pub goal: String,
    /// Maximum observe→think→act steps (budget guard).
    pub max_steps: u32,
    /// Fara/llama.cpp OpenAI-compatible base URL.
    pub fara_url: String,
    /// Where the evidence directory is written.
    pub output_dir: PathBuf,
    /// Wall-clock timeout per Fara call, in milliseconds.
    pub timeout_ms: u64,
}

impl ComputerUseSpec {
    /// Minimal spec: open a URL with a goal and a step budget.
    pub fn new(url: impl Into<String>, goal: impl Into<String>, output_dir: PathBuf) -> Self {
        Self {
            url: url.into(),
            goal: goal.into(),
            max_steps: 10,
            fara_url: std::env::var("FARA_URL").unwrap_or_else(|_| "http://127.0.0.1:8082".into()),
            output_dir,
            timeout_ms: 60_000,
        }
    }
}

/// Result of a computer-use run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerUseOutcome {
    /// Directory holding trajectory + screenshots.
    pub evidence_dir: PathBuf,
    /// Steps actually taken (<= max_steps).
    pub steps_taken: u32,
    /// Whether the agent declared the goal done.
    pub done: bool,
    /// Why the loop stopped: `agent_done` | `no_progress` | `max_steps`.
    pub stop_reason: String,
    /// Final page title (if captured).
    pub page_title: Option<String>,
}

/// Failures while running the computer-use harness.
#[derive(Debug, Error)]
pub enum ComputerUseError {
    /// The harness exited non-zero or could not be spawned.
    #[error("computer-use run failed for {url}: {message}")]
    Run {
        /// Target URL.
        url: String,
        /// Failure message.
        message: String,
    },
    /// The evidence dir could not be prepared.
    #[error("cannot prepare evidence dir {path}: {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

/// Environment allowlist: node/browser + FARA_URL for the harness.
fn computer_use_env(fara_url: &str) -> std::collections::BTreeMap<String, String> {
    let mut env = std::collections::BTreeMap::new();
    for key in ["PATH", "HOME", "NODE_PATH", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            env.insert(key.to_owned(), value.to_string_lossy().into_owned());
        }
    }
    env.insert("FARA_URL".to_owned(), fara_url.to_owned());
    env
}

/// Executes a computer-use run via `node <harness>`.
///
/// `harness_path` defaults to `assets/uat-driver/computer_use.mjs` relative
/// to the current directory; callers embedding the framework can pass an
/// absolute path.
pub fn run_computer_use(
    spec: &ComputerUseSpec,
    harness_path: Option<&Path>,
    node_bin: Option<&str>,
) -> Result<ComputerUseOutcome, ComputerUseError> {
    use std::io::Read;

    let harness = harness_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::resolve_uat_driver("computer_use.mjs"));

    std::fs::create_dir_all(&spec.output_dir).map_err(|source| ComputerUseError::Io {
        path: spec.output_dir.display().to_string(),
        source,
    })?;

    let args: Vec<String> = vec![
        harness.display().to_string(),
        "--url".into(),
        spec.url.clone(),
        "--goal".into(),
        spec.goal.clone(),
        "--output".into(),
        spec.output_dir.display().to_string(),
        "--max-steps".into(),
        spec.max_steps.to_string(),
        "--fara-url".into(),
        spec.fara_url.clone(),
        "--timeout".into(),
        spec.timeout_ms.to_string(),
    ];
    let run_spec = crate::runner::RunSpec {
        program: node_bin.unwrap_or("node").to_owned(),
        args,
        env: computer_use_env(&spec.fara_url),
        timeout_ms: spec
            .timeout_ms
            .saturating_mul(spec.max_steps as u64 + 1)
            .max(60_000),
        output_max_bytes: 1_048_576,
    };

    let outcome = crate::runner::run(&run_spec).map_err(|source| ComputerUseError::Run {
        url: spec.url.clone(),
        message: format!("spawn failed: {source}"),
    })?;
    if outcome.exit_status != Some(0) {
        let message = if !outcome.stderr.trim().is_empty() {
            outcome.stderr.trim().to_owned()
        } else {
            outcome.stdout.trim().to_owned()
        };
        return Err(ComputerUseError::Run {
            url: spec.url.clone(),
            message: if message.is_empty() {
                format!("exit {:?}", outcome.exit_status)
            } else {
                message
            },
        });
    }

    // Parse the harness summary.
    let summary_path = spec.output_dir.join("summary.json");
    let mut raw = String::new();
    std::fs::File::open(&summary_path)
        .map_err(|source| ComputerUseError::Io {
            path: summary_path.display().to_string(),
            source,
        })?
        .read_to_string(&mut raw)
        .map_err(|source| ComputerUseError::Io {
            path: summary_path.display().to_string(),
            source,
        })?;
    let summary: serde_json::Value =
        serde_json::from_str(&raw).map_err(|source| ComputerUseError::Run {
            url: spec.url.clone(),
            message: format!("invalid harness summary: {source}"),
        })?;

    Ok(ComputerUseOutcome {
        evidence_dir: spec.output_dir.clone(),
        steps_taken: summary
            .get("steps_taken")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32,
        done: summary
            .get("done")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        stop_reason: summary
            .get("stop_reason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        page_title: summary
            .get("page_title")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_new_sets_sane_defaults() {
        let spec = ComputerUseSpec::new(
            "https://example.com",
            "fill the form",
            PathBuf::from("/tmp/cu"),
        );
        assert_eq!(spec.max_steps, 10);
        assert_eq!(spec.timeout_ms, 60_000);
        assert!(!spec.fara_url.is_empty());
    }

    #[test]
    fn missing_harness_is_reported_before_spawn() {
        let spec = ComputerUseSpec::new("https://example.com", "goal", PathBuf::from("/tmp/cu-x"));
        let err = run_computer_use(
            &spec,
            Some(Path::new("/nonexistent/computer_use.mjs")),
            Some("node"),
        )
        .unwrap_err();
        assert!(matches!(err, ComputerUseError::Run { .. }));
    }
}
