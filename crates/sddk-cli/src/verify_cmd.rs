//! Shadow router for the `verify` facade verb (M6.1).
//!
//! Routes to `ledger verify` then `capability status` to give a single
//! cross-cutting verification command.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at module scope.
//!
#![allow(missing_docs)]

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    capability::{self, CapabilityCommand},
    cycle::RuntimeArgs,
    ledger::{self, LedgerCommand, LedgerVerifyArgs},
};

/// Run the `verify` facade command — delegates to `ledger verify` then
/// `capability status` in sequence.
pub(crate) fn run_verify(format: OutputFormat, environment: &CliEnvironment) -> CommandOutput {
    let runtime = RuntimeArgs {
        root: Some(PathBuf::from(".")),
        scope: Some(".".to_string()),
        remote: None,
        fallback_seed: None,
        no_infer: false,
    };
    // First: ledger verify (sequence continuity + predecessor links + hashes).
    let ledger_output = ledger::run_ledger(
        LedgerCommand::Verify(LedgerVerifyArgs {
            runtime: runtime.clone(),
            format,
        }),
        environment,
    );

    // Second: capability status (default-deny policy snapshot).
    let capability_output = capability::run_capability(
        CapabilityCommand::Status(crate::capability::CapabilityStatusArgs {
            runtime: runtime.clone(),
            format,
        }),
        environment,
    );

    // Stitch the two outputs together; prefer JSON if requested.
    match format {
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "kind": "verify",
                "ledger": ledger_output.stdout,
                "capability": capability_output.stdout,
            });
            let stdout = serde_json::to_string_pretty(&payload).unwrap_or_default();
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str("=== ledger verify ===\n");
            out.push_str(&ledger_output.stdout);
            out.push('\n');
            out.push_str("=== capability status ===\n");
            out.push_str(&capability_output.stdout);
            out.push('\n');
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}
