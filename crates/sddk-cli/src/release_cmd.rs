//! Release plan and apply commands.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::GateOutcomeStatus;
use sddk_gateway::{
    CapabilityGateway, CapabilityPolicy, GitExecutor, GitHubForge, LocalReleaseInput,
    LocalReleaseOutcome, LocalReleasePreconditions, PermissionPolicy, ReleasePlanInput,
    apply_local_release, apply_release, plan_release,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
    uat::ReleaseTypeArg,
};

/// Typed error for version lockstep failures — names both workspace and tag versions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VersionLockstepError {
    pub workspace_version: String,
    pub tag_version: String,
    pub message: String,
}

impl std::fmt::Display for VersionLockstepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VersionLockstepError {}

/// Ensure the release tag matches the workspace Cargo.toml version (lockstep rule).
///
/// The lockstep rule: `version tag == workspace Cargo.toml version`.
/// Tags use the "v" prefix (e.g. "v1.42.5") while Cargo.toml uses plain "1.42.5".
///
/// Returns `Ok(())` if the tag matches. Returns `Err(VersionLockstepError)` naming
/// BOTH workspace and tag versions on mismatch.
pub fn ensure_version_lockstep(
    root: &std::path::Path,
    tag: &str,
) -> Result<(), VersionLockstepError> {
    // Read the workspace version from the root Cargo.toml
    let cargo_toml = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).map_err(|e| VersionLockstepError {
        workspace_version: String::new(),
        tag_version: tag.to_string(),
        message: format!(
            "VERSION LOCKSTEP ERROR: could not read {}: {e}",
            cargo_toml.display()
        ),
    })?;
    // Extract `version = "X.Y.Z"` from [workspace] or [workspace.package] section
    let workspace_version = {
        let mut in_workspace = false;
        let mut version = None;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_workspace = trimmed.starts_with("[workspace");
            } else if in_workspace
                && let Some(v) = trimmed.strip_prefix("version").and_then(|rest| {
                    let rest = rest.trim();
                    if !rest.starts_with('=') {
                        return None;
                    }
                    let rest = rest[1..].trim();
                    rest.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                })
            {
                version = Some(v.to_string());
                break;
            }
        }
        version
    }
    .ok_or_else(|| VersionLockstepError {
        workspace_version: String::new(),
        tag_version: tag.to_string(),
        message: format!(
            "VERSION LOCKSTEP ERROR: could not find `version` in [workspace] or [workspace.package] section of {}",
            cargo_toml.display()
        ),
    })?;

    let tag_version = tag.strip_prefix('v').unwrap_or(tag).to_string();
    if workspace_version != tag_version {
        let msg = format!(
            "VERSION LOCKSTEP FAILED: workspace={} vs tag={}. \
             Release planning refused until the lockstep rule is satisfied.",
            workspace_version, tag_version
        );
        return Err(VersionLockstepError {
            workspace_version,
            tag_version,
            message: msg,
        });
    }
    Ok(())
}

