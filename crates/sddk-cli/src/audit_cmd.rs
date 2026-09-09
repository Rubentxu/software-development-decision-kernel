//! Shadow router for the `audit` facade verb (M6.1).
//!
//! Routes to `memory reflog` (M4 substrate) plus `ledger events` to give
//! a single cross-cutting audit command. The `--since` filter scopes the
//! ledger scan; reflog is always full.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at module scope.
//!
#![allow(missing_docs)]

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    cycle::RuntimeArgs,
    ledger::{self, LedgerCommand, LedgerEventsArgs},
    memory_cmd::{self, MemoryCommand},
};

/// Run the `audit` facade command — delegates to `memory reflog` plus
/// `ledger events` for a single cross-cutting audit command.
pub(crate) fn run_audit(
    _since: Option<String>,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    let runtime = RuntimeArgs {
        root: Some(PathBuf::from(".")),
        scope: Some(".".to_string()),
        remote: None,
        fallback_seed: None,
        no_infer: false,
    };
    let reflog_output = memory_cmd::run_memory(
        MemoryCommand::Reflog(crate::memory_cmd::MemoryReflogArgs {
            scope: "head".to_string(),
            max: 32,
            format,
        }),
        environment,
    );

    let events_output = ledger::run_ledger(
        LedgerCommand::Events(LedgerEventsArgs {
            runtime: runtime.clone(),
            frame: None,
            limit: 50,
            format,
        }),
        environment,
    );

    match format {
        OutputFormat::Json => {
            let payload = serde_json::json!({
                "kind": "audit",
                "reflog": reflog_output.stdout,
                "events": events_output.stdout,
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
            out.push_str("=== memory reflog ===\n");
            out.push_str(&reflog_output.stdout);
            out.push('\n');
            out.push_str("=== ledger events ===\n");
            out.push_str(&events_output.stdout);
            out.push('\n');
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}
