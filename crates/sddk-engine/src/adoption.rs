//! Repairable two-resource project adoption.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sddk_domain::error::SddkErrorCode;
use sddk_domain::{
    AdoptionReceipt, IdentityError, IdentitySource, Ledger, ResolvedProjectIdentity,
    resolve_project_identity, stable_workspace_id,
};
use sddk_domain::{ProjectRecord, StorageError, WorkspaceRecord};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    AdoptionPaths, PathResolutionError, XdgEnvironment, knowledge_vault_path, resolve_xdg_paths,
};

/// Current adoption receipt schema.
pub const ADOPTION_SCHEMA_VERSION: i32 = 2;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Explicit deterministic input for adoption planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionPlanInput {
    /// Raw remote URL, or `None` for fallback identity.
    pub remote_url: Option<String>,
    /// Required monorepo scope.
    pub scope: String,
    /// Stable UUID required when no remote is available.
    pub fallback_seed: Option<String>,
    /// Canonical absolute checkout or worktree path.
    pub canonical_workspace_path: PathBuf,
    /// Human-readable project name.
    pub display_name: String,
    /// Explicit environment values for XDG resolution.
    pub xdg: XdgEnvironment,
    /// SDDK product version.
    pub sddk_version: String,
    /// Runtime implementation version.
    pub runtime_version: String,
    /// Caller-supplied receipt timestamp.
    pub timestamp: String,
    /// Caller-supplied actor.
    pub actor: String,
}

/// Write-free deterministic adoption plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdoptionPlan {
    /// Resolved logical project identity.
    pub identity: ResolvedProjectIdentity,
    /// Stable checkout or worktree identifier.
    pub workspace_id: String,
    /// Canonical absolute checkout path.
    pub canonical_workspace_path: PathBuf,
    /// Resolved XDG paths.
    pub paths: AdoptionPaths,
    /// Canonical external knowledge profile.
    pub knowledge: sddk_domain::KnowledgeProfile,
    /// Receipt that will be written if the plan is applied.
    pub receipt: AdoptionReceipt,
}

/// Observable adoption convergence state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionStatusKind {
    /// Neither receipt nor SQLite registration exists.
    Absent,
    /// Both resources exist and agree.
    Complete,
    /// Only the matching receipt exists, or SQLite registration is incomplete.
    ReceiptOnly,
    /// Only matching SQLite identity data exists.
    LedgerOnly,
    /// Existing valid data disagrees with the requested plan.
    Conflict,
    /// A receipt or database cannot be decoded or verified.
    Corrupt,
}

/// Detailed adoption status returned by apply, status, and repair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdoptionStatus {
    /// Classified convergence state.
    pub status: AdoptionStatusKind,
    /// Expected logical project identifier.
    pub project_id: String,
    /// Expected workspace identifier.
    pub workspace_id: String,
    /// Expected receipt path.
    pub receipt_path: PathBuf,
    /// Expected project database path.
    pub ledger_path: PathBuf,
    /// Existing verified receipt, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<AdoptionReceipt>,
    /// Stable explanation for partial or invalid states.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Errors emitted by adoption planning or convergence.