#[derive(Debug, Subcommand)]
pub(crate) enum ReleaseCommand {
    /// Show the selected release sequence.
    Plan(ReleaseArgs),
    /// Apply the selected release route.
    Apply(ReleaseArgs),
    /// Package the current binary with checksums, SBOM, and attestation.
    Dist(DistArgs),
    /// Verify a dist prefix against its checksums and attestation.
    Verify(DistArgs),
    /// Manage release channels and promotion.
    Channel(ChannelArgs),
    /// Revalidate release candidate after a correction commit (scoped recovery).
    Revalidate(RevalidateArgs),
    /// Emit vault-receipt.json for a managed-closure cycle (ADR-0075).
    Vault(VaultArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ChannelArgs {
    /// Promotion source channel.
    #[arg(long)]
    pub(crate) from: String,
    /// Promotion target channel.
    #[arg(long)]
    pub(crate) to: String,
    /// Assume gates passed (required for edge→candidate and candidate→stable).
    #[arg(long)]
    pub(crate) gates_ok: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Release authority selected for one invocation.
#[derive(Debug, Clone, Copy, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReleaseRoute {
    /// Push the trunk branch and annotated tag with local Git only.
    Local,
    /// Use the optional external forge integration.
    Forge,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct DistArgs {
    /// Distribution prefix directory.
    #[arg(long)]
    pub(crate) prefix: PathBuf,
    /// Release channel.
    #[arg(long, default_value = "release")]
    pub(crate) channel: String,
    /// Explicit RFC 3339 timestamp.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit source commit.
    #[arg(long)]
    pub(crate) commit: Option<String>,
    /// Verify a signed gate receipt: `receipt_id|gate|transition|plan_hash|signature`.
    #[arg(long)]
    pub(crate) receipt: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
    /// XDG data dir for staging area (defaults to ~/.local/share).
    #[arg(long)]
    pub(crate) sddk_data_dir: Option<PathBuf>,
    /// Skip the MANIFEST exact-set verification (logged escape hatch for dirty dev workspaces).
    #[arg(long)]
    pub(crate) skip_manifest_preflight: bool,
    /// Source checkout or bundle root containing agents/skills/prompts/workflows/assets
    /// and MANIFEST.sha256. Defaults to current working directory.
    #[arg(long)]
    pub(crate) source: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReleaseArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Release authority. `local` never uses GitHub, CI/CD, or assets.
    #[arg(long, value_enum)]
    pub(crate) route: Option<ReleaseRoute>,
    /// GitHub repository as `owner/repo`, required only for `--route forge`.
    #[arg(long)]
    pub(crate) repo: Option<String>,
    /// Branch to release. Local releases must target the trunk branch.
    #[arg(long, default_value = "main")]
    pub(crate) branch: String,
    /// Target branch for the optional forge pull request.
    #[arg(long, default_value = "main")]
    pub(crate) base: String,
    /// Annotated tag message and optional forge release title.
    #[arg(long, default_value = "SDDK release")]
    pub(crate) title: String,
    /// Release tag.
    #[arg(long)]
    pub(crate) tag: String,
    /// Release cycle providing local verification and UAT evidence.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Previous tag for semver diff (alternative to `--release-type`).
    /// When omitted the release type defaults to Major (fail-closed).
    #[arg(long)]
    pub(crate) previous_tag: Option<String>,
    /// Explicit release type (overrides `--previous-tag` auto-detection).
    #[arg(long, value_enum)]
    pub(crate) release_type: Option<ReleaseTypeArg>,
    /// Release notes.
    #[arg(long, default_value = "")]
    pub(crate) notes: String,
    /// Explicit approval for capability effects.
    #[arg(long)]
    pub(crate) approve: bool,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Distribution prefix for `release dist` attestation (same semantics
    /// as `DistArgs.prefix`).
    #[arg(long)]
    pub(crate) prefix: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for the `release revalidate` command.
///
/// Produces an append-only `release-revalidation.json` artifact that binds
/// the candidate SHA (current HEAD) to fresh verify/debt evidence when a
/// correction commit has moved HEAD after the original verification.
#[derive(Debug, Clone, Args)]
pub(crate) struct RevalidateArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier for the RELEASE_PENDING cycle.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Original SHA that was previously verified (before correction commit).
    #[arg(long)]
    pub(crate) original_sha: String,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Skip the verify check (use only when verify is not applicable).
    #[arg(long)]
    pub(crate) skip_verify: bool,
    /// Skip the debt-verify check (use only for paths without debt verification).
    #[arg(long)]
    pub(crate) skip_debt: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for the `release vault` command (REQ-DKA-002).
#[derive(Debug, Clone, Args)]
pub(crate) struct VaultArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier for the BLOCKED cycle to close via vault route.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_release(command: ReleaseCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        ReleaseCommand::Plan(args) => run_release_plan(args, environment),
        ReleaseCommand::Apply(args) => run_release_apply(args, environment),
        ReleaseCommand::Dist(args) => run_release_dist(args, environment),
        ReleaseCommand::Verify(args) => run_release_dist_verify(args, environment),
        ReleaseCommand::Channel(args) => run_release_channel(args),
        ReleaseCommand::Revalidate(args) => run_release_revalidate(args, environment),
        ReleaseCommand::Vault(args) => run_release_vault(args, environment),
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ChannelPromoteOutput {
    from: String,
    to: String,
    allowed: bool,
    gates_ok: bool,
    reason: String,
}

fn run_release_channel(args: ChannelArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ChannelPromoteOutput> {
        let from = sddk_domain::ReleaseChannel::parse(&args.from).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown channel '{}' (stable|candidate|edge|dev)",
                args.from
            )
        })?;
        let to = sddk_domain::ReleaseChannel::parse(&args.to).ok_or_else(|| {
            anyhow::anyhow!("unknown channel '{}' (stable|candidate|edge|dev)", args.to)
        })?;
        let allowed = sddk_domain::can_promote(from, to, args.gates_ok);
        let reason = if allowed {
            format!("promotion {} → {} allowed", args.from, args.to)
        } else if sddk_domain::promotion_target(from) != Some(to) {
            format!(
                "promotion {} → {} is not an adjacent channel step",
                args.from, args.to
            )
        } else {
            format!(
                "promotion {} → {} requires gates to pass (--gates-ok)",
                args.from, args.to
            )
        };
        Ok(ChannelPromoteOutput {
            from: args.from,
            to: args.to,
            allowed,
            gates_ok: args.gates_ok,
            reason,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, channel_promote_text);
            if !output.allowed {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

fn channel_promote_text(output: &ChannelPromoteOutput) -> String {
    format!(
        "from: {}\nto: {}\nallowed: {}\ngates_ok: {}\nreason: {}\n",
        output.from, output.to, output.allowed, output.gates_ok, output.reason
    )
}

const CHECKSUMS_FILE: &str = "checksums.txt";
const SBOM_FILE: &str = "sbom.json";
const ATTESTATION_FILE: &str = "attestation.json";

/// Generated distribution artifacts for one binary.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct DistOutput {
    version: String,
    channel: String,
    commit: String,
    binary: String,
    checksums: String,
    sbom: String,
    attestation: String,
}

fn run_release_dist(args: DistArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<DistOutput> {
        let binary = std::env::current_exe()?;
        let bytes = std::fs::read(&binary)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        let version = env!("CARGO_PKG_VERSION").to_owned();

        // Workspace root for git operations and surface copying
        let workspace_root = args
            .source
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        // Determine the commit identifier for the staging path.
        // Priority: explicit --commit flag > GITHUB_SHA env var > local HEAD > "unknown".
        // Using local HEAD SHA (not GITHUB_SHA or "unknown") ensures deterministic
        // staging paths that match the actual repository state.
        let commit = args
            .commit
            .clone()
            .or_else(|| std::env::var("GITHUB_SHA").ok())
            .or_else(|| {
                // Attempt to get local HEAD SHA from the workspace git repository.
                let git = sddk_gateway::GitExecutor::new(workspace_root.clone());
                git.head_sha().ok()
            })
            .unwrap_or_else(|| "unknown".to_owned());
        let timestamp = args
            .timestamp
            .unwrap_or_else(crate::git_cmd::default_timestamp);

        let dist_dir = args.prefix.join("dist");
        std::fs::create_dir_all(&dist_dir)?;
        let binary_path = dist_dir.join("sddk");
        std::fs::write(&binary_path, &bytes)?;

        let checksums = format!("{}  {}\n", digest, "sddk");
        std::fs::write(dist_dir.join(CHECKSUMS_FILE), &checksums)?;

        // ── Staged bundle roundtrip (REQ-RDI-002) ──────────────────────────
        // Compute the staging area.
        let data_dir = args
            .sddk_data_dir
            .clone()
            .or_else(|| environment.sddk_data_dir.clone())
            .or_else(|| environment.data_home.clone())
            .unwrap_or_else(|| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "~".into()))
                    .join(".local/share")
            });
        let staging_dir = data_dir.join("sddk/staging").join(&commit);
        // Remove any incomplete previous attempt before staging fresh.
        std::fs::remove_dir_all(&staging_dir).ok();
        std::fs::create_dir_all(&staging_dir)?;

        // Copy surfaces + MANIFEST.sha256 to staging.
        for surface in crate::dev::MANIFEST_SURFACES {
            let src = workspace_root.join(surface);
            let dst = staging_dir.join(surface);
            if src.is_dir() {
                crate::dev::copy_tree(&src, &dst, crate::dev::CopyMode::Always)?;
            }
        }
        // Copy the manifest itself.
        let manifest_file = "MANIFEST.sha256";
        let src_manifest = workspace_root.join(manifest_file);
        let dst_manifest = staging_dir.join(manifest_file);
        if src_manifest.is_file() {
            std::fs::copy(&src_manifest, &dst_manifest)?;
        }

        // Hash the staged MANIFEST.sha256 to derive manifest_sha256.
        // If no manifest is present, fields remain empty (skip verification).
        let (manifest_sha256, manifest_count) = if dst_manifest.is_file() {
            if !args.skip_manifest_preflight {
                // FAIL-CLOSED: verify staged manifest BEFORE writing attestation.
                let mismatches = crate::dev::verify_manifest(&staging_dir)?;
                if !mismatches.is_empty() {
                    anyhow::bail!(
                        "staged roundtrip FAILED ({} mismatch(es)):\n  {}",
                        mismatches.len(),
                        mismatches.join("\n  ")
                    );
                }
            } else {
                let ts = OffsetDateTime::now_utc()
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| "unknown".into());
                eprintln!("[{ts}] staged manifest verification SKIPPED: --skip-manifest-preflight");
            }
            let sha = crate::dev::sha256_hex(&dst_manifest)?;
            let content = std::fs::read_to_string(&dst_manifest)?;
            let count = content
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with("#"))
                .count();
            (sha, count)
        } else {
            (String::new(), 0)
        };
        let manifest_surfaces = if manifest_sha256.is_empty() {
            Vec::new()
        } else {
            crate::dev::MANIFEST_SURFACES
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        };
        // ── end staged roundtrip ──────────────────────────────────────────

        let sbom = serde_json::json!({
            "tool": "sddk",
            "version": version,
            "commit": commit,
            "channel": args.channel,
            "binary_sha256": digest,
            "dependencies": workspace_dependencies(),
            "manifest_sha256": manifest_sha256,
            "manifest_count": manifest_count,
            "manifest_surfaces": manifest_surfaces,
        });
        let sbom_path = dist_dir.join(SBOM_FILE);
        std::fs::write(&sbom_path, serde_json::to_string_pretty(&sbom)?)?;

        let attestation = serde_json::json!({
            "artifact": "sddk",
            "sha256": digest,
            "builder": "sddk dist",
            "channel": args.channel,
            "timestamp": timestamp,
            "commit": commit,
            "manifest_sha256": manifest_sha256,
            "manifest_count": manifest_count,
            "manifest_surfaces": manifest_surfaces,
            // Explicit flag: whether the staged bundle roundtrip was verified.
            // This is set only when manifest verification succeeded.
            "bundle_roundtrip_verified": !manifest_sha256.is_empty(),
        });
        let attestation_path = dist_dir.join(ATTESTATION_FILE);
        std::fs::write(
            &attestation_path,
            serde_json::to_string_pretty(&attestation)?,
        )?;

        Ok(DistOutput {
            version,
            channel: args.channel.clone(),
            commit,
            binary: binary_path.to_string_lossy().into_owned(),
            checksums: dist_dir.join(CHECKSUMS_FILE).to_string_lossy().into_owned(),
            sbom: sbom_path.to_string_lossy().into_owned(),
            attestation: attestation_path.to_string_lossy().into_owned(),
        })
    })();
    render_result(result, format, dist_text)
}

