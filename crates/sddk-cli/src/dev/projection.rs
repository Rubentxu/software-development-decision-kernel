//! `dev projection` — projection rebuild and inspection tooling.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::{CliEnvironment, CommandOutput};

/// Projection management subcommands.
#[derive(Debug, Args)]
pub(crate) struct ProjectionArgs {
    #[clap(subcommand)]
    pub sub: ProjectionSub,
}

#[derive(Debug, Subcommand)]
pub(crate) enum ProjectionSub {
    /// Rebuild a projection from the event ledger.
    Rebuild {
        /// Projection name (currently only `cycle_state` is supported).
        name: String,
        /// Stream ID — for `cycle_state`, pass the cycle_id.
        #[clap(long, value_name = "STREAM_ID")]
        stream_id: String,
        /// Optional starting sequence (resume from this sequence).
        #[clap(long, value_name = "SEQ")]
        from_sequence: Option<u64>,
        /// Ledger directory containing `ledger.sqlite`.
        /// Defaults to `$XDG_STATE_HOME/sddk` (or `$HOME/.local/state/sddk`).
        #[clap(long, value_name = "DIR")]
        ledger_dir: Option<PathBuf>,
    },
}

/// Resolves the default ledger directory from the environment.
fn default_ledger_dir(env: &CliEnvironment) -> PathBuf {
    let state = env
        .state_home
        .clone()
        .or_else(|| env.home.clone().map(|h| h.join(".local/state")));
    state
        .unwrap_or_else(|| dirs::state_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("sddk")
}

pub(super) fn run_dev_projection(args: &ProjectionArgs, env: &CliEnvironment) -> CommandOutput {
    match &args.sub {
        ProjectionSub::Rebuild {
            name,
            stream_id,
            from_sequence,
            ledger_dir,
        } => {
            if name != "cycle_state" {
                return CommandOutput {
                    status: 1,
                    stdout: String::new(),
                    stderr: format!("error: unknown projection '{name}'; supported: cycle_state\n"),
                };
            }

            let ledger = ledger_dir
                .clone()
                .unwrap_or_else(|| default_ledger_dir(env));

            let event_store = match sddk_storage::SqliteEventStore::open(&ledger) {
                Ok(s) => s,
                Err(e) => {
                    return CommandOutput {
                        status: 1,
                        stdout: String::new(),
                        stderr: format!("error: open event store: {e}\n"),
                    };
                }
            };

            let mut proj_store = match sddk_storage::SqliteProjectionStore::open(&ledger) {
                Ok(s) => s,
                Err(e) => {
                    return CommandOutput {
                        status: 1,
                        stdout: String::new(),
                        stderr: format!("error: open projection store: {e}\n"),
                    };
                }
            };

            let id = stream_id.clone();
            let state: sddk_domain::CycleState = match sddk_storage::rebuild(
                &event_store,
                &mut proj_store,
                || sddk_domain::CycleStateProjection::new(&id),
                stream_id,
                *from_sequence,
            ) {
                Ok(s) => s,
                Err(e) => {
                    return CommandOutput {
                        status: 1,
                        stdout: String::new(),
                        stderr: format!("error: rebuild: {e}\n"),
                    };
                }
            };

            CommandOutput {
                status: 0,
                stdout: format!(
                    "rebuilt projection 'cycle_state' from stream '{}'; phase={} seq={}\n",
                    stream_id, state.phase, state.last_event_sequence
                ),
                stderr: String::new(),
            }
        }
    }
}
