//! EvidenceCollector — normaliza artefactos de ejecución a un
//! `EvidenceBundle` content-addressable (ADR-014 §2.3).
//!
//! El executor (Playwright ahora, ComputerUse en F8) escribe un directorio
//! de evidencia cruda; el collector lee ese directorio, hashea cada payload
//! a `sha256:<hex>` y construye el bundle con `environment` + `execution`.
//! Este adapter es agnóstico del executor: consume el layout de ficheros
//! que el driver escribe.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use sddk_domain::{
    EvidenceArtifact, EvidenceBundle, EvidenceEnvironment, EvidenceExecution, EvidenceKind,
};

/// What the collector knows about the execution context.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceContext {
    /// Executor kind (e.g. `playwright`, `computer_use`).
    pub executor: String,
    /// Git SHA of the application under test, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    /// App version under test, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    /// Browser (e.g. `chromium`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    /// Viewport `WxH`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<String>,
    /// Model that executed (computer-use / agentic runs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// SHA-256 of the model binary (Fara hash), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_hash: Option<String>,
    /// SHA-256 of the prompt/goal used, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_hash: Option<String>,
}

/// Artifact layout entry: a file in the evidence dir plus its semantic kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceFile {
    /// Evidence kind (Screenshot, Trace, Console, Network, Dom, Aria,
    /// Geometry, Video, Trajectory, CommandOutput, File, ...).
    pub kind: EvidenceKind,
    /// Absolute or relative path to the payload file.
    pub path: PathBuf,
    /// Optional mime type.
    pub mime: Option<String>,
    /// Optional human note.
    pub note: Option<String>,
}

/// Failure modes of the collector.
#[derive(Debug, Error)]
pub enum EvidenceCollectorError {
    /// The payload could not be read.
    #[error("cannot read evidence payload {path}: {source}")]
    Read {
        /// Path that failed.
        path: String,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// No payloads were provided.
    #[error("cannot build bundle from zero artifacts")]
    Empty,
}

/// Collects evidence files into a content-addressable bundle.
#[derive(Debug, Clone, Default)]
pub struct EvidenceCollector {
    context: EvidenceContext,
    files: Vec<EvidenceFile>,
}

impl EvidenceCollector {
    /// Collector for the given execution context.
    pub fn new(context: EvidenceContext) -> Self {
        Self {
            context,
            files: Vec::new(),
        }
    }

    /// Adds a payload file to be hashed into the bundle.
    pub fn add(&mut self, file: EvidenceFile) -> &mut Self {
        self.files.push(file);
        self
    }

    /// Builds the bundle, hashing every payload to `sha256:<hex>`.
    pub fn build(&self) -> Result<EvidenceBundle, EvidenceCollectorError> {
        if self.files.is_empty() {
            return Err(EvidenceCollectorError::Empty);
        }
        let mut artifacts = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let bytes =
                std::fs::read(&file.path).map_err(|source| EvidenceCollectorError::Read {
                    path: file.path.display().to_string(),
                    source,
                })?;
            let digest = sddk_domain::sha256_hex(&bytes);
            let size = bytes.len() as u64;
            artifacts.push(EvidenceArtifact {
                kind: file.kind,
                r#ref: digest,
                path: Some(file.path.display().to_string()),
                mime: file.mime.clone(),
                size_bytes: Some(size),
                note: file.note.clone(),
            });
        }
        artifacts.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.r#ref.cmp(&b.r#ref)));
        Ok(EvidenceBundle {
            artifacts,
            environment: EvidenceEnvironment {
                git_sha: self.context.git_sha.clone(),
                app_version: self.context.app_version.clone(),
                browser: self.context.browser.clone(),
                viewport: self.context.viewport.clone(),
                os: Some(std::env::consts::OS.to_owned()),
            },
            execution: EvidenceExecution {
                executor: Some(self.context.executor.clone()),
                model: self.context.model.clone(),
                model_hash: self.context.model_hash.clone(),
                prompt_hash: self.context.prompt_hash.clone(),
            },
        })
    }

    /// Convenience: collect the standard driver layout from an evidence dir.
    /// Scans for the driver's canonical filenames and maps each to the
    /// matching `EvidenceKind`.
    pub fn collect_dir(&mut self, dir: &Path) -> &mut Self {
        use EvidenceKind::*;
        let candidates: &[(EvidenceKind, &str, &str)] = &[
            (Screenshot, "screenshot.png", "image/png"),
            (Trace, "trace.zip", "application/zip"),
            (Console, "console.json", "application/json"),
            (Network, "network.json", "application/json"),
            (Dom, "dom.html", "text/html"),
            (Aria, "aria.json", "application/json"),
            (Geometry, "geometry.json", "application/json"),
            (Video, "video.webm", "video/webm"),
            (Trajectory, "trajectory.json", "application/json"),
            (CommandOutput, "output.log", "text/plain"),
            // HTTP response snapshot (status oracle).
            (Http, "http.json", "application/json"),
        ];
        for (kind, name, mime) in candidates {
            let path = dir.join(name);
            if path.is_file() {
                self.add(EvidenceFile {
                    kind: *kind,
                    path,
                    mime: Some((*mime).to_owned()),
                    note: None,
                });
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_collector_is_error() {
        let collector = EvidenceCollector::new(EvidenceContext::default());
        assert!(matches!(
            collector.build(),
            Err(EvidenceCollectorError::Empty)
        ));
    }

    #[test]
    fn hashes_payload_to_sha256_ref() {
        let dir = std::env::temp_dir().join("sddk-ev-test-1");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("screenshot.png"), b"fake-png-bytes").unwrap();

        let mut collector = EvidenceCollector::new(EvidenceContext {
            executor: "playwright".into(),
            browser: Some("chromium".into()),
            viewport: Some("1366x768".into()),
            ..Default::default()
        });
        collector.collect_dir(&dir);
        let bundle = collector.build().unwrap();

        assert_eq!(bundle.artifacts.len(), 1);
        let artifact = &bundle.artifacts[0];
        assert_eq!(artifact.kind, EvidenceKind::Screenshot);
        assert!(artifact.r#ref.starts_with("sha256:"));
        assert_eq!(artifact.size_bytes, Some(14));
        assert_eq!(bundle.environment.browser.as_deref(), Some("chromium"));
        assert_eq!(bundle.environment.os.as_deref(), Some(std::env::consts::OS));
        assert_eq!(bundle.execution.executor.as_deref(), Some("playwright"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_payload_is_read_error() {
        let mut collector = EvidenceCollector::new(EvidenceContext::default());
        collector.add(EvidenceFile {
            kind: EvidenceKind::Screenshot,
            path: PathBuf::from("/nonexistent/never.png"),
            mime: None,
            note: None,
        });
        assert!(matches!(
            collector.build(),
            Err(EvidenceCollectorError::Read { .. })
        ));
    }
}