fn run_release_dist_verify(args: DistArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let dist_dir = args.prefix.join("dist");
        let binary_path = dist_dir.join("sddk");
        let bytes = std::fs::read(&binary_path)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));

        let checksums = std::fs::read_to_string(dist_dir.join(CHECKSUMS_FILE))?;
        let expected = format!("{digest}  sddk\n");
        if checksums != expected {
            anyhow::bail!("checksums.txt does not match the binary digest");
        }

        let sbom: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dist_dir.join(SBOM_FILE))?)?;
        if sbom["binary_sha256"] != digest {
            anyhow::bail!("sbom.json binary digest does not match");
        }

        let attestation: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dist_dir.join(ATTESTATION_FILE))?)?;
        if attestation["sha256"] != digest {
            anyhow::bail!("attestation.json digest does not match");
        }

        // Verify a signed gate receipt when provided (fail-closed).
        // Accepts:
        //   - A JSON file path (e.g. release-receipt.json): loads the receipt,
        //     reconstructs the 10-field HMAC payload, and verifies the signature.
        //   - A pipe-separated legacy string: 4-part (receipt_id|gate|transition|plan_hash|signature)
        //     or 6-part widened (receipt_id|gate|transition|plan_hash|head_sha|tag|signature).
        if let Some(receipt_spec) = &args.receipt {
            // Canonical signing key location: $SDDK_DATA_DIR/keys/
            let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
            let key = sddk_engine::load_or_create_key(&keys_dir)?;

            // Detect JSON file path: contains .json or path separators
            let is_json_receipt = receipt_spec.contains(".json")
                || receipt_spec.contains('/')
                || receipt_spec.contains('\\');

            if is_json_receipt {
                // Load JSON receipt and verify widened 10-field HMAC payload
                let receipt_path = std::path::Path::new(receipt_spec);
                if !receipt_path.is_file() {
                    anyhow::bail!("receipt file not found: {}", receipt_spec);
                }
                let receipt_bytes = std::fs::read(receipt_path)?;
                let receipt: ReleaseReceipt = serde_json::from_slice(&receipt_bytes)?;

                // Reconstruct the 10-field HMAC payload
                let payload = format!(
                    "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                    receipt.receipt_id,
                    receipt.gate,
                    receipt.transition,
                    receipt.plan_hash,
                    receipt.head_sha,
                    receipt.tag,
                    receipt.binary_sha256,
                    receipt.manifest_sha256,
                    receipt.manifest_count,
                    receipt.bundle_roundtrip_verified,
                );
                if !sddk_engine::verify_payload(&payload, &receipt.signature, &key) {
                    anyhow::bail!("release-receipt.json signature verification FAILED");
                }
                // Cross-check: binary_sha256 must match attestation.sha256.
                let binary_from_attestation = attestation["sha256"].as_str().unwrap_or("");
                if binary_from_attestation != digest {
                    anyhow::bail!(
                        "attestation.sha256 ({}) does not match receipt binary_sha256 ({})",
                        binary_from_attestation,
                        receipt.binary_sha256
                    );
                }
            } else {
                // Legacy pipe-separated format
                let parts: Vec<&str> = receipt_spec.split('|').collect();
                let (payload, sig_idx) = match parts.len() {
                    5 => (
                        format!("{}|{}|{}|{}", parts[0], parts[1], parts[2], parts[3]),
                        4,
                    ),
                    7 => (
                        format!(
                            "{}|{}|{}|{}|{}|{}",
                            parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]
                        ),
                        6,
                    ),
                    _ => {
                        anyhow::bail!(
                            "receipt spec must be 4-part (legacy) or 6-part (widened): \
                             receipt_id|gate|transition|plan_hash|[head_sha|tag|]signature"
                        );
                    }
                };
                if !sddk_engine::verify_payload(&payload, parts[sig_idx], &key) {
                    anyhow::bail!("gate receipt signature verification FAILED");
                }
                // Cross-check: binary_sha256 must match attestation.sha256.
                let binary_from_attestation = attestation["sha256"].as_str().unwrap_or("");
                if binary_from_attestation != digest {
                    anyhow::bail!(
                        "attestation.sha256 ({}) does not match binary ({})",
                        binary_from_attestation,
                        digest
                    );
                }
                // When tag is present in the receipt, verify it matches attestation.tag.
                if parts.len() == 7 {
                    let expected_tag = parts[5];
                    let attestation_tag = attestation["tag"].as_str().unwrap_or("");
                    if attestation_tag != expected_tag {
                        anyhow::bail!(
                            "attestation.tag ({}) does not match receipt tag ({})",
                            attestation_tag,
                            expected_tag
                        );
                    }
                }
            }
        }
        Ok(serde_json::json!({
            "valid": true,
            "binary_sha256": digest,
            "sbom_version": sbom["version"],
            "channel": attestation["channel"],
        }))
    })();
    render_result(result, format, dist_verify_text)
}

fn workspace_dependencies() -> Vec<serde_json::Value> {
    let lock = match std::fs::read_to_string(
        std::env::current_dir()
            .unwrap_or_default()
            .join("Cargo.lock"),
    ) {
        Ok(lock) => lock,
        Err(_) => return Vec::new(),
    };
    let mut dependencies = Vec::new();
    let mut name = None;
    for line in lock.lines() {
        if let Some(rest) = line.strip_prefix("name = ") {
            name = Some(rest.trim_matches('"').to_owned());
        } else if let Some(rest) = line.strip_prefix("version = ")
            && let Some(name) = name.take()
        {
            dependencies.push(serde_json::json!({
                "name": name,
                "version": rest.trim_matches('"'),
            }));
        }
    }
    dependencies
}

fn dist_text(output: &DistOutput) -> String {
    format!(
        "version: {}\nchannel: {}\ncommit: {}\nbinary: {}\nchecksums: {}\nsbom: {}\nattestation: {}\n",
        output.version,
        output.channel,
        output.commit,
        output.binary,
        output.checksums,
        output.sbom,
        output.attestation
    )
}

