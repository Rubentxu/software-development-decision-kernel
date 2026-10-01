//! Debt management subcommands: report, incs, backfill, gates.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use sddk_domain::{DebtReport, FindingStatus};
use sddk_engine::{self, render_inc_template};
use thiserror::Error;

use crate::{CliEnvironment, CommandOutput};

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("vault path not found: {0}")]
    VaultNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("report parse error: {0}")]
    ReportParse(#[from] serde_json::Error),
}

/// Debt management subcommands.
#[derive(Debug, Clone, Args)]
pub struct DebtArgs {
    #[command(subcommand)]
    pub command: DebtCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DebtCommand {
    /// Write a debt-report.json for the current cycle state.
    Report {
        /// Output path for the debt-report.json file.
        output: PathBuf,
    },
    /// List existing INC files in the project vault.
    Incs,
    /// Backfill INC files from an archived cycle's debt-report.json.
    Backfill {
        /// Cycle ID to backfill INCs from (e.g. p-52b95ef55999f9de/kernel-cycle-7b-durable-debt-runtime).
        cycle_id: String,
    },
    /// Evaluate a named gate against the current debt report.
    Gates {
        /// Gate name to evaluate (e.g. debt-severity-assigned, debt-priority-assigned).
        name: String,
    },
}

/// Runs the debt subcommand.
pub fn run_debt(args: DebtArgs, env: &CliEnvironment) -> CommandOutput {
    match args.command {
        DebtCommand::Report { output } => cmd_report(&output),
        DebtCommand::Incs => cmd_incs(env),
        DebtCommand::Backfill { cycle_id } => cmd_backfill(&cycle_id, env),
        DebtCommand::Gates { name } => cmd_gates(&name),
    }
}

/// Explains that this build has no debt detector, and what it will not do.
///
/// The previous implementation wrote a report naming a hard-coded foreign cycle
/// with no findings, and the gates read that report, so every debt gate in every
/// project answered pass without examining anything. Refusing is the honest
/// outcome: a measurement that never happened is not a passing measurement.
const DEBT_DETECTION_ABSENT: &str = "\
debt detection is not implemented in this build, so there is no report to read and \
no gate to evaluate.

This command previously wrote a report for a hard-coded cycle that did not belong \
to the caller, with no findings, and the debt gates answered PASS over that \
fabricated report. That made the verdict independent of the project it claimed to \
check.

Until detection exists, the gates debt-severity-assigned and \
debt-priority-assigned cannot pass, and the verify phase is blocked by that fact \
rather than by a measurement that never happened.";

fn cmd_report(output: &PathBuf) -> CommandOutput {
    // Nothing is written. A file shaped like evidence but not measured is worse
    // than no file, because the next reader cannot tell the two apart.
    let _ = output;
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!("error: {DEBT_DETECTION_ABSENT}\n"),
    }
}

fn cmd_incs(env: &CliEnvironment) -> CommandOutput {
    let vault = match resolve_vault_path(env) {
        Ok(p) => p,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error resolving vault: {e}\n"),
            };
        }
    };
    let incs_dir = vault.join("incs");
    if !incs_dir.exists() {
        return CommandOutput {
            status: 0,
            stdout: format!("{}\n", incs_dir.display()),
            stderr: String::new(),
        };
    }
    let mut files: Vec<String> = Vec::new();
    match std::fs::read_dir(&incs_dir) {
        Ok(entries) => {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry.file_name().to_str()
                    && entry.file_type().is_ok_and(|ft| ft.is_file())
                {
                    files.push(name.to_string());
                }
            }
        }
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error reading incs dir: {e}\n"),
            };
        }
    }
    files.sort();
    let stdout = if files.is_empty() {
        format!("no INC files found in {}\n", incs_dir.display())
    } else {
        files.join("\n") + "\n"
    };
    CommandOutput {
        status: 0,
        stdout,
        stderr: String::new(),
    }
}

/// Locates the most recent archived debt-report.json whose cycle_id matches.
fn locate_archived_report(vault: &Path, cycle_id: &str) -> Option<PathBuf> {
    let archive_dir = vault.join("archive");
    let Ok(entries) = std::fs::read_dir(&archive_dir) else {
        return None;
    };
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let subdir = entry.path();
        if !subdir.is_dir() {
            continue;
        }
        let report_path = subdir.join("debt-report.json");
        if !report_path.is_file() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&report_path) else {
            continue;
        };
        let Ok(report): Result<DebtReport, _> = serde_json::from_str(&content) else {
            continue;
        };
        if report.cycle_id == cycle_id {
            let modified = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            candidates.push((modified, report_path));
        }
    }
    candidates
        .into_iter()
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

/// Pure logic: emits INC files for non-resolved findings.
fn backfill_report(vault: &Path, report: &DebtReport) -> Result<usize, VaultError> {
    let project_id = vault
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sddk-framework")
        .to_string();
    let incs_dir = vault.join("incs");
    let mut existing_ids: HashSet<String> = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(&incs_dir) {
        for entry in entries.filter_map(Result::ok) {
            if let Some(name) = entry.file_name().to_str()
                && name.ends_with(".md")
            {
                existing_ids.insert(name.trim_end_matches(".md").to_string());
            }
        }
    }
    std::fs::create_dir_all(&incs_dir)?;
    let mut emitted = 0;
    for finding in report.findings.iter().filter(|f| {
        !matches!(
            f.status,
            FindingStatus::Resolved | FindingStatus::Superseded
        )
    }) {
        let inc_content = render_inc_template(finding, &project_id, &report.cycle_id);
        let inc_id = sddk_engine::derive_inc_id(finding, &existing_ids);
        let inc_path = incs_dir.join(format!("{}.md", inc_id));
        std::fs::write(&inc_path, &inc_content)?;
        existing_ids.insert(inc_id);
        emitted += 1;
    }
    Ok(emitted)
}

