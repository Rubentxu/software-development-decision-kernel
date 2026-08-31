//! Developer tooling: environment doctor, gates, and atomic install/verify.

use self::projection::ProjectionArgs;
use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

mod agent_models;
mod check;
mod check_arch;
mod comments_check;
pub(crate) mod common;
mod doctor;
mod editor_adapters;
mod entropy;
mod framework_check;
mod install;
mod link;
pub(crate) mod manifest;
mod models_cmd;
pub(crate) mod paths;
pub(crate) mod projection;
mod reconcile;
mod registry;
mod uninstall;
mod update;
mod use_cmd;
mod verify;

use crate::{CliEnvironment, CommandOutput, OutputFormat};

// Re-exports for use by sibling modules (release_cmd.rs) that need to access
// dev-internal items. These are pub(crate) so only the sddk-cli crate can
// access them, preserving encapsulation while enabling cross-module use.
pub(crate) use common::MANIFEST_SURFACES;
pub(crate) use common::{CopyMode, atomic_write, copy_tree, sha256_hex};
pub(crate) use manifest::verify_manifest;

/// Manifest file name, written at the framework root (and shipped in the
/// release bundle).
// `dead_code` allow: pre-existing API surface retained for future
/// bundle validation; tracked for cleanup in phase2-hygiene-baseline.
#[allow(dead_code)]
pub(super) const MANIFEST_FILE: &str = "MANIFEST.sha256";

/// Persisted installation receipt for side-by-side prefixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct InstallReceipt {
    /// Installed version.
    pub version: String,
    /// Source commit.
    pub commit: String,
    /// SHA-256 of the installed binary.
    pub binary_sha256: String,
    /// Release channel.
    pub channel: String,
    /// Installation timestamp.
    pub installed_at: String,
    /// Binary path relative to the prefix.
    pub binary_path: String,
    /// Whether this install included the full bundle (agents/skills/prompts/assets).
    /// When true, `dev verify` checks installed surfaces against the manifest.
    #[serde(default = "default_bundle_true")]
    pub bundle: bool,
    /// Release tag this install was built from, when known.
    /// Populated by `dev install --release-receipt <path>`.
    #[serde(default)]
    pub tag: Option<String>,
}

fn default_bundle_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum LinkEditor {
    #[value(name = "opencode")]
    OpenCode,
    #[value(name = "zcode")]
    ZCode,
    #[value(name = "claude")]
    Claude,
    #[value(name = "codex")]
    Codex,
    #[value(name = "all")]
    All,
}