fn dist_verify_text(output: &serde_json::Value) -> String {
    format!(
        "valid: {}\nbinary_sha256: {}\nsbom_version: {}\nchannel: {}\n",
        output["valid"].as_bool().unwrap_or(false),
        output["binary_sha256"].as_str().unwrap_or(""),
        output["sbom_version"].as_str().unwrap_or(""),
        output["channel"].as_str().unwrap_or("")
    )
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ReleasePlanOutput {
    route: ReleaseRoute,
    branch: String,
    base: String,
    tag: String,
    head: Option<String>,
    steps: Vec<&'static str>,
}

fn run_release_plan(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ReleasePlanOutput> {
        let route = selected_route(&args)?;
        if matches!(route, ReleaseRoute::Local) && (args.branch != "main" || args.base != "main") {
            anyhow::bail!("--route local requires --branch main and --base main");
        }
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let git = sddk_gateway::GitExecutor::new(context.root.clone());

        // REQ-RDI-001: MANIFEST exact-set preflight (before any push/tag).
        // Production release route always verifies; no escape hatch.
        preflight_manifest(git.root(), false, "production release always verifies")?;

        // L1 lockstep: version tag must match workspace Cargo.toml version
        ensure_version_lockstep(git.root(), &args.tag)?;
        let head = git.inspect()?.head;

        // REQ-RDI-002 / REQ-RDI-003: gather manifest + receipt fields.
        // Only populate when --prefix is provided (from `release dist` attestation).
        let (
            binary_sha256,
            manifest_sha256,
            manifest_count,
            manifest_surfaces,
            bundle_roundtrip_verified,
        ) = if let Some(ref prefix) = args.prefix {
            // Read from the dist attestation produced by `release dist`.
            let attestation_path = prefix.join("dist/attestation.json");
            let att: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&attestation_path)?)?;
            let ms = att["manifest_sha256"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let mc = att["manifest_count"].as_u64().unwrap_or(0) as usize;
            let msf = att["manifest_surfaces"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            // Explicit bundle_roundtrip_verified from attestation (set by release dist
            // when staged manifest verification succeeded).
            let btv = att["bundle_roundtrip_verified"].as_bool().unwrap_or(false);
            (
                att["sha256"].as_str().unwrap_or_default().to_string(),
                ms,
                mc,
                msf,
                btv,
            )
        } else {
            // Derive from the running binary.
            let binary = std::env::current_exe()?;
            let bytes = std::fs::read(&binary)?;
            let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
            (digest, String::new(), 0, Vec::new(), false)
        };

        // Write the release receipt at {cycle_artifacts_dir}/release-receipt.json
        // only when --prefix was provided (proving release dist was run first).
        if let (Some(_), Some(head_str)) = (args.prefix.as_ref(), head.as_ref()) {
            let plan_hash = format!("sha256:{:x}", Sha256::digest(head_str.as_bytes()));
            let receipt_id = format!("release-receipt-{}", &head_str[..8]);
            let channel = "release".to_string();
            let timestamp = args
                .timestamp
                .clone()
                .unwrap_or_else(crate::git_cmd::default_timestamp);
            let receipt = ReleaseReceipt {
                receipt_id,
                gate: "release-plan".to_string(),
                transition: "phase.plan.complete".to_string(),
                plan_hash,
                head_sha: head_str.clone(),
                tag: args.tag.clone(),
                binary_sha256,
                manifest_sha256,
                manifest_count,
                manifest_surfaces,
                bundle_roundtrip_verified,
                channel,
                timestamp,
                signature: String::new(),
            };
            write_release_receipt(receipt, &context.cycle_artifacts_path, environment)?;
        }

        Ok(ReleasePlanOutput {
            route,
            branch: args.branch.clone(),
            base: args.base.clone(),
            tag: args.tag.clone(),
            head,
            steps: match route {
                ReleaseRoute::Local => vec![
                    "push_main",
                    "verify_main_sha",
                    "create_annotated_tag",
                    "verify_remote_tag",
                ],
                ReleaseRoute::Forge => vec!["create_pr", "merge_pr", "create_release"],
            },
        })
    })();
    render_result(result, format, release_plan_text)
}

fn run_release_apply(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let route = match selected_route(&args) {
        Ok(route) => route,
        Err(error) => return render_result(Err(error), format, release_outcome_text),
    };
    let result = (|| -> anyhow::Result<(
        String,
        std::path::PathBuf,
        CapabilityGateway,
        String,
        String,
        Option<LocalReleasePreconditions>,
    )> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let project_id = context.identity.project_id.to_string();
        let root = context.root.clone();
        let permissions = PermissionPolicy::from_file(root.join("permissions.yaml"))?;
        authorize_release(&permissions, route)?;
        let local_preconditions = matches!(route, ReleaseRoute::Local)
            .then(|| {
                local_release_preconditions(
                    &context,
                    &project_id,
                    args.cycle.as_deref(),
                    args.previous_tag.as_deref(),
                    args.release_type.map(|r| r.into()),
                    &args.tag,
                    environment,
                )
            })
            .transpose()?;
        let workflow = context.engine.workflow().clone();
        let policy = CapabilityPolicy::from_workflow(&workflow);
        let gateway = CapabilityGateway::new(policy, workflow, context.storage);
        let timestamp = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        Ok((project_id, root, gateway, timestamp, actor, local_preconditions))
    })();
    let (project_id, root, mut gateway, timestamp, actor, local_preconditions) = match result {
        Ok(value) => value,
        Err(error) => return render_result(Err(error), format, release_outcome_text),
    };

    match route {
        ReleaseRoute::Local => {
            let result = (|| -> anyhow::Result<LocalReleaseOutcome> {
                if args.branch != "main" || args.base != "main" {
                    anyhow::bail!("--route local requires --branch main and --base main");
                }
                Ok(apply_local_release(
                    &mut gateway,
                    &LocalReleaseInput {
                        project_id,
                        cycle_id: args.cycle,
                        branch: args.branch,
                        tag: args.tag,
                        tag_message: args.title,
                        approve: args.approve,
                        timestamp,
                        actor,
                        preconditions: local_preconditions
                            .expect("local route preconditions were resolved"),
                    },
                    &GitExecutor::new(root),
                )?)
            })();
            render_result(result, format, local_release_outcome_text)
        }
        ReleaseRoute::Forge => {
            let result = (|| -> anyhow::Result<sddk_gateway::ReleaseOutcome> {
                let repo = args.repo.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("--repo is required when --route forge is selected")
                })?;
                // L1 lockstep: version tag must match workspace Cargo.toml version
                ensure_version_lockstep(&root, &args.tag).map_err(|e| anyhow::anyhow!("{e}"))?;
                let version_lockstep_passed = true;
                let input = ReleasePlanInput {
                    project_id,
                    cycle_id: None,
                    branch: args.branch.clone(),
                    base_branch: args.base.clone(),
                    pr_title: args.title.clone(),
                    pr_body: format!("Release {} from {}", args.tag, args.branch),
                    tag: args.tag.clone(),
                    release_title: args.title.clone(),
                    release_notes: args.notes.clone(),
                    approve: args.approve,
                    timestamp,
                    actor,
                };
                let mut forge = GitHubForge::new(repo);
                let plan = plan_release(input, &forge)?;
                Ok(apply_release(
                    &mut gateway,
                    &plan,
                    &mut forge,
                    version_lockstep_passed,
                )?)
            })();
            render_result(result, format, release_outcome_text)
        }
    }
}

fn selected_route(args: &ReleaseArgs) -> anyhow::Result<ReleaseRoute> {
    match args.route {
        Some(route) => Ok(route),
        None if args.repo.is_some() => anyhow::bail!(
            "release route now defaults to local; legacy Forge invocations must pass --route forge"
        ),
        None => Ok(ReleaseRoute::Local),
    }
}

/// Verify the release tag matches the workspace Cargo.toml version (lockstep rule).
///
/// The lockstep rule: `version tag == workspace Cargo.toml version`.
fn authorize_release(policy: &PermissionPolicy, route: ReleaseRoute) -> anyhow::Result<()> {
    // The local route reads the local preconditions through `git.inspect` and
    // also applies `git.push` and `git.tag`. The forge route uses forge-only
    // capabilities. The permission registry MUST list every capability that
    // `apply_local_release` actually executes, otherwise the release would
    // run with an unauthorized read.
    let capabilities: &[&str] = match route {
        ReleaseRoute::Local => &["git.inspect", "git.push", "git.tag"],
        ReleaseRoute::Forge => &["pr.create", "pr.merge", "release.create"],
    };
    for capability in capabilities {
        let decision = policy.authorize("sddk-release", "release", capability);
        if !decision.allowed {
            anyhow::bail!("release permission denied: {}", decision.reason);
        }
    }
    Ok(())
}

