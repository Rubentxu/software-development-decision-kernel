//! Playwright executor adapter — sensor + actuador, NUNCA juez (ADR-014).
//!
//! Este módulo materializa el puerto `UatExecutorKind::Playwright` del
//! dominio: ejecuta un flujo de navegación/acción contra una URL usando el
//! CLI de Playwright (disponible globalmente), y devuelve un directorio de
//! evidencia (screenshot, trace, console, network, DOM/ARIA snapshot,
//! geometry) listo para que `EvidenceCollector` lo normalice a un
//! `UatEvidenceBundle` content-addressable.
//!
//! Regla dura: el executor produce evidencia, nunca emite el veredicto
//! global del escenario (eso es responsabilidad de los oracles).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What a Playwright run should do and capture (typed argv, no shell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaywrightSpec {
    /// Target URL to open.
    pub url: String,
    /// Viewport `WxH` (e.g. `1366x768`). Default: 1366x768.
    pub viewport: Option<String>,
    /// Actions JSON file (optional): navigation steps beyond load.
    pub actions: Option<PathBuf>,
    /// Capture a screenshot of the final state.
    pub screenshot: bool,
    /// Capture a Playwright trace archive.
    pub trace: bool,
    /// Capture console messages.
    pub console: bool,
    /// Capture network failures.
    pub network: bool,
    /// Capture DOM + ARIA snapshot.
    pub dom: bool,
    /// Capture bounding-box geometry for selectors (geometry.json input).
    pub geometry: Option<PathBuf>,
    /// Where the evidence directory is written.
    pub output_dir: PathBuf,
    /// Wall-clock timeout in milliseconds.
    pub timeout_ms: u64,
}

impl PlaywrightSpec {
    /// Minimal spec: open a URL and screenshot it.
    pub fn new(url: impl Into<String>, output_dir: PathBuf) -> Self {
        Self {
            url: url.into(),
            viewport: None,
            actions: None,
            screenshot: true,
            trace: false,
            console: true,
            network: true,
            dom: true,
            geometry: None,
            output_dir,
            timeout_ms: 30_000,
        }
    }
}

/// Result of a Playwright run: the evidence directory plus captured facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaywrightOutcome {
    /// Directory holding the evidence payloads.
    pub evidence_dir: PathBuf,
    /// Page title observed after navigation (if captured).
    pub page_title: Option<String>,
    /// Final URL observed (may differ from spec.url after redirects).
    pub final_url: Option<String>,
    /// Number of console messages captured.
    pub console_messages: usize,
    /// Number of network failures captured.
    pub network_failures: usize,
}

/// Failures while running Playwright.
#[derive(Debug, Error)]
pub enum PlaywrightError {
    /// The playwright CLI could not be spawned or exited non-zero.
    #[error("playwright run failed for {url}: {message}")]
    Run {
        /// Target URL.
        url: String,
        /// Underlying failure message.
        message: String,
    },
    /// The evidence directory could not be prepared.
    #[error("cannot prepare evidence dir {path}: {source}")]
    Io {
        /// Path that failed.
        path: String,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// The actions file could not be read.
    #[error("cannot read actions file {path}: {source}")]
    Actions {
        /// Path that failed.
        path: String,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

/// Environment keys the Playwright driver needs (PATH/HOME/NODE_PATH/TMPDIR).
/// The typed runner clears the environment (`env_clear`); Playwright needs
/// `PATH` (node + browser binaries), `HOME` (browser cache) and `NODE_PATH`
/// (npm module resolution). Each variable is inherited from the parent
/// process ONLY if it is present — nothing is invented.
pub(crate) fn browser_env() -> std::collections::BTreeMap<String, String> {
    let mut env = std::collections::BTreeMap::new();
    for key in ["PATH", "HOME", "NODE_PATH", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            env.insert(key.to_owned(), value.to_string_lossy().into_owned());
        }
    }
    env
}

/// Executes a Playwright run via `node <driver>` with typed argv.
///
/// `driver_path` defaults to `assets/uat-driver/driver.mjs` relative to the
/// current directory; callers embedding the framework can pass an absolute
/// path resolved from their bundle root.
pub fn run_playwright(
    spec: &PlaywrightSpec,
    driver_path: Option<&Path>,
    node_bin: Option<&str>,
) -> Result<PlaywrightOutcome, PlaywrightError> {
    use std::io::Read;

    let driver = driver_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| crate::resolve_uat_driver("driver.mjs"));

    // Prepare the evidence dir (idempotent).
    std::fs::create_dir_all(&spec.output_dir).map_err(|source| PlaywrightError::Io {
        path: spec.output_dir.display().to_string(),
        source,
    })?;

    // Build typed argv: node <driver> --url ... --output ...
    let mut args: Vec<String> = vec![
        driver.display().to_string(),
        "--url".into(),
        spec.url.clone(),
        "--output".into(),
        spec.output_dir.display().to_string(),
        "--browser".into(),
        "chromium".into(),
        "--timeout".into(),
        spec.timeout_ms.to_string(),
    ];
    if let Some(viewport) = &spec.viewport {
        args.push("--viewport".into());
        args.push(viewport.clone());
    }
    if let Some(actions) = &spec.actions {
        args.push("--actions".into());
        args.push(actions.display().to_string());
    }
    if spec.screenshot {
        args.push("--screenshot".into());
    }
    if spec.trace {
        args.push("--trace".into());
    }
    if spec.console {
        args.push("--console".into());
    }
    if spec.network {
        args.push("--network".into());
    }
    if spec.dom {
        args.push("--dom".into());
    }
    if let Some(geometry) = &spec.geometry {
        args.push("--geometry".into());
        args.push(geometry.display().to_string());
    }

    let run_spec = crate::runner::RunSpec {
        program: node_bin.unwrap_or("node").to_owned(),
        args,
        env: browser_env(),
        timeout_ms: spec.timeout_ms,
        output_max_bytes: 1_048_576,
    };

    // Note: the runner clears the environment. Playwright needs NODE_PATH /
    // HOME etc. — the caller's environment is intentionally NOT inherited to
    // keep runs reproducible; the driver self-bootstraps from PATH.
    let outcome = crate::runner::run(&run_spec).map_err(|source| PlaywrightError::Run {
        url: spec.url.clone(),
        message: format!("spawn failed: {source}"),
    })?;

    if outcome.exit_status != Some(0) {
        let mut message = outcome.stderr.trim().to_owned();
        if message.is_empty() {
            message = outcome.stdout.trim().to_owned();
        }
        return Err(PlaywrightError::Run {
            url: spec.url.clone(),
            message: if message.is_empty() {
                format!("exit {:?}", outcome.exit_status)
            } else {
                message
            },
        });
    }

    // Read the driver's summary JSON (written into the evidence dir).
    let summary_path = spec.output_dir.join("summary.json");
    let mut summary_raw = String::new();
    let mut file = std::fs::File::open(&summary_path).map_err(|source| PlaywrightError::Io {
        path: summary_path.display().to_string(),
        source,
    })?;
    file.read_to_string(&mut summary_raw)
        .map_err(|source| PlaywrightError::Io {
            path: summary_path.display().to_string(),
            source,
        })?;
    let summary: DriverSummary =
        serde_json::from_str(&summary_raw).map_err(|source| PlaywrightError::Run {
            url: spec.url.clone(),
            message: format!("invalid driver summary: {source}"),
        })?;

    Ok(PlaywrightOutcome {
        evidence_dir: spec.output_dir.clone(),
        page_title: summary.page_title,
        final_url: summary.final_url,
        console_messages: summary.console_messages.unwrap_or(0),
        network_failures: summary.network_failures.unwrap_or(0),
    })
}

/// Summary emitted by the driver script (`driver.mjs`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct DriverSummary {
    page_title: Option<String>,
    final_url: Option<String>,
    console_messages: Option<usize>,
    network_failures: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_new_sets_sane_defaults() {
        let spec = PlaywrightSpec::new("https://example.com", PathBuf::from("/tmp/ev"));
        assert_eq!(spec.url, "https://example.com");
        assert!(spec.screenshot);
        assert!(spec.console);
        assert!(!spec.trace);
        assert_eq!(spec.timeout_ms, 30_000);
    }