#[derive(Debug, Error)]
pub enum AdoptionError {
    /// Project identity could not be resolved.
    #[error("adoption identity error: {0}")]
    Identity(#[from] IdentityError),
    /// XDG paths could not be resolved.
    #[error("adoption path error: {0}")]
    Paths(#[from] PathResolutionError),
    /// Adoption filesystem work failed.
    #[error("adoption filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// Receipt serialization failed.
    #[error("adoption receipt serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// SQLite registration failed.
    #[error("adoption storage error: {0}")]
    Storage(#[from] StorageError),
    /// An explicit planning value is empty or invalid.
    #[error("invalid adoption input: {0}")]
    InvalidInput(String),
    /// Apply or repair refused conflicting existing state.
    #[error("adoption state is {status:?}: {detail}")]
    UnsafeState {
        /// Refused state classification.
        status: AdoptionStatusKind,
        /// Reason for refusal.
        detail: String,
    },
    /// Repair was requested for a project with no partial adoption state.
    #[error("adoption is absent; use adopt apply before repair")]
    NothingToRepair,
}

impl SddkErrorCode for AdoptionError {
    fn code(&self) -> &'static str {
        match self {
            Self::Identity(_) => "ADOPTION_IDENTITY",
            Self::Paths(_) => "ADOPTION_PATHS",
            Self::Io(_) => "ADOPTION_IO",
            Self::Serialization(_) => "ADOPTION_SERIALIZATION",
            Self::Storage(_) => "ADOPTION_STORAGE",
            Self::InvalidInput(_) => "ADOPTION_INVALID_INPUT",
            Self::UnsafeState { status, .. } => match status {
                AdoptionStatusKind::Conflict => "ADOPTION_IDENTITY_CONFLICT",
                AdoptionStatusKind::Corrupt => "ADOPTION_RECEIPT_CORRUPT",
                // For Absent/ReceiptOnly/LedgerOnly/Complete, this variant is not
                // reachable in practice; fall through to a generic identity drift code.
                _ => "ADOPTION_IDENTITY_CONFLICT",
            },
            Self::NothingToRepair => "ADOPTION_NOTHING_TO_REPAIR",
        }
    }

    fn recovery(&self) -> &'static str {
        match self {
            Self::UnsafeState {
                status: AdoptionStatusKind::Conflict,
                ..
            } => {
                "if only the CLI version changed, run `sddk adopt refresh`; \
                 for identity drift, inspect the receipt manually"
            }
            Self::UnsafeState {
                status: AdoptionStatusKind::Corrupt,
                ..
            } => {
                "inspect the receipt file (it may have been truncated or \
                 edited) and re-adopt only after backing it up"
            }
            Self::NothingToRepair => {
                "nothing to repair; run `sddk adopt apply` to create the \
                 initial adoption state"
            }
            _ => "inspect the error detail and retry",
        }
    }
}

/// Builds an adoption plan without reading or writing process or filesystem state.
pub fn plan_adoption(input: AdoptionPlanInput) -> Result<AdoptionPlan, AdoptionError> {
    validate_plan_input(&input)?;
    let identity = resolve_project_identity(
        input.remote_url.as_deref(),
        &input.scope,
        input.fallback_seed.as_deref(),
    )?;
    let canonical_workspace_path = path_string(&input.canonical_workspace_path)?;
    let workspace_id = stable_workspace_id(&identity.project_id, &canonical_workspace_path);
    let paths = resolve_xdg_paths(&input.xdg, identity.project_id.as_str(), &workspace_id)?;
    let knowledge = sddk_domain::KnowledgeProfile {
        project_id: identity.project_id.clone(),
        project_name: input.display_name.clone(),
        vault_path: knowledge_vault_path(
            &input.xdg,
            identity.project_id.as_str(),
            &input.display_name,
        )?,
        engram_enabled: false,
    };
    let storage_paths = paths.to_storage_paths(&knowledge.vault_path)?;
    let mut receipt = AdoptionReceipt {
        schema_version: ADOPTION_SCHEMA_VERSION,
        sddk_version: input.sddk_version,
        runtime_version: input.runtime_version,
        project_id: identity.project_id.to_string(),
        workspace_id: workspace_id.clone(),
        display_name: input.display_name,
        canonical_workspace_path,
        identity_source: identity.identity_source,
        remote_url: identity.remote_url.clone(),
        scope: identity.scope.clone(),
        fallback_seed: identity.fallback_seed.clone(),
        configuration_hash: String::new(),
        paths: storage_paths,
        timestamp: input.timestamp,
        actor: input.actor,
    };
    receipt.configuration_hash = configuration_hash(&receipt)?;
    Ok(AdoptionPlan {
        identity,
        workspace_id,
        canonical_workspace_path: input.canonical_workspace_path,
        paths,
        knowledge,
        receipt,
    })
}

/// Inspects receipt and SQLite registration without modifying either resource.
pub fn adoption_status(
    plan: &AdoptionPlan,
    ledger: &impl Ledger,
) -> Result<AdoptionStatus, AdoptionError> {
    let base = base_status(plan);
    let receipt = match inspect_receipt(plan) {
        ReceiptInspection::Absent => None,
        ReceiptInspection::Matching(receipt) => Some(*receipt),
        ReceiptInspection::Conflict(detail) => {
            return Ok(invalid_status(base, AdoptionStatusKind::Conflict, detail));
        }
        ReceiptInspection::Corrupt(detail) => {
            return Ok(invalid_status(base, AdoptionStatusKind::Corrupt, detail));
        }
    };
    let ledger = inspect_ledger(plan, ledger);
    if let Some((status, detail)) = ledger.invalid {
        return Ok(invalid_status(base, status, detail));
    }

    let status = match (receipt.is_some(), ledger.any, ledger.complete) {
        (false, false, false) => AdoptionStatusKind::Absent,
        (true, _, true) => AdoptionStatusKind::Complete,
        (true, _, false) => AdoptionStatusKind::ReceiptOnly,
        (false, true, _) => AdoptionStatusKind::LedgerOnly,
        (false, false, true) => unreachable!("complete registration must contain records"),
    };
    Ok(AdoptionStatus {
        status,
        receipt,
        detail: partial_detail(status),
        ..base
    })
}

/// Applies a plan and converges matching partial state idempotently.
pub fn apply_adoption(
    plan: &AdoptionPlan,
    ledger: &mut impl Ledger,
) -> Result<AdoptionStatus, AdoptionError> {
    let status = adoption_status(plan, ledger)?;
    match status.status {
        AdoptionStatusKind::Complete => {
            converge(plan, ledger)?;
            return require_complete(adoption_status(plan, ledger)?);
        }
        AdoptionStatusKind::Conflict | AdoptionStatusKind::Corrupt => {
            return Err(unsafe_status(status));
        }
        AdoptionStatusKind::Absent
        | AdoptionStatusKind::ReceiptOnly
        | AdoptionStatusKind::LedgerOnly => {}
    }
    converge(plan, ledger)?;
    require_complete(adoption_status(plan, ledger)?)
}

/// Repairs a matching receipt-only or ledger-only adoption state.
pub fn repair_adoption(
    plan: &AdoptionPlan,
    ledger: &mut impl Ledger,
) -> Result<AdoptionStatus, AdoptionError> {
    let status = adoption_status(plan, ledger)?;
    match status.status {
        AdoptionStatusKind::Complete => {
            converge(plan, ledger)?;
            return require_complete(adoption_status(plan, ledger)?);
        }
        AdoptionStatusKind::Absent => return Err(AdoptionError::NothingToRepair),
        AdoptionStatusKind::Conflict | AdoptionStatusKind::Corrupt => {
            return Err(unsafe_status(status));
        }
        AdoptionStatusKind::ReceiptOnly | AdoptionStatusKind::LedgerOnly => {}
    }
    converge(plan, ledger)?;
    require_complete(adoption_status(plan, ledger)?)
}

/// Reads a receipt without interpreting it against a plan.
pub fn read_adoption_receipt(path: impl AsRef<Path>) -> Result<AdoptionReceipt, AdoptionError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn converge(plan: &AdoptionPlan, ledger: &mut impl Ledger) -> Result<(), AdoptionError> {
    fs::create_dir_all(&plan.knowledge.vault_path)?;
    fs::create_dir_all(&plan.paths.artifacts)?;
    fs::create_dir_all(&plan.paths.cache)?;
    ledger.register_project_workspace(&project_record(plan), &workspace_record(plan))?;
    let overwrite = if plan.paths.receipt.exists() {
        let existing = read_adoption_receipt(&plan.paths.receipt)?;
        if !same_identity(&existing, &plan.receipt) {
            return Err(AdoptionError::UnsafeState {
                status: AdoptionStatusKind::Conflict,
                detail: "existing receipt has a different identity".into(),
            });
        }
        true
    } else {
        false
    };
    write_receipt_atomically(&plan.paths.receipt, &plan.receipt, overwrite)?;
    write_knowledge_profile(plan)?;
    Ok(())
}

/// Converges runtime metadata of an existing adoption receipt without
/// overwriting its identity. Refuses on absent, corrupt, or identity-drifted
/// state. Always refreshes the on-disk receipt when the identity matches.
pub fn refresh_adoption(
    plan: &AdoptionPlan,
    ledger: &mut impl Ledger,
) -> Result<AdoptionStatus, AdoptionError> {
    let status = adoption_status(plan, ledger)?;
    match status.status {
        AdoptionStatusKind::Complete | AdoptionStatusKind::ReceiptOnly => {
            let existing = read_adoption_receipt(&plan.paths.receipt)?;
            if same_identity(&existing, &plan.receipt) {
                converge(plan, ledger)?;
                require_complete(adoption_status(plan, ledger)?)
            } else {
                Ok(invalid_status(
                    base_status(plan),
                    AdoptionStatusKind::Conflict,
                    "identity drift detected; refresh only accepts runtime metadata drift".into(),
                ))
            }
        }
        AdoptionStatusKind::Absent
        | AdoptionStatusKind::LedgerOnly
        | AdoptionStatusKind::Conflict
        | AdoptionStatusKind::Corrupt => Ok(status),
    }
}

/// Writes the knowledge profile to `$XDG_DATA_HOME/sddk/projects/{project_id}/knowledge-profile.json`.
fn write_knowledge_profile(plan: &AdoptionPlan) -> Result<(), AdoptionError> {
    if plan.paths.knowledge_profile.exists() {
        let existing: sddk_domain::KnowledgeProfile =
            serde_json::from_slice(&fs::read(&plan.paths.knowledge_profile)?)?;
        if existing.project_id != plan.knowledge.project_id
            || existing.vault_path != plan.knowledge.vault_path
        {
            return Err(AdoptionError::InvalidInput(
                "knowledge profile conflicts with the adoption plan".into(),
            ));
        }
        return Ok(());
    }
    let parent = plan.paths.knowledge_profile.parent().ok_or_else(|| {
        AdoptionError::InvalidInput("knowledge profile has no parent directory".into())
    })?;
    fs::create_dir_all(parent)?;
    fs::write(
        &plan.paths.knowledge_profile,
        serde_json::to_vec_pretty(&plan.knowledge)?,
    )?;
    Ok(())
}

fn inspect_receipt(plan: &AdoptionPlan) -> ReceiptInspection {
    if !plan.paths.receipt.exists() {
        return ReceiptInspection::Absent;
    }
    let bytes = match fs::read(&plan.paths.receipt) {
        Ok(bytes) => bytes,
        Err(error) => return ReceiptInspection::Corrupt(format!("receipt read failed: {error}")),
    };
    let receipt: AdoptionReceipt = match serde_json::from_slice(&bytes) {
        Ok(receipt) => receipt,
        Err(error) => return ReceiptInspection::Corrupt(format!("invalid receipt JSON: {error}")),
    };
    if receipt.schema_version != ADOPTION_SCHEMA_VERSION {
        return ReceiptInspection::Corrupt(format!(
            "unsupported receipt schema version {}",
            receipt.schema_version
        ));
    }
    match configuration_hash(&receipt) {
        Ok(hash) if hash == receipt.configuration_hash => {}
        Ok(_) => {
            return ReceiptInspection::Corrupt(
                "receipt configuration hash does not match its contents".into(),
            );
        }
        Err(error) => return ReceiptInspection::Corrupt(error.to_string()),
    }
    if same_identity(&receipt, &plan.receipt) {
        ReceiptInspection::Matching(Box::new(receipt))
    } else {
        ReceiptInspection::Conflict(
            "receipt identity differs from plan; refresh only accepts runtime metadata drift"
                .into(),
        )
    }
}

fn inspect_ledger(plan: &AdoptionPlan, ledger: &impl Ledger) -> LedgerInspection {
    if !plan.paths.ledger.exists() {
        return LedgerInspection::default();
    }
    let project = match ledger.get_project_optional(plan.identity.project_id.as_str()) {
        Ok(project) => project,
        Err(error) => return LedgerInspection::corrupt(format!("project read failed: {error}")),
    };
    let workspace = match ledger.get_workspace_optional(&plan.workspace_id) {
        Ok(workspace) => workspace,
        Err(error) => {
            return LedgerInspection::corrupt(format!("workspace read failed: {error}"));
        }
    };
    let has_projects = match ledger.has_projects() {
        Ok(has_projects) => has_projects,
        Err(error) => return LedgerInspection::corrupt(format!("ledger read failed: {error}")),
    };
    if project.is_none() && has_projects {
        return LedgerInspection::conflict("ledger belongs to a different project".into());
    }
    if let Some(existing) = &project
        && (existing.remote_url != plan.identity.remote_url
            || existing.scope != plan.identity.scope)
    {
        return LedgerInspection::conflict("ledger project identity differs from plan".into());
    }
    if let Some(existing) = &workspace
        && (existing.project_id != plan.identity.project_id.as_str()
            || existing.canonical_path != plan.receipt.canonical_workspace_path)
    {
        return LedgerInspection::conflict("ledger workspace identity differs from plan".into());
    }
    LedgerInspection {
        any: project.is_some() || workspace.is_some(),
        complete: project.is_some() && workspace.is_some(),
        invalid: None,
    }
}

/// Atomically writes the receipt. When `overwrite` is `true`, an existing
/// receipt at `path` is replaced (used by `converge` after the caller has
/// verified identity matching). When `overwrite` is `false`, the existence
/// of a receipt at `path` produces `AdoptionError::UnsafeState { status:
/// Conflict, .. }` to surface the race-condition guard for unexpected
/// concurrent writes.
fn write_receipt_atomically(
    path: &Path,
    receipt: &AdoptionReceipt,
    overwrite: bool,
) -> Result<(), AdoptionError> {
    let parent = path.parent().ok_or_else(|| {
        AdoptionError::InvalidInput(format!("receipt path has no parent: {path:?}"))
    })?;
    fs::create_dir_all(parent)?;
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(
        ".adoption.json.tmp-{}-{}",
        std::process::id(),
        sequence
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = options.open(&temp)?;
    let mut bytes = serde_json::to_vec_pretty(receipt)?;
    bytes.push(b'\n');
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    drop(file);
    if path.exists() && !overwrite {
        let _ = fs::remove_file(&temp);
        return Err(AdoptionError::UnsafeState {
            status: AdoptionStatusKind::Conflict,
            detail: "receipt appeared during apply; refusing to overwrite it".into(),
        });
    }
    fs::rename(&temp, path)?;
    OpenOptions::new().read(true).open(parent)?.sync_all()?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ConfigurationMaterial<'a> {
    schema_version: i32,
    sddk_version: &'a str,
    runtime_version: &'a str,
    project_id: &'a str,
    workspace_id: &'a str,
    display_name: &'a str,
    canonical_workspace_path: &'a str,
    identity_source: IdentitySource,
    remote_url: &'a Option<String>,
    scope: &'a str,
    fallback_seed: &'a Option<String>,
    paths: &'a sddk_domain::AdoptionStoragePaths,
}

fn configuration_hash(receipt: &AdoptionReceipt) -> Result<String, AdoptionError> {
    let material = ConfigurationMaterial {
        schema_version: receipt.schema_version,
        sddk_version: &receipt.sddk_version,
        runtime_version: &receipt.runtime_version,
        project_id: &receipt.project_id,
        workspace_id: &receipt.workspace_id,
        display_name: &receipt.display_name,
        canonical_workspace_path: &receipt.canonical_workspace_path,
        identity_source: receipt.identity_source,
        remote_url: &receipt.remote_url,
        scope: &receipt.scope,
        fallback_seed: &receipt.fallback_seed,
        paths: &receipt.paths,
    };
    let mut hasher = Sha256::new();
    hasher.update(b"sddk.adoption.configuration.v2\0");
    hasher.update(serde_json::to_vec(&material)?);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// Stable identity comparison: equal iff every immutable field matches.
/// Runtime metadata (`sddk_version`, `runtime_version`, `timestamp`,
/// `actor`, `configuration_hash`) is excluded so that CLI bumps can be
/// refreshed without re-adoption.
///
/// The legacy `paths.vault` compatibility shim (`vault` may live at
/// `$project_data/vault` instead of the canonical `$HOME/.sddk-knowledge/$name`)
/// is preserved to avoid forcing users with an existing legacy receipt to
/// re-adopt; when `left.paths.vault` matches the legacy layout, the right
/// side is normalised to that layout before comparison.
fn same_identity(left: &AdoptionReceipt, right: &AdoptionReceipt) -> bool {
    let mut right_paths = right.paths.clone();
    if let Some(legacy_vault) = legacy_vault_path(left)
        && left.paths.vault == legacy_vault
    {
        right_paths.vault = legacy_vault;
    }
    left.schema_version == right.schema_version
        && left.project_id == right.project_id
        && left.workspace_id == right.workspace_id
        && left.remote_url == right.remote_url
        && left.scope == right.scope
        && left.fallback_seed == right.fallback_seed
        && left.canonical_workspace_path == right.canonical_workspace_path
        && left.paths == right_paths
}

/// Returns the legacy vault path (`$project_data/vault`) inferred from the
/// receipt's `paths.artifacts` parent directory. The `project_data`
/// directory is the parent of `paths.artifacts` in legacy receipts.
fn legacy_vault_path(receipt: &AdoptionReceipt) -> Option<String> {
    Path::new(&receipt.paths.artifacts)
        .parent()
        .and_then(|project_data| project_data.join("vault").to_str().map(str::to_owned))
}

fn validate_plan_input(input: &AdoptionPlanInput) -> Result<(), AdoptionError> {
    for (name, value) in [
        ("display_name", input.display_name.as_str()),
        ("sddk_version", input.sddk_version.as_str()),
        ("runtime_version", input.runtime_version.as_str()),
        ("timestamp", input.timestamp.as_str()),
        ("actor", input.actor.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AdoptionError::InvalidInput(format!(
                "{name} cannot be empty"
            )));
        }
    }
    if !input.canonical_workspace_path.is_absolute() {
        return Err(AdoptionError::InvalidInput(format!(
            "canonical workspace path must be absolute: {:?}",
            input.canonical_workspace_path
        )));
    }
    Ok(())
}

fn project_record(plan: &AdoptionPlan) -> ProjectRecord {
    ProjectRecord {
        project_id: plan.identity.project_id.to_string(),
        display_name: plan.receipt.display_name.clone(),
        remote_url: plan.identity.remote_url.clone(),
        scope: plan.identity.scope.clone(),
        created_at: plan.receipt.timestamp.clone(),
    }
}

fn workspace_record(plan: &AdoptionPlan) -> WorkspaceRecord {
    WorkspaceRecord {
        workspace_id: plan.workspace_id.clone(),
        project_id: plan.identity.project_id.to_string(),
        canonical_path: plan.receipt.canonical_workspace_path.clone(),
        created_at: plan.receipt.timestamp.clone(),
    }
}

fn path_string(path: &Path) -> Result<String, AdoptionError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| AdoptionError::InvalidInput(format!("path is not valid UTF-8: {path:?}")))
}

