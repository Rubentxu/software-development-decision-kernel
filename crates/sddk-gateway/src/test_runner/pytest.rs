//! `pytest` adapter (Python test family).
//!
//! Family: [`TestFamily::Pytest`](super::TestFamily)
//! `program`: `"pytest"`
//! `args`: `["--maxfail=1", "-q"]`
//! addenda: `PYTHONDONTWRITEBYTECODE=1`, `PYTHONHASHSEED=0`

use std::path::Path;

use super::{AdapterError, AdapterRequest, ResolvedSpec, TestFamily, TestRunnerAdapter, toolchain};
use crate::runner::RunSpec;

/// `pytest` value-type adapter.
#[derive(Debug, Clone, Default)]
pub struct PytestAdapter;

impl TestRunnerAdapter for PytestAdapter {
    fn family(&self) -> TestFamily {
        TestFamily::Pytest
    }

    fn build_args(&self, _req: &AdapterRequest) -> Vec<String> {
        vec!["--maxfail=1".to_owned(), "-q".to_owned()]
    }

    fn env_addenda(&self) -> &[(&'static str, &'static str)] {
        &[("PYTHONDONTWRITEBYTECODE", "1"), ("PYTHONHASHSEED", "0")]
    }

    fn resolve(&self, req: &AdapterRequest) -> Result<ResolvedSpec, AdapterError> {
        let program = "pytest";
        toolchain::accept_direct_program(program).map_err(|_| AdapterError::ToolchainMissing {
            family: TestFamily::Pytest,
            searched: vec![Path::new(program).to_path_buf()],
        })?;

        let spec = RunSpec {
            program: program.to_owned(),
            args: self.build_args(req),
            env: Default::default(),
            timeout_ms: req.timeout_ms,
            output_max_bytes: req.output_max_bytes,
        };

        Ok(ResolvedSpec {
            spec,
            last_candidate: Path::new(program).to_path_buf(),
        })
    }
}
