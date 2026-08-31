//! `jest` adapter (JavaScript/TypeScript test family).
//!
//! Family: [`TestFamily::Jest`](super::TestFamily)
//! `program`: `"node"`
//! `args`: `["node_modules/jest/bin/jest.js", "--ci", "--runInBand"]`
//! addenda: `&[]` (NODE_OPTIONS is a code-injection vector and must NEVER be forwarded)

use std::path::Path;

use super::{AdapterError, AdapterRequest, ResolvedSpec, TestFamily, TestRunnerAdapter, toolchain};
use crate::runner::RunSpec;

/// `jest` value-type adapter via node (cross-platform, no shell).
#[derive(Debug, Clone, Default)]
pub struct JestAdapter;

impl TestRunnerAdapter for JestAdapter {
    fn family(&self) -> TestFamily {
        TestFamily::Jest
    }

    fn build_args(&self, _req: &AdapterRequest) -> Vec<String> {
        vec![
            "node_modules/jest/bin/jest.js".to_owned(),
            "--ci".to_owned(),
            "--runInBand".to_owned(),
        ]
    }

    /// Jest MUST NOT forward NODE_OPTIONS — it is a code-injection vector via `--require`.
    fn env_addenda(&self) -> &[(&'static str, &'static str)] {
        &[]
    }

    fn resolve(&self, req: &AdapterRequest) -> Result<ResolvedSpec, AdapterError> {
        let program = "node";
        toolchain::accept_direct_program(program).map_err(|_| AdapterError::ToolchainMissing {
            family: TestFamily::Jest,
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