fn base_status(plan: &AdoptionPlan) -> AdoptionStatus {
    AdoptionStatus {
        status: AdoptionStatusKind::Absent,
        project_id: plan.identity.project_id.to_string(),
        workspace_id: plan.workspace_id.clone(),
        receipt_path: plan.paths.receipt.clone(),
        ledger_path: plan.paths.ledger.clone(),
        receipt: None,
        detail: None,
    }
}

fn invalid_status(
    mut status: AdoptionStatus,
    kind: AdoptionStatusKind,
    detail: String,
) -> AdoptionStatus {
    status.status = kind;
    status.detail = Some(detail);
    status
}

fn partial_detail(status: AdoptionStatusKind) -> Option<String> {
    match status {
        AdoptionStatusKind::Absent => Some("receipt and ledger registration are absent".into()),
        AdoptionStatusKind::ReceiptOnly => Some("ledger registration is incomplete".into()),
        AdoptionStatusKind::LedgerOnly => Some("adoption receipt is absent".into()),
        AdoptionStatusKind::Complete
        | AdoptionStatusKind::Conflict
        | AdoptionStatusKind::Corrupt => None,
    }
}

fn unsafe_status(status: AdoptionStatus) -> AdoptionError {
    AdoptionError::UnsafeState {
        status: status.status,
        detail: status
            .detail
            .unwrap_or_else(|| "existing state cannot be safely converged".into()),
    }
}