#[derive(Debug, Subcommand)]
pub(super) enum DevCommand {
    /// Check the toolchain and environment prerequisites.
    Doctor(DoctorArgs),
    /// Run repository quality gates (fmt, clippy, tests).
    Check(CheckArgs),
    /// Install this binary atomically into a prefix with a receipt.
    Install(InstallArgs),
    /// Verify an installed prefix against its receipt.
    Verify(VerifyArgs),
    /// Remove an installed prefix only when it matches its receipt.
    Uninstall(UninstallArgs),
    /// Symlink the framework assets (agents/skills/prompts/workflows) into an editor.
    Link(LinkArgs),
    /// Select the active framework bundle version (asdf-style `use`).
    Use(UseArgs),
    /// Generate or verify MANIFEST.sha256 — per-file content hashes of the
    /// framework surfaces (agents, skills, prompts, workflows, assets).
    Manifest(ManifestArgs),
    /// Evaluate architecture rules against the live workspace baseline.
    CheckArchitecture(CheckArchitectureArgs),
    /// Install a framework release bundle (download, verify checksum +
    /// internal MANIFEST.sha256, extract). Never touches git — source
    /// checkouts are managed by the developer (`git pull` + `dev link`).
    Update(UpdateArgs),
    /// Projection rebuild and inspection tooling.
    Projection(ProjectionArgs),
    /// Manage agent-models.yaml (list/set/validate) and locate the TUI.
    Models(self::models_cmd::ModelsArgs),
    /// Multidimensional architecture health report (LOC, coupling, fan-in/out).
    Entropy(EntropyArgs),
    /// Reconcile IDE agent configs with bundle sources (dry-run by default).
    Reconcile(self::reconcile::ReconcileArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct ManifestArgs {
    /// Framework root to scan (default: current directory).
    #[arg(long)]
    pub(super) root: Option<std::path::PathBuf>,
    /// Verify an existing manifest instead of generating one.
    #[arg(long)]
    pub(super) verify: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct UseArgs {
    /// Version to activate (installed bundle) or `path:<dir>` for dogfooding.
    #[arg(long, required_unless_present = "show")]
    pub(super) version: Option<String>,
    /// Show the active version without changing it.
    #[arg(long)]
    pub(super) show: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DoctorArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
    /// Strict mode: exit 1 when any surface brevity check reports a file over threshold.
    #[arg(long)]
    pub(super) strict: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CheckArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
    /// Scope the comments gate to files changed since this git ref
    /// (commit SHA, branch, tag). Pre-existing violations are treated as
    /// out-of-scope for the verify-phase gate; they are pre-existing debt
    /// tracked via `sddk-debt-verify`. When unset, scans the whole repo.
    #[arg(long, value_name = "GIT_REF")]
    pub(super) since: Option<String>,
    /// Path to a custom comments-rules.yaml contract. Overrides the
    /// compile-time default. May also be set via the `SDDK_COMMENTS_RULES`
    /// env var (which takes priority over `--rules`).
    #[arg(long, value_name = "PATH")]
    pub(super) rules: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct InstallArgs {
    /// Installation prefix directory.
    #[arg(long)]
    pub(super) prefix: std::path::PathBuf,
    /// Release channel.
    #[arg(long, default_value = "dev")]
    pub(super) channel: String,
    /// Explicit RFC 3339 timestamp.
    #[arg(long)]
    pub(super) timestamp: Option<String>,
    /// Explicit source commit.
    #[arg(long)]
    pub(super) commit: Option<String>,
    /// Source checkout or bundle root containing agents/skills/prompts/workflows/assets
    /// and MANIFEST.sha256.
    #[arg(long)]
    pub(super) source: Option<std::path::PathBuf>,
    /// Populate the `tag` field of the install receipt from this
    /// `release-receipt.json` path (opt-in; absent = tag stays None).
    #[arg(long)]
    pub(super) release_receipt: Option<std::path::PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct VerifyArgs {
    /// Installation prefix directory.
    #[arg(long)]
    pub(super) prefix: std::path::PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct UninstallArgs {
    /// Installation prefix directory (optional when removing editor assets only).
    #[arg(long)]
    pub(super) prefix: Option<std::path::PathBuf>,
    /// Also remove framework assets from an editor (opencode|zcode|all).
    #[arg(long, value_enum)]
    pub(super) editor: Option<LinkEditor>,
    /// Repository root (required with --editor).
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Override the OpenCode config dir.
    #[arg(long)]
    pub(super) opencode_dir: Option<std::path::PathBuf>,
    /// Override the ZCode dir.
    #[arg(long)]
    pub(super) zcode_dir: Option<std::path::PathBuf>,
    /// Override the Claude Code dir.
    #[arg(long)]
    pub(super) claude_dir: Option<std::path::PathBuf>,
    /// Override the Codex dir.
    #[arg(long)]
    pub(super) codex_dir: Option<std::path::PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct LinkArgs {
    /// Repository root containing agents/skills/prompts.
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Target editor(s).
    #[arg(long, value_enum, default_value_t = LinkEditor::All)]
    pub(super) editor: LinkEditor,
    /// Override the OpenCode config dir.
    #[arg(long)]
    pub(super) opencode_dir: Option<std::path::PathBuf>,
    /// Override the ZCode dir.
    #[arg(long)]
    pub(super) zcode_dir: Option<std::path::PathBuf>,
    /// Override the Claude Code dir.
    #[arg(long)]
    pub(super) claude_dir: Option<std::path::PathBuf>,
    /// Override the Codex dir.
    #[arg(long)]
    pub(super) codex_dir: Option<std::path::PathBuf>,
    /// Write an idempotent, deduplicated skill registry to
    /// `$SDDK_DATA_DIR/projects/<project_id>/skill-registry.md`.
    #[arg(long)]
    pub(super) write_registry: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct UpdateArgs {
    /// Framework root containing agents/skills/prompts (bundle install)
    /// or a git checkout (developer install).
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Release version to fetch when the root is not a git checkout.
    #[arg(long)]
    pub(super) version: Option<String>,
    /// GitHub repository (owner/name) providing release assets.
    #[arg(long, default_value = "Rubentxu/software-development-decision-kernel")]
    pub(super) repo: String,
    /// Release base URL override (testing with file://).
    #[arg(long)]
    pub(super) base_url: Option<String>,
    /// Target editor(s) to re-link after the update.
    #[arg(long, value_enum, default_value_t = LinkEditor::All)]
    pub(super) editor: LinkEditor,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CheckArchitectureArgs {
    /// Workspace root (default: current directory).
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Path to architecture-rules.yaml.
    #[arg(long)]
    pub(super) rules: Option<std::path::PathBuf>,
    /// Write JSON output to this path.
    #[arg(long)]
    pub(super) out: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
pub(super) enum EntropyFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Args)]
pub(super) struct EntropyArgs {
    /// Workspace root (default: current directory).
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = EntropyFormat::Text)]
    pub(super) format: EntropyFormat,
    /// Exit 1 if any WARN-level issues are found (advisory by default).
    #[arg(long)]
    pub(super) strict: bool,
}

pub(super) fn run_dev(command: DevCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        DevCommand::Doctor(args) => self::doctor::run_dev_doctor(args, environment),
        DevCommand::Check(args) => self::check::run_dev_check(args),
        DevCommand::Install(args) => self::install::run_dev_install(args),
        DevCommand::Verify(args) => self::verify::run_dev_verify(args),
        DevCommand::Uninstall(args) => self::uninstall::run_dev_uninstall(args),
        DevCommand::Link(args) => self::link::run_dev_link(args, environment),
        DevCommand::Use(args) => self::use_cmd::run_dev_use(args, environment),
        DevCommand::Update(args) => self::update::run_dev_update(args, environment),
        DevCommand::Manifest(args) => self::manifest::run_dev_manifest(args),
        DevCommand::CheckArchitecture(args) => self::check_arch::run_check_architecture(args),
        DevCommand::Projection(args) => self::projection::run_dev_projection(&args, environment),
        DevCommand::Models(args) => self::models_cmd::run_dev_models(args, environment),
        DevCommand::Entropy(args) => self::entropy::run_dev_entropy(args, environment),
        DevCommand::Reconcile(args) => self::reconcile::run_dev_reconcile(args, environment),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Smoke tests — verify subcommand entry points do not panic
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod smoke_tests {
    use super::*;

    fn env() -> CliEnvironment {
        CliEnvironment::default()
    }

    #[test]
    fn dev_doctor_does_not_panic() {
        let args = super::DoctorArgs {
            format: OutputFormat::Text,
            strict: false,
        };
        let _ = self::doctor::run_dev_doctor(args, &env());
    }

    #[test]
    fn dev_check_does_not_panic() {
        let args = super::CheckArgs {
            root: std::path::PathBuf::from("."),
            format: OutputFormat::Text,
            since: None,
            rules: None,
        };
        let _ = self::check::run_dev_check(args);
    }

    #[test]
    fn dev_use_show_does_not_panic() {
        let args = super::UseArgs {
            version: None,
            show: true,
            format: OutputFormat::Text,
        };
        let _ = self::use_cmd::run_dev_use(args, &env());
    }

    #[test]
    fn dev_manifest_write_does_not_panic() {
        let tmp = tempfile::tempdir().unwrap();
        let args = super::ManifestArgs {
            root: Some(tmp.path().to_path_buf()),
            verify: false,
            format: OutputFormat::Text,
        };
        let _ = self::manifest::run_dev_manifest(args);
    }

    #[test]
    fn dev_check_architecture_does_not_panic() {
        let args = CheckArchitectureArgs {
            root: std::path::PathBuf::from("."),
            rules: None,
            out: None,
        };
        // Smoke test: just verify it doesn't panic (exit status may be non-zero)
        let _ = self::check_arch::run_check_architecture(args);
    }

    #[test]
    fn dev_models_list_does_not_panic() {
        let args = self::models_cmd::ModelsArgs {
            command: self::models_cmd::ModelsCommand::List(self::models_cmd::ModelsListArgs {
                file: None,
                format: OutputFormat::Text,
            }),
        };
        let _ = self::models_cmd::run_dev_models(args, &env());
    }

    #[test]
    fn dev_models_tui_path_does_not_panic() {
        let args = self::models_cmd::ModelsArgs {
            command: self::models_cmd::ModelsCommand::TuiPath,
        };
        let _ = self::models_cmd::run_dev_models(args, &env());
    }

    #[test]
    fn dev_entropy_does_not_panic() {
        let args = super::EntropyArgs {
            root: std::path::PathBuf::from("."),
            format: super::EntropyFormat::Text,
            strict: false,
        };
        let _ = self::entropy::run_dev_entropy(args, &env());
    }
}

#[cfg(test)]
#[path = "tests/rdi_tests.rs"]
mod rdi_tests;

#[cfg(test)]
#[path = "tests/release_revalidation_tests.rs"]
mod release_revalidation_tests;
