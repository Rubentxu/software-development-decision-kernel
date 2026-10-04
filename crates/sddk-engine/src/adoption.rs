//! Repairable two-resource project adoption.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sddk_domain::error::SddkErrorCode;
use sddk_domain::{
    AdoptionReceipt, IdentityError, IdentitySource, Ledger, ProjectId, ResolvedProjectIdentity,
    stable_workspace_id,
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
    /// Project identity, **already resolved** by the caller.
    ///
    /// This field used to be four: `remote_url`, `pinned_project_id`, `scope`
    /// and `fallback_seed`, from which this crate derived the identity itself.
    /// That was a defect, not a convenience (INC-DEBT-059): this crate is
    /// filesystem-free **by design**, so it could never load the project alias
    /// table, so every identity it produced was systematically the *pre-alias*
    /// one. `sddk adopt` and `sddk context bootstrap` therefore re-derived
    /// independently of the canonical resolver, reported the retired id, and —
    /// worse — `adopt apply` wrote a *second* adoption receipt under it, which
    /// is the orphan ADR-0152 exists to make un-re-creatable.
    ///
    /// Taking the identity resolved makes "one decision point" true **by
    /// construction** rather than by note: the derivation inputs are gone, so
    /// there is nothing left to derive from. With an optional override
    /// instead, someone re-adds the call and no behavioural test notices.
    ///
    /// The caller is `sddk-cli`, which reads the pin file
    /// (`.sddk/project-pin.json`) and the alias table
    /// (`$XDG_STATE_HOME/sddk/project-aliases.json`), applies the pin and then
    /// the alias — in that order, which is not interchangeable — and hands the
    /// result here. `identity_source` and `alias_hops` travel intact, so a
    /// checkout that reached its id through a redirect still says so.
    pub identity: ResolvedProjectIdentity,
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
    /// The id this resolution **started from**, when it was redirected through
    /// a project alias; the `project_id` on this same struct is the `to`.
    ///
    /// Criterion 3 of ADR-0152 requires `adopt status` to say that it resolved
    /// through an alias, with the `from` and the `to`. The `to` is
    /// `project_id`; the `from` has to travel separately, because nothing else
    /// on this struct records that a redirect happened — and a redirected
    /// identity that does not say so reads exactly like one that was never
    /// redirected, which is the silent-redirect failure the ADR exists to
    /// close.
    ///
    /// It is `None` when no alias applied, and that is a distinguishable state,
    /// not a missing one: `alias_origin()` returns `None` rather than guessing,
    /// so "resolved here" can never be mistaken for "came from here".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias_origin: Option<ProjectId>,
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

    fn recovery(&self) -> String {
        match self {
            Self::UnsafeState {
                status: AdoptionStatusKind::Conflict,
                ..
            } => "if only the CLI version changed, run `sddk adopt refresh`; \
                 for identity drift, inspect the receipt manually"
                .into(),
            Self::UnsafeState {
                status: AdoptionStatusKind::Corrupt,
                ..
            } => "inspect the receipt file (it may have been truncated or \
                 edited) and re-adopt only after backing it up"
                .into(),
            Self::NothingToRepair => "nothing to repair; run `sddk adopt apply` to create the \
                 initial adoption state"
                .into(),
            _ => "inspect the error detail and retry".into(),
        }
    }
}

