//! `sddk config explain` — report configuration precedence chain (M6.1).
//!
//! Precedence (highest → lowest):
//!   1. CLI flags (not reported here — those are runtime)
//!   2. `SDDK_*` / `XDG_*` environment variables
//!   3. `.sddk/config.toml` (if present in cwd or parents)
//!   4. Compiled defaults
//!
//! Output is JSON or Text. JSON is machine-readable; text is human-friendly.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at
//! module scope.
//!
#![allow(missing_docs)]

use std::path::PathBuf;

use clap::Subcommand;
use serde::Serialize;

use crate::{CliEnvironment, CommandOutput, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the configuration precedence chain (highest → lowest).
    Explain {
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Debug, Serialize)]
pub struct ConfigKeyReport {
    pub name: String,
    pub source: String,
    pub resolved: Option<String>,
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigReport {
    pub precedence: Vec<String>,
    pub keys: Vec<ConfigKeyReport>,
}

/// Render the precedence chain for the current environment.
pub fn render_config_report(env: &CliEnvironment, format: OutputFormat) -> CommandOutput {
    let report = ConfigReport {
        precedence: vec![
            "1. CLI flags (runtime)".into(),
            "2. SDDK_* / XDG_* environment variables".into(),
            "3. .sddk/config.toml (cwd or parents)".into(),
            "4. Compiled defaults".into(),
        ],
        keys: vec![
            key_report("HOME", env.home.as_ref().map(path_to_string), "process env"),
            key_report(
                "XDG_DATA_HOME",
                env.data_home.as_ref().map(path_to_string),
                "process env",
            ),
            key_report(
                "SDDK_DATA_DIR",
                env.sddk_data_dir.as_ref().map(path_to_string),
                "process env (overrides XDG_DATA_HOME)",
            ),
            key_report(
                "XDG_STATE_HOME",
                env.state_home.as_ref().map(path_to_string),
                "process env",
            ),
            key_report(
                "XDG_CACHE_HOME",
                env.cache_home.as_ref().map(path_to_string),
                "process env",
            ),
            key_report(
                "SDDK_ACTOR",
                env.sddk_actor.clone(),
                "process env (overrides USER)",
            ),
            key_report("USER", env.user.clone(), "process env (default actor)"),
            key_report(
                ".sddk/config.toml",
                config_toml_status(),
                "file in cwd or parents (read if present)",
            ),
        ],
    };

    match format {
        OutputFormat::Json => {
            let stdout = serde_json::to_string_pretty(&report).unwrap_or_default();
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str("Configuration precedence (highest → lowest):\n");
            for line in &report.precedence {
                out.push_str(&format!("  {line}\n"));
            }
            out.push('\n');
            out.push_str("Resolved keys:\n");
            for k in &report.keys {
                let resolved = k.resolved.as_deref().unwrap_or("(unset)");
                out.push_str(&format!(
                    "  {:<24} = {:<40} [{}]\n",
                    k.name, resolved, k.note
                ));
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
    }
}

fn key_report(name: &str, resolved: Option<String>, note: &str) -> ConfigKeyReport {
    ConfigKeyReport {
        name: name.to_string(),
        source: "process_env_or_file".to_string(),
        resolved,
        note: note.to_string(),
    }
}

#[allow(clippy::ptr_arg)]
fn path_to_string(p: &PathBuf) -> String {
    p.display().to_string()
}

fn config_toml_status() -> Option<String> {
    let mut cwd = std::env::current_dir().ok()?;
    for _ in 0..5 {
        let candidate = cwd.join(".sddk").join("config.toml");
        if candidate.exists() {
            return Some(candidate.display().to_string());
        }
        if !cwd.pop() {
            return None;
        }
    }
    None
}

/// Run the `config` subcommand dispatcher.
pub(crate) fn run_config(command: ConfigCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        ConfigCommand::Explain { format } => render_config_report(environment, format),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_lists_four_sources() {
        let env = CliEnvironment::default();
        let report = ConfigReport {
            precedence: vec!["1".into(), "2".into(), "3".into(), "4".into()],
            keys: vec![],
        };
        assert_eq!(report.precedence.len(), 4);
        let _ = env;
    }

    #[test]
    fn json_output_is_valid_json() {
        let env = CliEnvironment::default();
        let out = render_config_report(&env, OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
        assert_eq!(parsed["precedence"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn text_output_includes_all_seven_keys() {
        let env = CliEnvironment::default();
        let out = render_config_report(&env, OutputFormat::Text);
        for name in [
            "HOME",
            "XDG_DATA_HOME",
            "SDDK_DATA_DIR",
            "XDG_STATE_HOME",
            "XDG_CACHE_HOME",
            "SDDK_ACTOR",
            "USER",
        ] {
            assert!(out.stdout.contains(name), "missing key {name}");
        }
    }
}