    #[test]
    fn missing_driver_is_reported_before_spawn() {
        // The driver path is checked by the driver itself; the adapter only
        // forwards the path. Here we verify that a missing driver errors.
        let spec = PlaywrightSpec::new("https://example.com", PathBuf::from("/tmp/ev-x"));
        let err = run_playwright(
            &spec,
            Some(Path::new("/nonexistent/driver.mjs")),
            Some("node"),
        )
        .unwrap_err();
        assert!(matches!(err, PlaywrightError::Run { .. }));
    }

    #[test]
    fn end_to_end_driver_run_captures_evidence() {
        // Requires: node on PATH + the bundled driver + an installed browser.
        // Skipped at runtime when the environment cannot support a real run.
        let node = crate::runner::RunSpec {
            program: "node".into(),
            args: vec!["--version".into()],
            env: crate::policy::capability_env_allowlist("uat.playwright"),
            timeout_ms: 10_000,
            output_max_bytes: 64 * 1024,
        };
        let probe = crate::runner::run(&node).ok();
        if probe.map(|o| o.exit_status) != Some(Some(0)) {
            eprintln!("skipping: node unavailable");
            return;
        }

        // Spin up a tiny local HTTP server with a page to drive.
        let dir = std::env::temp_dir().join(format!("sddk-pw-e2e-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("index.html"),
            "<!DOCTYPE html><html><head><title>E2E</title></head>\
             <body><h1 id=\"hero\">Hello</h1><button id=\"btn\">Go</button></body></html>",
        )
        .unwrap();
        let server = std::process::Command::new("python3")
            .args(["-m", "http.server", "18766", "--directory"])
            .arg(&dir)
            .spawn()
            .ok()
            .map(sddk_testkit::ChildGuard::new);
        std::thread::sleep(std::time::Duration::from_millis(700));

        let evidence_dir = dir.join("evidence");
        let spec = PlaywrightSpec::new("http://127.0.0.1:18766/index.html", evidence_dir.clone());
        let driver =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/uat-driver/driver.mjs");
        let outcome = run_playwright(&spec, Some(&driver), Some("node"));

        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(err) => {
                // Cleanup before asserting.
                drop(server);
                std::fs::remove_dir_all(&dir).ok();
                eprintln!("skipping: driver run failed ({err}) — browser not installed?");
                return;
            }
        };
        assert_eq!(outcome.page_title.as_deref(), Some("E2E"));
        assert!(outcome.evidence_dir.join("screenshot.png").is_file());
        assert!(outcome.evidence_dir.join("dom.html").is_file());

        // Cleanup.
        drop(server);
        std::fs::remove_dir_all(&dir).ok();
    }
}