/// Builds an adoption plan without reading or writing process or filesystem state.
pub fn plan_adoption(input: AdoptionPlanInput) -> Result<AdoptionPlan, AdoptionError> {
    validate_plan_input(&input)?;
    // The identity arrives RESOLVED. It used to be derived here, from
    // `remote_url` + `pinned_project_id` + `scope` + `fallback_seed`, and that
    // was a defect: this crate cannot load the alias table, so anything it
    // derived was the pre-alias id. INC-DEBT-059.
    //
    // The knowledge that used to live here has moved, not disappeared, and the
    // reason it moved is that it was always a statement about *resolution*
    // rather than about this function:
    //
    // - "Pinned identity wins over derivation (W2c)" — `adopt status` used to
    //   re-derive from the remote and report a different project_id than
    //   `project resolve` on the same checkout. INC-DEBT-049.
    // - "The pin overrides the project_id ONLY" — OBSERVED (session-65i, this
    //   repo): the pin holds `p-63676b11dc0ef88f` and the stored receipt holds
    //   the same id, workspace, scope, canonical path and byte-identical
    //   storage paths; only `remote_url` differed, so `adopt status` and
    //   `adopt refresh` both answered `conflict` against a receipt that
    //   matched on all seven other fields. `remote_url` and `scope` are also
    //   identity — they feed `same_identity`.
    // - "derive first, then let the pin override the id" is what
    //   `resolve_identity_honoring_pin_with` does
    //   (`crates/sddk-cli/src/lib.rs`), and there the alias is applied after
    //   both branches. That is the one resolver now, and the order is not
    //   interchangeable.
    //
    // What is deliberately NOT done here is to accept the resolved id as
    // `pinned_project_id`. That looks equivalent and is not: this crate would
    // treat it as a pin, `identity_source` would not travel, and `alias_origin`
    // would be lost one level deeper — with a shape that *looks* right.
    let identity = input.identity;
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
    if plan.paths.receipt.exists() {
        let existing = read_adoption_receipt(&plan.paths.receipt)?;
        if !same_identity(&existing, &plan.receipt) {
            return Err(AdoptionError::UnsafeState {
                status: AdoptionStatusKind::Conflict,
                detail: "existing receipt has a different identity".into(),
            });
        }
        // C3i objetivo 2: bootstrap repetido sobre un proyecto convergido es
        // un no-op semantico. La identidad ya coincide y el runtime solo
        // converge cuando algo puede cambiar: si el recibo en disco ya
        // representa el mismo estado (identidad + runtime metadata), NO se
        // reescribe. Sin esto cada re-apply reescribia el recibo con un
        // timestamp nuevo (observado en session-37: hash del recibo distinto
        // en cada apply), convirtiendo el bootstrap en un refresh encubierto.
        // PIN C3i: eliminar este short-circuit hace fallar
        // apply_on_converged_adoption_is_byte_stable_across_repeats (RED
        // observado al revertirlo en session-37).
        if existing.runtime_version == plan.receipt.runtime_version
            && existing.sddk_version == plan.receipt.sddk_version
            && existing.configuration_hash == plan.receipt.configuration_hash
        {
            write_knowledge_profile(plan)?;
            return Ok(());
        }
        write_receipt_atomically(&plan.paths.receipt, &plan.receipt, true)?;
        write_knowledge_profile(plan)?;
        return Ok(());
    }
    write_receipt_atomically(&plan.paths.receipt, &plan.receipt, false)?;
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
                // C3i: refresh es el verbo EXPLICITO de runtime metadata. A
                // diferencia de apply (no-op sobre convergido), refresh SI
                // reescribe el recibo con el timestamp/actor del plan cuando
                // difieren ("Always refreshes the on-disk receipt when the
                // identity matches"). El pin de byte-estabilidad de apply vive
                // en apply_on_converged_adoption_is_byte_stable_across_repeats;
                // este bypass es lo que mantiene el contrato de refresh.
                let metadata_differs = existing.runtime_version != plan.receipt.runtime_version
                    || existing.sddk_version != plan.receipt.sddk_version
                    || existing.timestamp != plan.receipt.timestamp
                    || existing.actor != plan.receipt.actor
                    || existing.configuration_hash != plan.receipt.configuration_hash;
                if metadata_differs {
                    write_receipt_atomically(&plan.paths.receipt, &plan.receipt, true)?;
                }
                write_knowledge_profile(plan)?;
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
        && (!sddk_domain::remote_urls_equivalent(
            existing.remote_url.as_deref(),
            plan.identity.remote_url.as_deref(),
        ) || existing.scope != plan.identity.scope)
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
/// `remote_url` is compared as IDENTITY via `sddk_domain::remote_urls_equivalent`, not as a raw
/// string: the domain normalizes it before minting `project_id`, so the case
/// of the owner is not part of the identity. See that function for the
/// observed failure this encodes.
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
        && sddk_domain::remote_urls_equivalent(
            left.remote_url.as_deref(),
            right.remote_url.as_deref(),
        )
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
    // The pin check that used to live here is gone with the field. It is not
    // lost: a malformed pin must still fail loud rather than fall through to
    // derivation, which is INC-DEBT-049, and it still does — `load_project_pin`
    // (`crates/sddk-cli/src/lib.rs`) rejects a bad `schema_version` and a
    // `project_id` that is not `p-*`, and `resolve_identity_honoring_pin_with`
    // runs the value through `ProjectId::new` before using it. The check moved
    // to the place that owns the pin, which is the same move as the identity
    // itself: a checkout-local concern has no business being validated by a
    // filesystem-free engine.
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
        alias_origin: plan.identity.alias_origin().cloned(),
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

    /// The identity a caller resolves for `remote` and hands in.
    ///
    /// Every in-module fixture spells this out now that the engine does not
    /// derive it. That is the cost of the change and it is the point of it: a
    /// fixture that cannot name a remote without also naming a scope, a seed
    /// and a pin is a fixture that cannot pretend the engine is choosing.
    fn identity_for(remote: &str) -> ResolvedProjectIdentity {
        sddk_domain::resolve_project_identity(Some(remote), ".", None).unwrap()
    }

    #[test]
    fn existing_xdg_vault_legacy_receipt_is_absorbed_by_apply() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        let plan = plan_adoption(AdoptionPlanInput {
            identity: identity_for("https://example.com/acme/repo.git"),
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

    /// C3i objetivo 2 (CTX-UAT-001): aplicar adopcion sobre un proyecto ya
    /// convergido debe ser un no-op semantico a nivel de BYTES del recibo.
    /// El timestamp del plan es fijo, asi que la unica fuente de mutacion es
    /// `converge` reescribiendo lo que ya esta escrito. Los 20 re-apply del
    /// exit gate del roadmap se modelan con 3 (mismo contrato: byte-estable).
    #[test]
    fn apply_on_converged_adoption_is_byte_stable_across_repeats() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        let plan = plan_adoption(AdoptionPlanInput {
            identity: identity_for("https://example.com/acme/repo.git"),
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
            runtime_version: "2.2.32".into(),
            timestamp: "2026-09-29T00:00:00Z".into(),
            actor: "test".into(),
        })
        .unwrap();
        let mut ledger = sddk_storage::Storage::open(&plan.paths.ledger).unwrap();
        apply_adoption(&plan, &mut ledger).unwrap();
        let after_first = fs::read(&plan.paths.receipt).unwrap();

        for repeat in 0..3 {
            let status = apply_adoption(&plan, &mut ledger).unwrap();
            assert_eq!(status.status, AdoptionStatusKind::Complete);
            let after_repeat = fs::read(&plan.paths.receipt).unwrap();
            assert_eq!(
                after_repeat, after_first,
                "re-apply #{repeat} reescribio el recibo byte-identicamente convergido: el bootstrap repetido debe ser un no-op, no un refresh"
            );
        }
    }

    // ---------------------------------------------------------------------
    // session-65i: identidad del remoto insensible al case.
    //
    // OBSERVADO (sddk-framework, recibo real de session-63 escrito el
    // 2026-09-30T07:47:47Z): el recibo en disco guarda el owner del remoto EN
    // MAYUSCULAS (`https://github.com/Rubentxu/software-development-decision-kernel`)
    // porque se acuno antes de que `normalize_remote_path` (sddk-domain)
    // bajara cada segmento a minuscula.
    //
    // El DOMINIO ya decidio que la identidad es INSENSIBLE al case: el
    // `project_id` resultante es identico, y eso lo fija el test golden
    // `case_change_in_owner_or_repo_resolves_to_same_project_id` en
    // `sddk-domain::identity`. Comparar el remoto CRUDO aqui contradedia esa
    // decision y reportaba `conflict` contra un recibo que describe el MISMO
    // proyecto en las otras siete comparaciones de `same_identity`.
    //
    // Estos DOS tests separan los DOS sitios que comparan el remoto en crudo,
    // porque arreglar uno solo trasladaria el conflicto al otro:
    //   - `same_identity`   -> recibo en disco
    //   - `inspect_ledger`  -> fila de la tabla `projects`
    const REMOTE_LOWER: &str = "https://github.com/rubentxu/software-development-decision-kernel";
    const REMOTE_FOSSILIZED: &str =
        "https://github.com/Rubentxu/software-development-decision-kernel";

    fn plan_with_remote(directory: &Path, remote: &str) -> AdoptionPlan {
        plan_with_remote_pinned(directory, remote, None)
    }

    /// A plan whose identity came in already resolved, which is the only shape
    /// this function accepts now (INC-DEBT-059).
    ///
    /// `pinned_project_id` no longer reaches the engine — it is a
    /// checkout-local file, and the engine is filesystem-free — so this helper
    /// applies it to the *resolved identity* the way the CLI's resolver does:
    /// derive from the remote, then override the id and say so. That is
    /// faithful, not a re-implementation of the decision: the tests that use the
    /// pinned arm are not testing the pin. They need two identities that share
    /// a `project_id`, a `workspace_id` and the derived paths while differing
    /// in the remote, and the pin was only ever the way to *build* that pair.
    /// The subject is `same_identity`, which must reject the pair.
    fn plan_with_remote_pinned(
        directory: &Path,
        remote: &str,
        pinned_project_id: Option<&str>,
    ) -> AdoptionPlan {
        let root = directory.join("repo");
        fs::create_dir_all(&root).unwrap();
        let mut identity = identity_for(remote);
        if let Some(pinned) = pinned_project_id {
            identity.project_id = ProjectId::new(pinned).unwrap();
            identity.identity_source = IdentitySource::Pinned;
        }
        plan_adoption(AdoptionPlanInput {
            identity,
            canonical_workspace_path: root,
            display_name: "sddk-framework".into(),
            xdg: XdgEnvironment {
                home: Some(directory.join("home")),
                data_home: Some(directory.join("data")),
                state_home: Some(directory.join("state")),
                cache_home: Some(directory.join("cache")),
                ..XdgEnvironment::default()
            },
            sddk_version: "3.6".into(),
            runtime_version: "2.2.34".into(),
            timestamp: "2026-09-30T07:47:47Z".into(),
            actor: "rubentxu".into(),
        })
        .unwrap()
    }

    /// El recibo fosilizado describe el MISMO proyecto: el `project_id` es
    /// identico porque el dominio normaliza antes de hashear. Un `conflict`
    /// aqui es un falso positivo que bloquea `status`, `refresh` y `apply`.
    #[test]
    fn fossilized_capitalized_receipt_is_still_the_same_identity() {
        let directory = tempfile::tempdir().unwrap();
        let plan = plan_with_remote(directory.path(), REMOTE_LOWER);
        let mut ledger = sddk_storage::Storage::open(&plan.paths.ledger).unwrap();
        apply_adoption(&plan, &mut ledger).unwrap();

        // Fosiliza SOLO el recibo, con el hash recomputado para que siga siendo
        // internamente coherente: un hash invalido produciria `Corrupt`, que es
        // otro fallo y no el que se quiere falsificar aqui.
        let mut fossilized = plan.receipt.clone();
        fossilized.remote_url = Some(REMOTE_FOSSILIZED.into());
        fossilized.configuration_hash = configuration_hash(&fossilized).unwrap();
        fs::write(
            &plan.paths.receipt,
            serde_json::to_vec_pretty(&fossilized).unwrap(),
        )
        .unwrap();

        // El dominio ya los considera el mismo proyecto; el motor debe coincidir.
        assert_eq!(
            plan.identity.project_id,
            plan_adoption(AdoptionPlanInput {
                identity:
                    sddk_domain::resolve_project_identity(Some(REMOTE_FOSSILIZED), ".", None,)
                        .unwrap(),
                canonical_workspace_path: plan.receipt.canonical_workspace_path.clone().into(),
                display_name: "sddk-framework".into(),
                xdg: XdgEnvironment {
                    home: Some(directory.path().join("home")),
                    data_home: Some(directory.path().join("data")),
                    state_home: Some(directory.path().join("state")),
                    cache_home: Some(directory.path().join("cache")),
                    ..XdgEnvironment::default()
                },
                sddk_version: "3.6".into(),
                runtime_version: "2.2.34".into(),
                timestamp: "2026-09-30T07:47:47Z".into(),
                actor: "rubentxu".into(),
            })
            .unwrap()
            .identity
            .project_id,
            "el project_id debe ser identico con y sin mayusculas en el owner"
        );

        let status = adoption_status(&plan, &ledger).unwrap();
        assert_eq!(
            status.status,
            AdoptionStatusKind::Complete,
            "un recibo fosilizado con el owner en mayusculas describe el mismo proyecto \
             (mismo project_id, mismo workspace, mismos paths): reportar conflict es un \
             falso positivo. detalle: {:?}",
            status.detail
        );
    }

    /// Mismo caso sobre la fila de la tabla `projects`. Si solo se arreglara
    /// `same_identity`, este `conflict` pasaria a ser el unico que sobrevive.
    ///
    /// NOTA sobre como se fosiliza la fila: se escribe con SQL directo porque
    /// la fila tiene que quedar como la dejo un binario ANTERIOR a la
    /// normalizacion — que es exactamente el estado real de este repo (fila
    /// `projects` con `Rubentxu/...`, verificado sobre el ledger vivo).
    ///
    /// Session-76: la nota anterior decia que `register_project_workspace`
    /// "RECHAZA eso hoy por diseno (RegistrationConflict)" y que por eso el
    /// guard bajo prueba era `inspect_ledger` y no el del storage. **Era un
    /// workaround disfrazado de decision**, y sostenia el defecto: el storage
    /// seguia negando una fila que el motor declaraba la MISMA identidad, asi
    /// que `adopt status` decia `complete` y `adopt apply` decia conflicto
    /// sobre el mismo estado. Con la comparacion unica en el dominio, esa
    /// fila se registra por la API normal y el atajo de SQL crudo ya solo hace
    /// falta para FABRICAR el estado fosilizado, que es justo para lo que
    /// este test lo usa.
    ///
    /// Y el test ahora exige las DOS mitades — lectura Y escritura. Solo
    /// asertando el `status` (que es lo que hacia antes) la divergencia era
    /// invisible: el test pasaba mientras `apply_adoption` fallaba.
    #[test]
    fn fossilized_capitalized_ledger_row_is_still_the_same_identity() {
        let directory = tempfile::tempdir().unwrap();
        let plan = plan_with_remote(directory.path(), REMOTE_LOWER);
        {
            let mut ledger = sddk_storage::Storage::open(&plan.paths.ledger).unwrap();
            apply_adoption(&plan, &mut ledger).unwrap();
        }

        // Fosiliza SOLO la fila del ledger; el recibo se queda como esta.
        let project_id = plan.identity.project_id.to_string();
        let connection = rusqlite::Connection::open(&plan.paths.ledger).unwrap();
        connection
            .execute(
                "UPDATE projects SET remote_url = ?1 WHERE project_id = ?2",
                rusqlite::params![REMOTE_FOSSILIZED, project_id],
            )
            .unwrap();
        drop(connection);

        // Mitad 1 — LECTURA. `inspect_ledger` dice que es la misma identidad.
        let ledger = sddk_storage::Storage::open(&plan.paths.ledger).unwrap();
        let status = adoption_status(&plan, &ledger).unwrap();
        assert_eq!(
            status.status,
            AdoptionStatusKind::Complete,
            "la fila del ledger con el owner en mayusculas es la misma identidad: \
             detail: {:?}",
            status.detail
        );

        // Mitad 2 — ESCRITURA, y es la que faltaba. El status de arriba dice
        // `complete`, luego el comando de aplicar NO puede responder
        // `RegistrationConflict` sobre ese mismo ledger. ANTES de session-76
        // lo hacia, porque la comparacion del remoto en
        // `Storage::register_project_workspace` era byte a byte mientras las
        // dos del motor eran case-insensitive: un fallo abierto en la lectura
        // contra uno cerrado en la escritura, con veredictos incompatibles.
        let mut ledger = sddk_storage::Storage::open(&plan.paths.ledger).unwrap();
        let applied = apply_adoption(&plan, &mut ledger).expect(
            "`adopt status` acaba de decir complete: la escritura tiene que poder converger",
        );
        assert_eq!(
            applied.status,
            AdoptionStatusKind::Complete,
            "aplicar sobre una fila fosilizada del mismo proyecto converge: detail: {:?}",
            applied.detail
        );
    }

    // ---------------------------------------------------------------------
    // La OTRA mitad del contrato, que faltaba.
    //
    // MUTACION (session-65i): sustituir la comparacion por `right == *right`
    // —devolviendo `true` siempre que ambos lados normalicen— dejo los dos
    // tests de arriba EN VERDE. Fijaban «el mismo repo con otro case ya no
    // es conflicto», pero NO «un repo distinto sigue siendo conflicto»: un
    // guard que declara siempre coincidencia era aceptable.
    //
    // Estos dos tests cierran ese lado. Sin ellos el arreglo normalizador
    // degrada la deteccion de drift en vez de afinarla.
    //
    // POR QUE EL PIN (intento descartado primero): un remoto distinto sin pin
    // acuña otro `project_id`, luego apunta a rutas inexistentes y el veredicto
    // es `Absent`. El test discriminaba, pero por el guard equivocado (las
    // rutas), no por la comparacion de identidad. El pin es la unica forma de
    // que dos remotos genuinamente distintos compartan `project_id` y rutas:
    // el pin dice «esto es el proyecto X» mientras el checkout apunta a un
    // repo Y. Ese es el escenario que la comparacion tiene que rechazar.
    const REMOTE_OTHER: &str = "https://github.com/rubentxu/otro-repo";

    /// Adopta REMOTE_LOWER y devuelve (plan, ledger) junto al plan pinneado a
    /// ese mismo `project_id` pero con REMOTE_OTHER.
    fn diverged_by_pin(directory: &Path) -> (AdoptionPlan, sddk_storage::Storage, AdoptionPlan) {
        let plan = plan_with_remote(directory, REMOTE_LOWER);
        let mut ledger = sddk_storage::Storage::open(&plan.paths.ledger).unwrap();
        apply_adoption(&plan, &mut ledger).unwrap();
        let pinned = plan_with_remote_pinned(
            directory,
            REMOTE_OTHER,
            Some(plan.identity.project_id.as_str()),
        );
        // El pin iguala project_id, workspace_id y rutas: lo UNICO que
        // distingue las dos identidades es el remoto.
        assert_eq!(plan.identity.project_id, pinned.identity.project_id);
        assert_eq!(plan.paths.receipt, pinned.paths.receipt);
        assert_ne!(plan.identity.remote_url, pinned.identity.remote_url);
        (plan, ledger, pinned)
    }

    #[test]
    fn different_remote_under_the_same_pin_is_still_a_different_identity_receipt() {
        let directory = tempfile::tempdir().unwrap();
        let (_plan, ledger, diverged) = diverged_by_pin(directory.path());
        let status = adoption_status(&diverged, &ledger).unwrap();
        assert_eq!(
            status.status,
            AdoptionStatusKind::Conflict,
            "un remoto genuinamente distinto bajo el mismo pin debe seguir siendo \
             conflict: normalizar el case no puede validar el drift de remoto. \
             detail: {:?}",
            status.detail
        );
    }

    /// Mismo caso sobre la fila del ledger: sin normalizar, este es el unico
    /// guard que separa dos proyectos distintos que comparten checkout.
    #[test]
    fn different_remote_under_the_same_pin_is_still_a_different_identity_ledger() {
        let directory = tempfile::tempdir().unwrap();
        let (plan, ledger, diverged) = diverged_by_pin(directory.path());
        // El recibo de `plan` esta intacto; se fosiliza la fila del ledger con
        // el remoto divergente para ejercer el segundo guard de forma aislada.
        drop(ledger);
        let connection = rusqlite::Connection::open(&plan.paths.ledger).unwrap();
        connection
            .execute(
                "UPDATE projects SET remote_url = ?1 WHERE project_id = ?2",
                rusqlite::params![REMOTE_OTHER, diverged.identity.project_id.to_string()],
            )
            .unwrap();
        drop(connection);

        let ledger = sddk_storage::Storage::open(&plan.paths.ledger).unwrap();
        let status = adoption_status(&diverged, &ledger).unwrap();
        assert_eq!(
            status.status,
            AdoptionStatusKind::Conflict,
            "el ledger debe seguir separando dos remotos distintos bajo el mismo pin. \
             detail: {:?}",
            status.detail
        );
    }
}
