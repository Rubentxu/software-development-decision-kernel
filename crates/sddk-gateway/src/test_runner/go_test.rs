//! `go test` adapter (Go test family).
//!
//! Family: [`TestFamily::GoTest`](super::TestFamily)
//! `program`: `"go"`
//! `args`: `["test", "-count=1", "-timeout={timeout_ms}ms"]`
//!
//! The `-timeout` value is derived from `req.timeout_ms` (Go accepts `ms` suffix).

use std::path::Path;

use super::{AdapterError, AdapterRequest, ResolvedSpec, TestFamily, TestRunnerAdapter, toolchain};
use crate::runner::RunSpec;

/// `go test` value-type adapter.
#[derive(Debug, Clone, Default)]
pub struct GoTestAdapter;

impl TestRunnerAdapter for GoTestAdapter {
    fn family(&self) -> TestFamily {
        TestFamily::GoTest
    }

    fn build_args(&self, req: &AdapterRequest) -> Vec<String> {
        vec![
            "test".to_owned(),
            "-count=1".to_owned(),
            format!("-timeout={}ms", req.timeout_ms),
        ]
    }

    fn env_addenda(&self) -> &[(&'static str, &'static str)] {
        &[]
    }

    fn resolve(&self, req: &AdapterRequest) -> Result<ResolvedSpec, AdapterError> {
        let program = "go";
        toolchain::accept_direct_program(program).map_err(|_| AdapterError::ToolchainMissing {
            family: TestFamily::GoTest,
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
