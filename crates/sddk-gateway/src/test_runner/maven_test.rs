//! `maven test` adapter (JVM / Java + Kotlin test family).
//!
//! Family: [`TestFamily::MavenTest`](super::TestFamily)
//! POSIX: `./mvnw` (if executable with trusted shebang) or `mvn` from PATH.
//! Windows: `mvn` only (`.cmd`/`.bat` MUST be rejected).
//! `args`: `["-B", "-q", "test"]`
//! addenda: `MAVEN_OPTS`

use std::path::{Path, PathBuf};

use super::{AdapterError, AdapterRequest, ResolvedSpec, TestFamily, TestRunnerAdapter, toolchain};
use crate::runner::RunSpec;

/// Maven test adapter.
#[derive(Debug, Clone, Default)]
pub struct MavenTestAdapter;

impl MavenTestAdapter {
    /// Attempts to resolve the best maven executable for the current platform.
    fn resolve_program() -> Result<(String, PathBuf), AdapterError> {
        // POSIX: try shebang wrapper first, then system mvn.
        #[cfg(unix)]
        {
            let mvnw = Path::new("./mvnw");
            if let Ok(path) = toolchain::resolve_posix_exec(mvnw) {
                return Ok((path.to_string_lossy().into_owned(), path));
            }
            let mvn = Path::new("mvn");
            toolchain::accept_direct_program("mvn").map_err(|_| {
                AdapterError::ToolchainMissing {
                    family: TestFamily::MavenTest,
                    searched: vec![mvn.to_path_buf(), mvnw.to_path_buf()],
                }
            })?;
            Ok(("mvn".to_owned(), mvn.to_path_buf()))
        }

        // Windows: only `mvn` (no .cmd/.bat wrappers).
        #[cfg(windows)]
        {
            let mvn = Path::new("mvn");
            toolchain::accept_direct_program("mvn").map_err(|_| {
                AdapterError::ToolchainMissing {
                    family: TestFamily::MavenTest,
                    searched: vec![mvn.to_path_buf()],
                }
            })?;
            Ok(("mvn".to_owned(), mvn.to_path_buf()))
        }

        #[cfg(not(any(unix, windows)))]
        {
            let mvn = Path::new("mvn");
            toolchain::accept_direct_program("mvn").map_err(|_| {
                AdapterError::ToolchainMissing {
                    family: TestFamily::MavenTest,
                    searched: vec![mvn.to_path_buf()],
                }
            })?;
            Ok(("mvn".to_owned(), mvn.to_path_buf()))
        }
    }
}

impl TestRunnerAdapter for MavenTestAdapter {
    fn family(&self) -> TestFamily {
        TestFamily::MavenTest
    }

    fn build_args(&self, _req: &AdapterRequest) -> Vec<String> {
        vec!["-B".to_owned(), "-q".to_owned(), "test".to_owned()]
    }

    fn env_addenda(&self) -> &[(&'static str, &'static str)] {
        &[("MAVEN_OPTS", "-Dmaven.test.failure.ignore=false")]
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