fn cmd_backfill(cycle_id: &str, env: &CliEnvironment) -> CommandOutput {
    let vault = match resolve_vault_path(env) {
        Ok(p) => p,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error resolving vault: {e}\n"),
            };
        }
    };
    let report_path = match locate_archived_report(&vault, cycle_id) {
        Some(p) => p,
        None => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("no archived debt-report found for cycle {}\n", cycle_id),
            };
        }
    };
    let report_json = match std::fs::read_to_string(&report_path) {
        Ok(c) => c,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error reading {}: {e}\n", report_path.display()),
            };
        }
    };
    let report: DebtReport = match serde_json::from_str(&report_json) {
        Ok(r) => r,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error parsing debt-report.json: {e}\n"),
            };
        }
    };
    match backfill_report(&vault, &report) {
        Ok(emitted) => CommandOutput {
            status: 0,
            stdout: format!(
                "emitted {} INC files to {}\n",
                emitted,
                vault.join("incs").display()
            ),
            stderr: String::new(),
        },
        Err(e) => CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("{e}\n"),
        },
    }
}

fn cmd_gates(gate_name: &str) -> CommandOutput {
    // There is no report to evaluate. Naming an absent measurement is not a
    // verdict, so this refuses rather than answering pass over an empty report.
    let _ = gate_name;
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!("error: {DEBT_DETECTION_ABSENT}\n"),
    }
}

/// Resolves the project vault path: ~/.sddk-knowledge/<project>/
fn resolve_vault_path(env: &CliEnvironment) -> Result<PathBuf, VaultError> {
    let home = env
        .home
        .clone()
        .or_else(dirs::home_dir)
        .ok_or_else(|| VaultError::VaultNotFound("no home directory".into()))?;
    let vault = home.join(".sddk-knowledge/sddk-framework");
    if !vault.exists() {
        std::fs::create_dir_all(&vault)?;
    }
    Ok(vault)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_env(home: &Path) -> CliEnvironment {
        CliEnvironment {
            home: Some(home.to_path_buf()),
            data_home: Some(PathBuf::from("/tmp/sddk-test-data")),
            sddk_data_dir: None,
            framework_dir: None,
            state_home: None,
            sddk_state_home: None,
            cache_home: None,
            sddk_actor: None,
            user: None,
        }
    }

    #[test]
    fn test_resolve_vault_path() {
        let temp = tempfile::tempdir().unwrap();
        let env = test_env(temp.path());
        let vault = resolve_vault_path(&env).unwrap();
        let path = vault.to_string_lossy();
        assert!(
            path.contains(".sddk-knowledge"),
            "vault path should be ~/.sddk-knowledge/sddk-framework: {}",
            path
        );
    }

    #[test]
    fn test_locate_archived_report_finds_cycle() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("archive");
        std::fs::create_dir_all(&archive).unwrap();
        let subdir = archive.join("2026-08-13-cycle-7b-durable-debt-runtime");
        std::fs::create_dir_all(&subdir).unwrap();
        let report: DebtReport = DebtReport {
            schema_version: "1.1.0".into(),
            cycle_id: "p-52b95ef55999f9de/kernel-cycle-7b-durable-debt-runtime".into(),
            generated_at: "2026-08-13T00:00:00Z".into(),
            findings: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        std::fs::write(subdir.join("debt-report.json"), json).unwrap();
        let vault = temp.path().to_path_buf();
        let found = locate_archived_report(
            &vault,
            "p-52b95ef55999f9de/kernel-cycle-7b-durable-debt-runtime",
        );
        assert!(found.is_some(), "should find archived report by cycle_id");
        assert_eq!(
            found.unwrap().file_name().unwrap().to_str().unwrap(),
            "debt-report.json"
        );
    }

    #[test]
    fn un_informe_de_deuda_no_se_fabrica_para_un_ciclo_ajeno() {
        // This test previously asserted the opposite: it required the command to
        // succeed and to write a file. That file named a cycle belonging to
        // another project and held no findings, so the assertion pinned the
        // fabrication in place.
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("debt-report.json");
        let result = cmd_report(&output);

        assert_eq!(
            result.status, 1,
            "without a detector there is nothing to report, so the command must fail closed"
        );
        assert!(
            !output.exists(),
            "a file shaped like evidence but never measured is worse than no file: the \
             next reader cannot tell them apart"
        );
        assert!(
            result.stderr.contains("not implemented"),
            "the refusal must say what is missing, not merely that it failed: {}",
            result.stderr
        );
    }

    #[test]
    fn un_gate_de_deuda_no_responde_pass_sin_medicion() {
        // The gate used to answer PASS over a fabricated empty report, for every
        // gate name, in every project. An unmeasurable gate has no verdict.
        for gate in [
            "debt-severity-assigned",
            "debt-priority-assigned",
            "unknown-gate",
        ] {
            let result = cmd_gates(gate);
            assert_eq!(
                result.status, 1,
                "{gate} must not pass when nothing was measured"
            );
            assert!(
                !result.stdout.contains("PASS"),
                "{gate} emitted a pass verdict with no report behind it: {}",
                result.stdout
            );
            assert!(
                result.stderr.contains("not implemented"),
                "{gate} must name the missing measurement: {}",
                result.stderr
            );
        }
    }
}