fn require_complete(status: AdoptionStatus) -> Result<AdoptionStatus, AdoptionError> {
    if status.status == AdoptionStatusKind::Complete {
        Ok(status)
    } else {
        Err(unsafe_status(status))
    }
}

enum ReceiptInspection {
    Absent,
    Matching(Box<AdoptionReceipt>),
    Conflict(String),
    Corrupt(String),
}

#[derive(Default)]
struct LedgerInspection {
    any: bool,
    complete: bool,
    invalid: Option<(AdoptionStatusKind, String)>,
}

impl LedgerInspection {
    fn conflict(detail: String) -> Self {
        Self {
            invalid: Some((AdoptionStatusKind::Conflict, detail)),
            ..Self::default()
        }
    }

    fn corrupt(detail: String) -> Self {
        Self {
            invalid: Some((AdoptionStatusKind::Corrupt, detail)),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_xdg_vault_legacy_receipt_is_absorbed_by_apply() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        let plan = plan_adoption(AdoptionPlanInput {
            remote_url: Some("https://example.com/acme/repo.git".into()),
            scope: ".".into(),
            fallback_seed: None,
            canonical_workspace_path: root,
            display_name: "repo".into(),
            xdg: XdgEnvironment {
                home: Some(directory.path().join("home")),
                data_home: Some(directory.path().join("data")),
                state_home: Some(directory.path().join("state")),
                cache_home: Some(directory.path().join("cache")),
                ..XdgEnvironment::default()
            },
            sddk_version: "3.6".into(),
            runtime_version: "1.5.3".into(),
            timestamp: "2026-08-10T00:00:00Z".into(),
            actor: "test".into(),
        })
        .unwrap();
        apply_adoption(
            &plan,
            &mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap(),
        )
        .unwrap();

        // Simulate a legacy receipt authored when `paths.vault` lived at
        // `$project_data/vault` instead of the canonical `$HOME/.sddk-knowledge/$name`.
        let mut legacy = plan.receipt.clone();
        legacy.paths.vault = path_string(&plan.paths.project_data.join("vault")).unwrap();
        legacy.configuration_hash = configuration_hash(&legacy).unwrap();
        let legacy_bytes = serde_json::to_vec_pretty(&legacy).unwrap();
        fs::write(&plan.paths.receipt, &legacy_bytes).unwrap();
        fs::remove_file(&plan.paths.knowledge_profile).unwrap();

        // apply must absorb the legacy receipt: status Complete, profile created,
        // and the receipt migrated to the canonical vault (better than today's
        // behaviour where the legacy vault path survived indefinitely).
        assert_eq!(
            apply_adoption(
                &plan,
                &mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap()
            )
            .unwrap()
            .status,
            AdoptionStatusKind::Complete
        );
        assert!(plan.paths.knowledge_profile.is_file());
        let on_disk = read_adoption_receipt(&plan.paths.receipt).unwrap();
        assert_eq!(
            on_disk.paths.vault, plan.receipt.paths.vault,
            "apply must migrate the legacy vault path to the canonical location"
        );
        assert_ne!(
            fs::read(&plan.paths.receipt).unwrap(),
            legacy_bytes,
            "apply must rewrite the receipt to converge on the canonical vault"
        );
    }
}
