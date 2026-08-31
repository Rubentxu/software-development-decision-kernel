//! `gradle test` adapter (JVM / Java + Kotlin test family).
//!
//! Family: [`TestFamily::GradleTest`](super::TestFamily)
//! POSIX: `./gradlew` (if executable with trusted shebang) or `gradle` from PATH.
//! Windows: `gradle` only (`.cmd`/`.bat` MUST be rejected).
//! `args`: `["--no-daemon", "-q", "test"]`
//! addenda: `GRADLE_OPTS`

use std::path::{Path, PathBuf};

use super::{AdapterError, AdapterRequest, ResolvedSpec, TestFamily, TestRunnerAdapter, toolchain};
use crate::runner::RunSpec;

/// Gradle test adapter.
#[derive(Debug, Clone, Default)]
pub struct GradleTestAdapter;

impl GradleTestAdapter {
    /// Attempts to resolve the best gradle executable for the current platform.
    fn resolve_program() -> Result<(String, PathBuf), AdapterError> {
        // POSIX: try shebang wrapper first, then system gradle.
        #[cfg(unix)]
        {
            let gradlew = Path::new("./gradlew");
            if let Ok(path) = toolchain::resolve_posix_exec(gradlew) {
                return Ok((path.to_string_lossy().into_owned(), path));
            }
            let gradle = Path::new("gradle");
            toolchain::accept_direct_program("gradle").map_err(|_| {
                AdapterError::ToolchainMissing {
                    family: TestFamily::GradleTest,
                    searched: vec![gradle.to_path_buf(), gradlew.to_path_buf()],
                }
            })?;
            Ok(("gradle".to_owned(), gradle.to_path_buf()))
        }

        // Windows: only `gradle` (no .cmd/.bat wrappers).
        #[cfg(windows)]
        {
            let gradle = Path::new("gradle");
            toolchain::accept_direct_program("gradle").map_err(|_| {
                AdapterError::ToolchainMissing {
                    family: TestFamily::GradleTest,
                    searched: vec![gradle.to_path_buf()],
                }
            })?;
            Ok(("gradle".to_owned(), gradle.to_path_buf()))
        }

        #[cfg(not(any(unix, windows)))]
        {
            let gradle = Path::new("gradle");
            toolchain::accept_direct_program("gradle").map_err(|_| {
                AdapterError::ToolchainMissing {
                    family: TestFamily::GradleTest,
                    searched: vec![gradle.to_path_buf()],
                }
            })?;
            Ok(("gradle".to_owned(), gradle.to_path_buf()))
        }
    }
}

impl TestRunnerAdapter for GradleTestAdapter {
    fn family(&self) -> TestFamily {
        TestFamily::GradleTest
    }

    fn build_args(&self, _req: &AdapterRequest) -> Vec<String> {
        vec!["--no-daemon".to_owned(), "-q".to_owned(), "test".to_owned()]
    }

    fn env_addenda(&self) -> &[(&'static str, &'static str)] {
        &[("GRADLE_OPTS", "-Dorg.gradle.daemon=false")]
    }

    fn resolve(&self, req: &AdapterRequest) -> Result<ResolvedSpec, AdapterError> {
        let (program, last_candidate) = Self::resolve_program()?;

        let spec = RunSpec {
            program: program.clone(),
            args: self.build_args(req),
            env: Default::default(),
            timeout_ms: req.timeout_ms,
            output_max_bytes: req.output_max_bytes,
        };

        Ok(ResolvedSpec {
            spec,
            last_candidate,
        })
    }
}
