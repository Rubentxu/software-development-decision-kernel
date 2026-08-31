//! `cargo nextest` adapter (Rust test family).
//!
//! Family: [`TestFamily::CargoNextest`](super::TestFamily)
//! `program`: `"cargo"`
//! `args`: `["--locked", "nextest", "run", "--workspace"]`

use std::path::Path;

use super::{AdapterError, AdapterRequest, ResolvedSpec, TestFamily, TestRunnerAdapter, toolchain};
use crate::runner::RunSpec;

/// `cargo nextest` value-type adapter.
#[derive(Debug, Clone, Default)]
pub struct CargoNextestAdapter;

impl TestRunnerAdapter for CargoNextestAdapter {
    fn family(&self) -> TestFamily {
        TestFamily::CargoNextest
    }

    fn build_args(&self, _req: &AdapterRequest) -> Vec<String> {
        vec![
            "--locked".to_owned(),
            "nextest".to_owned(),
            "run".to_owned(),
            "--workspace".to_owned(),
        ]
    }

    fn env_addenda(&self) -> &[(&'static str, &'static str)] {
        &[]
    }

    fn resolve(&self, req: &AdapterRequest) -> Result<ResolvedSpec, AdapterError> {
        let program = "cargo";
        toolchain::accept_direct_program(program).map_err(|_| AdapterError::ToolchainMissing {
            family: TestFamily::CargoNextest,
            searched: vec![Path::new(program).to_path_buf()],
        })?;

        let spec = RunSpec {
            program: program.to_owned(),
            args: self.build_args(req),
            env: Default::default(), // filled in dispatch()
            timeout_ms: req.timeout_ms,
            output_max_bytes: req.output_max_bytes,
        };

        Ok(ResolvedSpec {
            spec,
            last_candidate: Path::new(program).to_path_buf(),
        })
    }
}