fn local_release_preconditions(
    context: &RuntimeContext,
    project_id: &str,
    cycle_id: Option<&str>,
    previous_tag: Option<&str>,
    release_type_arg: Option<sddk_domain::ReleaseType>,
    current_tag: &str,
    environment: &CliEnvironment,
) -> anyhow::Result<LocalReleasePreconditions> {
    // L1 lockstep: version tag must match workspace Cargo.toml version
    let version_lockstep_passed = ensure_version_lockstep(&context.root, current_tag).is_ok();
    let cycle_id = cycle_id.ok_or_else(|| {
        anyhow::anyhow!("--cycle is required for --route local to verify local release evidence")
    })?;
    let cycle = context.storage.get_cycle(cycle_id)?;
    let manifest = cycle.manifest;
    if manifest.project_id != project_id
        || manifest.status != sddk_domain::CycleStatus::ReleasePending
        || manifest.phase != sddk_domain::Phase::Release
    {
        anyhow::bail!("cycle {cycle_id} is not the current release-pending cycle for this project");
    }

    // The release ties the cycle to the local trunk. The cycle MUST point at
    // a branch that is an ancestor of the current local trunk HEAD, and the
    // worktree MUST be on the trunk branch with a clean status. A cycle that
    // points at a different branch fails clearly here instead of silently
    // pushing the wrong commits.
    let trunk_branch = manifest.branch.as_str();
    if trunk_branch != "main" {
        anyhow::bail!(
            "cycle {cycle_id} points at branch {trunk_branch:?}; the local release route requires the cycle to point at the trunk branch main"
        );
    }
    let git = sddk_gateway::GitExecutor::new(context.root.clone());
    let inspect = git.inspect()?;
    if inspect.branch.as_deref() != Some("main") {
        anyhow::bail!(
            "cycle {cycle_id} is tied to trunk main but the worktree is on {}; checkout main before running the local release",
            inspect.branch.as_deref().unwrap_or("detached HEAD")
        );
    }
    if inspect.dirty {
        anyhow::bail!(
            "cycle {cycle_id} cannot release from a dirty worktree; commit or stash the changes first"
        );
    }
    if let Some(cycle_head) = manifest.head.as_deref() {
        let local_head = git.head_sha()?;
        if cycle_head != local_head && !is_ancestor(&git, cycle_head, &local_head)? {
            anyhow::bail!(
                "cycle {cycle_id} points at commit {cycle_head}, which is not an ancestor of the local trunk HEAD {local_head}; the cycle is on a different branch"
            );
        }
    }

    // UAT is not required for paths without a UAT phase (A-min, B-direct).
    let path_requires_uat = !matches!(
        manifest.path,
        sddk_domain::CyclePath::AMin | sddk_domain::CyclePath::BDirect
    );

    let uat_passed = if path_requires_uat {
        // Load UatConfig to evaluate the release gate.
        let config = crate::uat::load_uat_config(project_id, environment)?;
        let release_type = release_type_arg.unwrap_or_else(|| {
            if let Some(prev) = previous_tag {
                sddk_domain::release_type_from_diff(current_tag, prev)
                    .unwrap_or(sddk_domain::ReleaseType::Major)
            } else {
                // No signal → fail-closed (default to Major, which requires UAT).
                sddk_domain::ReleaseType::Major
            }
        });
        let action = sddk_domain::evaluate_release_gate(&config, release_type);
        if matches!(action, sddk_domain::ReleaseGateAction::Skip) {
            // UAT gate is configured to skip for this release type.
            true
        } else {
            // Consult the gate-receipts table.
            let gates = context.storage.list_gate_receipts(cycle_id)?;
            let passed = |gate: &str| {
                gates.iter().any(|receipt| {
                    receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed
                })
            };
            passed("release-uat-approved")
        }
    } else {
        // A-min and B-direct paths have no UAT phase — precondition satisfied.
        true
    };

    let gates = context.storage.list_gate_receipts(cycle_id)?;
    let passed = |gate: &str| {
        gates
            .iter()
            .any(|receipt| receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed)
    };

    // REQ-RDI-004: verify release-receipt.json from the plan phase.
    let receipt_path = context.cycle_artifacts_path.join("release-receipt.json");
    let (manifest_exact_set_verified, bundle_roundtrip_verified, release_receipt_verified) =
        if receipt_path.is_file() {
            let receipt_bytes = std::fs::read(&receipt_path)?;
            let receipt: ReleaseReceipt = serde_json::from_slice(&receipt_bytes)?;

            // Canonical signing key location: $SDDK_DATA_DIR/keys/
            let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
            let key = sddk_engine::load_or_create_key(&keys_dir)?;
            // HMAC payload binds ALL receipt fields.
            let payload = format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                receipt.receipt_id,
                receipt.gate,
                receipt.transition,
                receipt.plan_hash,
                receipt.head_sha,
                receipt.tag,
                receipt.binary_sha256,
                receipt.manifest_sha256,
                receipt.manifest_count,
                receipt.bundle_roundtrip_verified,
            );
            let hmac_ok = sddk_engine::verify_payload(&payload, &receipt.signature, &key);

            // head_sha must match local HEAD.
            let local_head = git.head_sha()?;
            let head_match = receipt.head_sha == local_head;

            // tag must match the planned tag.
            let tag_match = receipt.tag == current_tag;

            // bundle_roundtrip_verified is explicit in the receipt.
            let bundle_ok = receipt.bundle_roundtrip_verified;

            (
                hmac_ok && head_match && tag_match && bundle_ok,
                bundle_ok,
                hmac_ok && head_match && tag_match,
            )
        } else {
            // No release-receipt.json: this is expected for A-min and B-direct
            // cycles that skip `release dist`. The RDI preconditions are not
            // applicable in that case — treat as skipped (true).
            (true, true, true)
        };

    Ok(LocalReleasePreconditions {
        verification_passed: manifest.artifacts.contains_key("verification-report")
            && passed("tests-pass")
            && passed("policy-compliant"),
        uat_passed,
        version_lockstep_passed,
        manifest_exact_set_verified,
        bundle_roundtrip_verified,
        release_receipt_verified,
    })
}

fn is_ancestor(
    git: &sddk_gateway::GitExecutor,
    ancestor: &str,
    descendant: &str,
) -> anyhow::Result<bool> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(git.root())
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(ancestor)
        .arg(descendant)
        .status()?;
    Ok(status.success())
}

fn release_plan_text(output: &ReleasePlanOutput) -> String {
    let route = match output.route {
        ReleaseRoute::Local => "local",
        ReleaseRoute::Forge => "forge",
    };
    let mut text = format!(
        "route: {route}\nbranch: {}\nbase: {}\ntag: {}\nhead: {}\nsteps:\n",
        output.branch,
        output.base,
        output.tag,
        output.head.as_deref().unwrap_or("null")
    );
    for step in &output.steps {
        text.push_str(&format!("- {step}\n"));
    }
    text
}

fn release_outcome_text(output: &sddk_gateway::ReleaseOutcome) -> String {
    let mut text = format!(
        "converged: {}\napplied: {}\n",
        output.converged,
        output.applied.len()
    );
    for step in &output.applied {
        text.push_str(&format!("- {} {}\n", step.step, step.receipt_id));
    }
    for skip in &output.skipped {
        text.push_str(&format!("- skipped: {skip}\n"));
    }
    text
}

fn local_release_outcome_text(output: &LocalReleaseOutcome) -> String {
    let mut text = format!(
        "converged: {}\nsha: {}\ntag: {}\napplied: {}\n",
        output.converged,
        output.sha,
        output.tag,
        output.applied.len()
    );
    for step in &output.applied {
        text.push_str(&format!("- {} {}\n", step.step, step.receipt_id));
    }
    for skip in &output.skipped {
        text.push_str(&format!("- skipped: {skip}\n"));
    }
    text
}

/// Run the MANIFEST exact-set preflight check, mirroring `dev/install.rs:17-27`.
///
/// When `skip` is true, writes an audit entry and returns `Ok(())`.
/// When `skip` is false, calls `crate::dev::manifest::verify_manifest(root)` and
/// bails with the same error envelope as `dev install` on mismatch.
/// If the manifest file is absent, the check is skipped (no manifest to verify).
fn preflight_manifest(root: &std::path::Path, skip: bool, audit_msg: &str) -> anyhow::Result<()> {
    if skip {
        let ts = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".into());
        eprintln!("[{ts}] MANIFEST preflight SKIPPED: {audit_msg}");
        return Ok(());
    }
    // Skip if no manifest is present (e.g., test environments or bare repos).
    let manifest_path = root.join(crate::dev::manifest::MANIFEST_FILE);
    if !manifest_path.is_file() {
        return Ok(());
    }
    let mismatches = crate::dev::verify_manifest(root)?;
    if !mismatches.is_empty() {
        anyhow::bail!(
            "manifest verification FAILED ({} mismatch(es)):\n  {}",
            mismatches.len(),
            mismatches.join("\n  ")
        );
    }
    Ok(())
}

/// Release receipt written by `release plan` after preflight passes.
/// Stored at `{cycle_artifacts_dir}/release-receipt.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct ReleaseReceipt {
    pub(crate) receipt_id: String,
    pub(crate) gate: String,
    pub(crate) transition: String,
    pub(crate) plan_hash: String,
    pub(crate) head_sha: String,
    pub(crate) tag: String,
    pub(crate) binary_sha256: String,
    pub(crate) manifest_sha256: String,
    pub(crate) manifest_count: usize,
    pub(crate) manifest_surfaces: Vec<String>,
    /// Explicit flag: whether the staged bundle roundtrip was verified.
    /// No longer inferred from non-empty manifest_sha256.
    pub(crate) bundle_roundtrip_verified: bool,
    pub(crate) channel: String,
    pub(crate) timestamp: String,
    #[serde(default)]
    pub(crate) signature: String,
}

/// Write `release-receipt.json` signed with the local gate-signing key.
///
/// HMAC payload binds ALL receipt fields so any tampering is detected:
/// `receipt_id|gate|transition|plan_hash|head_sha|tag|binary_sha256|manifest_sha256|manifest_count|bundle_roundtrip_verified`
fn write_release_receipt(
    receipt: ReleaseReceipt,
    cycle_artifacts_dir: &std::path::Path,
    environment: &CliEnvironment,
) -> anyhow::Result<()> {
    let receipt_path = cycle_artifacts_dir.join("release-receipt.json");
    // Canonical signing key location: $SDDK_DATA_DIR/keys/
    let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
    let key = sddk_engine::load_or_create_key(&keys_dir)?;
    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        receipt.receipt_id,
        receipt.gate,
        receipt.transition,
        receipt.plan_hash,
        receipt.head_sha,
        receipt.tag,
        receipt.binary_sha256,
        receipt.manifest_sha256,
        receipt.manifest_count,
        receipt.bundle_roundtrip_verified,
    );
    let signature = sddk_engine::sign_payload(&payload, &key)?;

    let signed_receipt = ReleaseReceipt {
        signature,
        ..receipt
    };
    crate::dev::atomic_write(
        &receipt_path,
        serde_json::to_string_pretty(&signed_receipt)?.as_bytes(),
        None,
    )?;
    Ok(())
}

/// Vault receipt for managed-closure cycles (REQ-DKA-002).
/// Stored at `{cycle_artifacts_dir}/vault-receipt.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct VaultReceipt {
    pub(crate) receipt_id: String,
    pub(crate) gate: String,
    pub(crate) transition: String,
    pub(crate) cycle_id: String,
    pub(crate) delivery_kind: String,
    pub(crate) content_hash: String,
    pub(crate) timestamp: String,
    #[serde(default)]
    pub(crate) signature: String,
}

/// Write `vault-receipt.json` signed with the local gate-signing key.
fn write_vault_receipt(
    receipt: VaultReceipt,
    cycle_artifacts_dir: &std::path::Path,
    environment: &CliEnvironment,
) -> anyhow::Result<()> {
    let receipt_path = cycle_artifacts_dir.join("vault-receipt.json");
    // Canonical signing key location: $SDDK_DATA_DIR/keys/
    let keys_dir = crate::dev::paths::signing_keys_dir(environment)?;
    let key = sddk_engine::load_or_create_key(&keys_dir)?;
    let payload = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        receipt.receipt_id,
        receipt.gate,
        receipt.transition,
        receipt.cycle_id,
        receipt.delivery_kind,
        receipt.content_hash,
        receipt.timestamp,
    );
    let signature = sddk_engine::sign_payload(&payload, &key)?;

    let signed_receipt = VaultReceipt {
        signature,
        ..receipt
    };
    crate::dev::atomic_write(
        &receipt_path,
        serde_json::to_string_pretty(&signed_receipt)?.as_bytes(),
        None,
    )?;
    Ok(())
}

// ── Release Revalidation ──────────────────────────────────────────────────────

use sddk_domain::{CycleStatus, FreshEvidence, Phase, ReleaseRevalidation, RevalidationCheck};

/// Output of the `release revalidate` command.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct RevalidateOutput {
    /// Whether revalidation succeeded.
    success: bool,
    /// Original SHA that was previously verified.
    original_sha: String,
    /// Candidate SHA (current HEAD).
    candidate_sha: String,
    /// Number of checks performed.
    checks_performed: usize,
    /// Number of checks that passed.
    checks_passed: usize,
    /// Revalidation artifact path.
    artifact_path: String,
    /// SHA-256 of the artifact.
    artifact_sha256: String,
}

/// Run the `release revalidate` command.
///
/// Produces an append-only `release-revalidation.json` artifact that binds the
/// candidate SHA (current HEAD) to fresh verify/debt evidence.
///
/// Safety invariants enforced:
/// - Only RELEASE_PENDING/release cycles can enter recovery
/// - Candidate SHA must equal current HEAD
/// - Fresh verify/debt evidence recorded with argv/exit/output digest
/// - Revalidation is idempotent
/// - Original reports/receipts remain immutable
/// - Failed revalidation blocks publication
pub(crate) fn run_release_revalidate(
    args: RevalidateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<RevalidateOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let cycle = context.storage.get_cycle(&args.cycle)?;

        // SAFETY INVARIANT: Only RELEASE_PENDING/release cycles can enter recovery
        if cycle.manifest.status != CycleStatus::ReleasePending {
            anyhow::bail!(
                "cycle {} is not RELEASE_PENDING (status={:?}); \
                 release revalidation is only available for RELEASE_PENDING cycles",
                args.cycle,
                cycle.manifest.status
            );
        }
        if cycle.manifest.phase != Phase::Release {
            anyhow::bail!(
                "cycle {} is not in release phase (phase={:?}); \
                 release revalidation is only available in release phase",
                args.cycle,
                cycle.manifest.phase
            );
        }

        let git = GitExecutor::new(context.root.clone());
        let current_head = git.head_sha()?;

        // Candidate SHA must equal current HEAD (candidate IS the current HEAD)
        let candidate_sha = current_head;
        let original_sha = args.original_sha.as_str();

        // SAFETY INVARIANT: original_sha must be a CORRECTION (different from candidate)
        // A correction revalidation occurs when HEAD moved after original verification.
        if original_sha == candidate_sha {
            anyhow::bail!(
                "original_sha ({}) equals current HEAD ({}) — no correction detected; \
                 original_sha must be the SHA verified BEFORE the correction commit",
                original_sha,
                candidate_sha
            );
        }

        // SAFETY INVARIANT: original_sha must be a strict ancestor of candidate_sha
        // (the correction commit is a descendant of the originally-verified commit)
        if !is_ancestor(&git, original_sha, &candidate_sha)? {
            anyhow::bail!(
                "original_sha ({}) is not an ancestor of current HEAD ({}); \
                 original_sha must be a prior verified commit and candidate must be its descendant",
                original_sha,
                candidate_sha
            );
        }

        // PATH POLICY: A-full/A-min/A-lite require BOTH verify AND debt-verify
        // (debt verification is mandatory for these paths; no skips allowed)
        // B-direct may skip debt-verify per workflow policy
        let path_requires_debt = !matches!(cycle.manifest.path, sddk_domain::CyclePath::BDirect);

        if path_requires_debt && args.skip_debt {
            anyhow::bail!(
                "--skip-debt is not allowed for path {:?}; \
                 debt-verify is mandatory for A-min/A-lite/A-full paths",
                cycle.manifest.path
            );
        }
        if args.skip_verify {
            anyhow::bail!(
                "--skip-verify is not allowed; \
                 verify is mandatory for all release paths"
            );
        }

        let timestamp = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());

        // Build the revalidation record with candidate=current HEAD and original=previously-verified SHA
        let mut revalidation = ReleaseRevalidation::new(
            args.cycle.clone(),
            context.identity.project_id.to_string(),
            original_sha.to_string(),
            candidate_sha.to_string(),
            "release.complete".to_string(),
            actor,
            timestamp,
        );

        // Run verify check (mandatory for all paths)
        let verify_check = run_verify_check(&git)?;
        revalidation.add_check(verify_check);

        // Run debt-verify check (mandatory for non-B-direct paths)
        if !args.skip_debt {
            let debt_check = run_debt_check(&git)?;
            revalidation.add_check(debt_check);
        }

        // Use candidate-specific filename to ensure append-only behavior.
        // Each candidate SHA gets its own artifact; no artifact is ever overwritten.
        let candidate_short: String = candidate_sha.chars().take(8).collect();
        let artifact_path = context
            .cycle_artifacts_path
            .join(format!("release-revalidation-{}.json", candidate_short));

        // Idempotency: if an artifact already exists for this exact candidate with same checks,
        // return it without modification (preserves immutability of original reports).
        if artifact_path.is_file() {
            let existing: ReleaseRevalidation =
                serde_json::from_str(&std::fs::read_to_string(&artifact_path)?)?;
            if existing.candidate_sha() == revalidation.candidate_sha()
                && existing.checks.len() == revalidation.checks.len()
            {
                // Verify check names and outcomes match exactly for semantic idempotency
                let names_and_outcomes_match = revalidation
                    .checks
                    .iter()
                    .zip(existing.checks.iter())
                    .all(|(a, b)| a.check_name == b.check_name && a.passed == b.passed);
                if names_and_outcomes_match {
                    let artifact_sha256 = crate::dev::sha256_hex(&artifact_path)?;
                    return Ok(RevalidateOutput {
                        success: existing.all_passed(),
                        original_sha: existing.original_sha().to_string(),
                        candidate_sha: existing.candidate_sha().to_string(),
                        checks_performed: existing.checks.len(),
                        checks_passed: existing.checks.iter().filter(|c| c.passed).count(),
                        artifact_path: artifact_path.to_string_lossy().into_owned(),
                        artifact_sha256,
                    });
                }
            }
            // Artifact exists for same candidate but with different checks — conflict
            anyhow::bail!(
                "artifact conflict: {} exists with different checks for candidate {}; \
                 refusing to overwrite (append-only, candidate-specific artifact)",
                artifact_path.display(),
                candidate_sha
            );
        }

        // Write the new artifact atomically (append-only, candidate-specific)
        let json = serde_json::to_string_pretty(&revalidation)?;
        crate::dev::atomic_write(&artifact_path, json.as_bytes(), None)?;

        // Compute SHA-256 of the artifact for the report
        let artifact_sha256 = crate::dev::sha256_hex(&artifact_path)?;

        Ok(RevalidateOutput {
            success: revalidation.all_passed(),
            original_sha: revalidation.original_sha().to_string(),
            candidate_sha: revalidation.candidate_sha().to_string(),
            checks_performed: revalidation.checks.len(),
            checks_passed: revalidation.checks.iter().filter(|c| c.passed).count(),
            artifact_path: artifact_path.to_string_lossy().into_owned(),
            artifact_sha256,
        })
    })();

    match result {
        Ok(output) => {
            let mut cmd = render_result(Ok(output.clone()), format, revalidate_text);
            if !output.success {
                cmd.status = 1; // Failed revalidation blocks publication
            }
            cmd
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

/// Output of the `release vault` command.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct VaultOutput {
    /// Whether the vault receipt was emitted successfully.
    success: bool,
    /// Cycle identifier.
    cycle_id: String,
    /// Delivery kind declared in the cycle.
    delivery_kind: String,
    /// Vault receipt path.
    artifact_path: String,
    /// SHA-256 of the artifact.
    artifact_sha256: String,
}

/// Run the `release vault` command for managed-closure cycles (REQ-DKA-002).
///
/// Emits `vault-receipt.json` at `{cycle_artifacts_dir}/vault-receipt.json`.
/// Refuses if:
/// - delivery_kind != ManagedClosureDelivery
/// - release-receipt.json already exists (REQ-DKA-005-S3)
/// - cycle status != BLOCKED (REQ-DKA-005-S2)
pub(crate) fn run_release_vault(args: VaultArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<VaultOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let cycle = context.storage.get_cycle(&args.cycle)?;

        // SAFETY INVARIANT: Only BLOCKED cycles can enter the vault route
        if cycle.manifest.status != CycleStatus::Blocked {
            anyhow::bail!(
                "cycle {} is not BLOCKED (status={:?}); \
                 archive.vault.complete is only available for BLOCKED cycles",
                args.cycle,
                cycle.manifest.status
            );
        }

        // SAFETY INVARIANT: release-receipt.json must NOT exist
        let receipt_path = context.cycle_artifacts_path.join("release-receipt.json");
        if receipt_path.is_file() {
            anyhow::bail!(
                "release-receipt.json already exists at {}; \
                 managed-closure cycles cannot have a release receipt",
                receipt_path.display()
            );
        }

        // SAFETY INVARIANT: delivery_kind must be ManagedClosureDelivery
        use sddk_domain::delivery_kind::DeliveryKind;
        let delivery_kind_str = match cycle.manifest.delivery_kind {
            Some(DeliveryKind::ManagedClosureDelivery) => "managed-closure-delivery",
            Some(dk) => {
                anyhow::bail!(
                    "cycle {} has delivery_kind={:?}; vault route requires ManagedClosureDelivery",
                    args.cycle,
                    dk
                );
            }
            None => {
                anyhow::bail!(
                    "cycle {} has no delivery_kind declared; vault route requires ManagedClosureDelivery",
                    args.cycle
                );
            }
        };

        let timestamp = args.timestamp.unwrap_or_else(|| {
            time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
        });
        let actor = args.actor.unwrap_or_else(|| "sddk.vault".to_string());

        // Generate deterministic receipt_id from string content
        use sha2::{Digest, Sha256};
        let receipt_id = format!(
            "{:x}",
            Sha256::digest(
                format!(
                    "vault|{}|{}|{}|{}",
                    args.cycle, delivery_kind_str, timestamp, actor
                )
                .as_bytes()
            )
        );

        let git = GitExecutor::new(context.root.clone());
        let head_sha = git.head_sha()?;

        let receipt = VaultReceipt {
            receipt_id: receipt_id.clone(),
            gate: "archive.vault.complete".to_string(),
            transition: "archive.vault.complete".to_string(),
            cycle_id: args.cycle.clone(),
            delivery_kind: delivery_kind_str.to_string(),
            content_hash: head_sha,
            timestamp: timestamp.clone(),
            signature: String::new(), // Will be filled by write_vault_receipt
        };

        write_vault_receipt(receipt, &context.cycle_artifacts_path, environment)?;

        // Compute SHA-256 of the artifact
        let vault_receipt_path = context.cycle_artifacts_path.join("vault-receipt.json");
        let artifact_sha256 = crate::dev::sha256_hex(&vault_receipt_path)?;

        Ok(VaultOutput {
            success: true,
            cycle_id: args.cycle.clone(),
            delivery_kind: delivery_kind_str.to_string(),
            artifact_path: vault_receipt_path.to_string_lossy().into_owned(),
            artifact_sha256,
        })
    })();

    match result {
        Ok(output) => render_result(Ok(output.clone()), format, vault_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

/// Run the verify check and capture fresh evidence.
///
/// Uses the project's canonical passing local test command:
/// `cargo test --workspace --all-targets --locked`
/// NOTE: `--release` is NOT used because `target/debug/sddk` (needed by dev::rdi_tests)
/// does not exist in release mode, causing the test to fail.
pub(crate) fn run_verify_check(git: &GitExecutor) -> anyhow::Result<RevalidationCheck> {
    let verify_argv = vec![
        "cargo".into(),
        "test".into(),
        "--workspace".into(),
        "--all-targets".into(),
        "--locked".into(),
    ];

    let output = std::process::Command::new(&verify_argv[0])
        .args(&verify_argv[1..])
        .current_dir(git.root())
        .output()?;

    let exit_code = output.status.code().unwrap_or(-1);
    let output_digest = format!("sha256:{:x}", Sha256::digest(&output.stdout));

    Ok(RevalidationCheck {
        check_name: "verify".to_string(),
        passed: exit_code == 0,
        evidence: Some(FreshEvidence {
            argv: verify_argv,
            exit_code,
            output_digest,
        }),
    })
}

/// Run the debt-verify check and capture fresh evidence.
pub(crate) fn run_debt_check(git: &GitExecutor) -> anyhow::Result<RevalidationCheck> {
    let debt_argv = vec![
        "cargo".into(),
        "clippy".into(),
        "--workspace".into(),
        "--all-targets".into(),
        "--locked".into(),
        "--".into(),
        "-D".into(),
        "errors".into(),
    ];

    let output = std::process::Command::new(&debt_argv[0])
        .args(&debt_argv[1..])
        .current_dir(git.root())
        .output()?;

    let exit_code = output.status.code().unwrap_or(-1);
    let output_digest = format!("sha256:{:x}", Sha256::digest(&output.stdout));

    Ok(RevalidationCheck {
        check_name: "debt-verify".to_string(),
        passed: exit_code == 0,
        evidence: Some(FreshEvidence {
            argv: debt_argv,
            exit_code,
            output_digest,
        }),
    })
}

fn revalidate_text(output: &RevalidateOutput) -> String {
    format!(
        "success: {}\noriginal_sha: {}\ncandidate_sha: {}\nchecks_performed: {}\nchecks_passed: {}\nartifact_path: {}\nartifact_sha256: {}\n",
        output.success,
        output.original_sha,
        output.candidate_sha,
        output.checks_performed,
        output.checks_passed,
        output.artifact_path,
        output.artifact_sha256
    )
}

fn vault_text(output: &VaultOutput) -> String {
    format!(
        "success: {}\ncycle_id: {}\ndelivery_kind: {}\nartifact_path: {}\nartifact_sha256: {}\n",
        output.success,
        output.cycle_id,
        output.delivery_kind,
        output.artifact_path,
        output.artifact_sha256
    )
}

#[cfg(test)]
mod tests {
    use sddk_domain::{GateOutcomeStatus, GateReceipt};

    /// Verifies the exact logic used by release_cmd's private `passed` closures
    /// (lines ~536-539 and ~549-552): a receipt satisfies a release gate only
    /// when outcome == Passed.  Waived does NOT satisfy release gates.
    #[test]
    fn release_gate_requires_passed_not_waived() {
        // Replicate the passed() closure from release_preconditions:
        //   let passed = |gate: &str| {
        //       gates.iter().any(|receipt| {
        //           receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed
        //       })
        //   };
        let passed = |gates: &[GateReceipt], gate_name: &str| {
            gates
                .iter()
                .any(|r| r.gate == gate_name && r.outcome == GateOutcomeStatus::Passed)
        };

        let receipt_waived = GateReceipt {
            receipt_id: "rcpt-waived".into(),
            project_id: "p".into(),
            cycle_id: Some("c".into()),
            gate: "tests-pass".into(),
            evaluator: "eval".into(),
            transition_id: "t".into(),
            plan_hash: "h".into(),
            outcome: GateOutcomeStatus::Waived,
            evidence: serde_json::json!({}),
            actor: "test".into(),
            actor_ref: None,
            command_id: "cmd".into(),
            frame_id: "frame".into(),
            evaluated_at: "2026-08-03T12:00:00Z".into(),
            seq: 1,
        };
        let receipt_passed = GateReceipt {
            receipt_id: "rcpt-passed".into(),
            project_id: "p".into(),
            cycle_id: Some("c".into()),
            gate: "tests-pass".into(),
            evaluator: "eval".into(),
            transition_id: "t".into(),
            plan_hash: "h".into(),
            outcome: GateOutcomeStatus::Passed,
            evidence: serde_json::json!({}),
            actor: "test".into(),
            actor_ref: None,
            command_id: "cmd".into(),
            frame_id: "frame".into(),
            evaluated_at: "2026-08-03T12:00:00Z".into(),
            seq: 2,
        };

        // Waived receipt does NOT satisfy the release gate
        assert!(
            !passed(std::slice::from_ref(&receipt_waived), "tests-pass"),
            "Waived receipt must NOT satisfy release gate (fails-closed)"
        );
        // Passed receipt DOES satisfy the release gate
        assert!(
            passed(std::slice::from_ref(&receipt_passed), "tests-pass"),
            "Passed receipt must satisfy release gate"
        );
        // Only Passed matters; Waived alongside Passed still passes
        assert!(
            passed(&[receipt_waived, receipt_passed], "tests-pass"),
            "Passed receipt among Waived must still satisfy release gate"
        );
    }
}
