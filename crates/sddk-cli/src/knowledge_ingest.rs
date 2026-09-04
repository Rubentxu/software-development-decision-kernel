//! Governed repository-knowledge ingestion and capability resolution.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::{DirEntry, WalkDir};

use crate::knowledge_cmd::{
    KnowledgeImportArgs, KnowledgeScanArgs, KnowledgeVerifyArgs, ManagedKnowledgeContext,
    resolve_managed_knowledge,
};
use crate::{CliEnvironment, CommandOutput, OutputFormat};
use sddk_engine::authority::{AuthorityContext, infer_actor_kind};

const SCHEMA_VERSION: &str = "1.0.0";
const INGESTION_ROOT: &str = "ingestion";
const ARCHITECTURE_CAPABILITY: &str = "architecture_rules";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum KnowledgeKind {
    Adr,
    Specification,
    Term,
    Incidence,
    Roadmap,
    Manifest,
    ArchitectureRules,
    ArchitectureBaseline,
    RepositoryContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TargetType {
    Adr,
    Requirement,
    Term,
    Incidence,
    Evidence,
    CapabilityResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    Import,
    Unchanged,
    NeedsReview,
    Quarantine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Authority {
    Proposed,
    Trusted,
    NeedsReview,
    Stale,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CapabilityStatus {
    Current,
    NeedsReview,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ImportCandidate {
    entry_id: String,
    source_path: PathBuf,
    source_commit: String,
    line_start: usize,
    line_end: usize,
    sha256: String,
    owner: Option<String>,
    kind: KnowledgeKind,
    target_type: TargetType,
    relation: String,
    links: Vec<String>,
    existing_entry_id: Option<String>,
    disposition: Disposition,
    reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ImportPlan {
    schema_version: String,
    plan_id: String,
    project_id: String,
    source_root: PathBuf,
    source_commit: String,
    created_at: String,
    candidates: Vec<ImportCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeVersion {
    version_id: String,
    source_commit: String,
    line_start: usize,
    line_end: usize,
    sha256: String,
    object_path: PathBuf,
    imported_at: String,
    authority: Authority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangelogEvent {
    recorded_at: String,
    valid_from: String,
    valid_to: Option<String>,
    action: String,
    plan_id: String,
    version_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeEntry {
    entry_id: String,
    source_path: PathBuf,
    owner: Option<String>,
    kind: KnowledgeKind,
    target_type: TargetType,
    relation: String,
    links: Vec<String>,
    versions: Vec<KnowledgeVersion>,
    changelog: Vec<ChangelogEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeCapability {
    capability_id: String,
    name: String,
    authority: Authority,
    status: CapabilityStatus,
    resources: BTreeMap<String, String>,
    registered_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeIncidence {
    incidence_id: String,
    kind: String,
    source_path: PathBuf,
    conflicting_entry_id: String,
    detected_at: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeRegistry {
    schema_version: String,
    project_id: String,
    entries: Vec<KnowledgeEntry>,
    capabilities: Vec<KnowledgeCapability>,
    incidences: Vec<KnowledgeIncidence>,
}

impl KnowledgeRegistry {
    fn empty(project_id: &str) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_owned(),
            project_id: project_id.to_owned(),
            entries: Vec::new(),
            capabilities: Vec::new(),
            incidences: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ScanOutput {
    plan_id: String,
    plan_path: PathBuf,
    source_commit: String,
    candidates: usize,
    importable: usize,
    unchanged: usize,
    needs_review: usize,
    quarantined: usize,
    plan: ImportPlan,
}

#[derive(Debug, Serialize)]
struct ImportOutput {
    plan_id: String,
    registry_path: PathBuf,
    imported: usize,
    unchanged: usize,
    quarantined: usize,
    approved: usize,
    capabilities_registered: usize,
}

#[derive(Debug, Serialize)]
struct VerifyEntry {
    entry_id: String,
    source_path: PathBuf,
    status: String,
    authority: Authority,
    expected_sha256: String,
    actual_sha256: Option<String>,
}

#[derive(Debug, Serialize)]
struct VerifyOutput {
    registry_present: bool,
    valid: bool,
    entries: Vec<VerifyEntry>,
    untracked: Vec<PathBuf>,
    incidences: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ArchitectureCapabilityResolution {
    pub(crate) capability_id: Option<String>,
    pub(crate) authority: Option<Authority>,
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) receipt_id: String,
    #[serde(skip)]
    pub(crate) catalog: Option<PathBuf>,
    #[serde(skip)]
    pub(crate) baseline: Option<PathBuf>,
}

pub(crate) fn run_scan(args: KnowledgeScanArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ScanOutput> {
        let context = resolve_context(
            &args.root,
            &args.scope,
            args.remote,
            args.fallback_seed,
            environment,
        )?;
        let registry = load_registry(&context)?
            .unwrap_or_else(|| KnowledgeRegistry::empty(&context.project_id));
        let source_commit = git_commit(&context.root);
        let tracked = git_tracked_files(&context.root);
        let candidates = discover_candidates(&context.root, &source_commit, &tracked, &registry)?;
        let created_at = crate::git_cmd::default_timestamp();
        let plan_id = plan_id(&context.project_id, &source_commit, &candidates)?;
        let mut plan = ImportPlan {
            schema_version: SCHEMA_VERSION.to_owned(),
            plan_id: plan_id.clone(),
            project_id: context.project_id,
            source_root: context.root,
            source_commit: source_commit.clone(),
            created_at,
            candidates,
        };
        let path = plan_path(&context.vault_path, &plan_id)?;
        if path.is_file() {
            let existing: ImportPlan = serde_json::from_slice(&fs::read(&path)?)?;
            if existing.project_id != plan.project_id
                || existing.source_commit != plan.source_commit
                || existing.candidates != plan.candidates
            {
                anyhow::bail!("knowledge plan id collision at {}", path.display());
            }
            plan = existing;
        } else {
            crate::atomic_write_path(&path, &serde_json::to_vec_pretty(&plan)?)?;
            append_log(
                &context.vault_path,
                &format!(
                    "scan | knowledge plan {} | {} candidates",
                    plan_id,
                    plan.candidates.len()
                ),
            )?;
        }
        Ok(ScanOutput {
            plan_id,
            plan_path: path,
            source_commit,
            candidates: plan.candidates.len(),
            importable: count_disposition(&plan.candidates, Disposition::Import),
            unchanged: count_disposition(&plan.candidates, Disposition::Unchanged),
            needs_review: count_disposition(&plan.candidates, Disposition::NeedsReview),
            quarantined: count_disposition(&plan.candidates, Disposition::Quarantine),
            plan,
        })
    })();
    render(result, format, scan_text)
}

pub(crate) fn run_import(args: KnowledgeImportArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ImportOutput> {
        let context = resolve_context(
            &args.root,
            &args.scope,
            args.remote,
            args.fallback_seed,
            environment,
        )?;
        // AC-EVT-LEDGER-09: knowledge vault writes require Human authority
        let actor = environment
            .user
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .unwrap_or_else(|| "anonymous".into());
        let actor_kind = infer_actor_kind(&actor);
        let auth = AuthorityContext::for_cli(actor, actor_kind, None, None);
        auth.validate(sddk_engine::authority::WritableSurface::KnowledgeGraphVault)
            .map_err(|e| anyhow::anyhow!("authority check failed: {}", e))?;
        let path = plan_path(&context.vault_path, &args.plan)?;
        let plan: ImportPlan = serde_json::from_slice(&fs::read(&path)?)?;
        validate_plan(&plan, &context)?;
        let mut registry = load_registry(&context)?
            .unwrap_or_else(|| KnowledgeRegistry::empty(&context.project_id));
        let now = crate::git_cmd::default_timestamp();
        let mut imported = 0;
        let mut unchanged = 0;
        let mut quarantined = 0;
        let approvals = args.approve.into_iter().collect::<BTreeSet<_>>();
        let approvable = plan
            .candidates
            .iter()
            .filter(|candidate| is_approvable_change(candidate))
            .map(|candidate| candidate.entry_id.clone())
            .collect::<BTreeSet<_>>();
        if let Some(invalid) = approvals.difference(&approvable).next() {
            anyhow::bail!(
                "entry {invalid} cannot be approved; only changed existing entries are approvable"
            );
        }
        let mut approved = 0;

        for candidate in &plan.candidates {
            if candidate.disposition == Disposition::Unchanged {
                unchanged += 1;
                continue;
            }
            let bytes = fs::read(context.root.join(&candidate.source_path))?;
            let actual = sha256(&bytes);
            if actual != candidate.sha256 {
                anyhow::bail!(
                    "source changed after scan: {}; run `sddk knowledge scan` again",
                    candidate.source_path.display()
                );
            }
            let object_path = object_path(&candidate.sha256)?;
            crate::atomic_write_path(&context.vault_path.join(&object_path), &bytes)?;
            let explicitly_approved = approvals.contains(&candidate.entry_id);
            let authority = if candidate.disposition == Disposition::Import || explicitly_approved {
                approved += usize::from(explicitly_approved);
                Authority::Trusted
            } else {
                quarantined += 1;
                Authority::NeedsReview
            };
            upsert_entry(
                &mut registry,
                candidate,
                &object_path,
                authority,
                &now,
                &plan.plan_id,
            );
            if candidate.disposition == Disposition::NeedsReview
                && !explicitly_approved
                && let Some(conflict) = &candidate.existing_entry_id
            {
                append_incidence(&mut registry, candidate, conflict, &now);
            }
            imported += 1;
        }
        rebuild_architecture_capability(&mut registry, &now);
        let registry_path = registry_path(&context.vault_path);
        crate::atomic_write_path(&registry_path, &serde_json::to_vec_pretty(&registry)?)?;
        append_log(
            &context.vault_path,
            &format!(
                "import | knowledge plan {} | {} versions",
                plan.plan_id, imported
            ),
        )?;
        Ok(ImportOutput {
            plan_id: plan.plan_id,
            registry_path,
            imported,
            unchanged,
            quarantined,
            approved,
            capabilities_registered: registry
                .capabilities
                .iter()
                .filter(|capability| capability.status == CapabilityStatus::Current)
                .count(),
        })
    })();
    render(result, format, import_text)
}

pub(crate) fn run_verify(args: KnowledgeVerifyArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<VerifyOutput> {
        let context = resolve_context(
            &args.root,
            &args.scope,
            args.remote,
            args.fallback_seed,
            environment,
        )?;
        let Some(mut registry) = load_registry(&context)? else {
            let candidates = discover_candidates(
                &context.root,
                &git_commit(&context.root),
                &git_tracked_files(&context.root),
                &KnowledgeRegistry::empty(&context.project_id),
            )?;
            return Ok(VerifyOutput {
                registry_present: false,
                valid: candidates.is_empty(),
                entries: Vec::new(),
                untracked: candidates
                    .into_iter()
                    .map(|candidate| candidate.source_path)
                    .collect(),
                incidences: Vec::new(),
            });
        };
        let mut entries = Vec::new();
        let mut registered = BTreeSet::new();
        let mut incidences = Vec::new();
        let mut pending_incidences = Vec::new();
        for entry in &registry.entries {
            registered.insert(entry.source_path.clone());
            let Some(version) = entry.versions.last() else {
                continue;
            };
            let source = context.root.join(&entry.source_path);
            let actual = fs::read(&source).ok().map(|bytes| sha256(&bytes));
            let object_current = safe_relative(&version.object_path)
                && fs::read(context.vault_path.join(&version.object_path))
                    .ok()
                    .map(|bytes| sha256(&bytes))
                    .as_deref()
                    == Some(version.sha256.as_str());
            let status = match (actual.as_deref(), object_current) {
                (None, _) => "missing",
                (Some(hash), _) if hash != version.sha256 => "changed",
                (Some(_), false) => "object_corrupt",
                (Some(_), true) => "current",
            };
            if status != "current" {
                incidences.push(format!("{}:{status}", entry.entry_id));
                pending_incidences.push((
                    entry.entry_id.clone(),
                    entry.source_path.clone(),
                    status.to_owned(),
                ));
            }
            entries.push(VerifyEntry {
                entry_id: entry.entry_id.clone(),
                source_path: entry.source_path.clone(),
                status: status.to_owned(),
                authority: version.authority,
                expected_sha256: version.sha256.clone(),
                actual_sha256: actual,
            });
        }
        let discovered = discover_candidates(
            &context.root,
            &git_commit(&context.root),
            &git_tracked_files(&context.root),
            &registry,
        )?;
        let untracked = discovered
            .into_iter()
            .filter(|candidate| !registered.contains(&candidate.source_path))
            .map(|candidate| candidate.source_path)
            .collect::<Vec<_>>();
        let valid = entries
            .iter()
            .all(|entry| entry.status == "current" && entry.authority == Authority::Trusted)
            && untracked.is_empty();
        let now = crate::git_cmd::default_timestamp();
        let mut registry_changed = false;
        for (entry_id, source_path, status) in pending_incidences {
            let incidence_id = digest_id("ki", format!("verify:{entry_id}:{status}").as_bytes());
            if !registry
                .incidences
                .iter()
                .any(|incidence| incidence.incidence_id == incidence_id)
            {
                registry.incidences.push(KnowledgeIncidence {
                    incidence_id,
                    kind: format!("source_{status}"),
                    source_path,
                    conflicting_entry_id: entry_id,
                    detected_at: now.clone(),
                    status: "open".to_owned(),
                });
                registry_changed = true;
            }
        }
        let stale_entries = entries
            .iter()
            .filter(|entry| entry.status != "current")
            .map(|entry| entry.entry_id.as_str())
            .collect::<BTreeSet<_>>();
        for capability in &mut registry.capabilities {
            if capability
                .resources
                .values()
                .any(|entry_id| stale_entries.contains(entry_id.as_str()))
                && capability.status != CapabilityStatus::Stale
            {
                capability.status = CapabilityStatus::Stale;
                capability.authority = Authority::Stale;
                registry_changed = true;
            }
        }
        if registry_changed {
            crate::atomic_write_path(
                &registry_path(&context.vault_path),
                &serde_json::to_vec_pretty(&registry)?,
            )?;
            append_log(
                &context.vault_path,
                &format!("verify | knowledge drift | {} incidences", incidences.len()),
            )?;
        }
        Ok(VerifyOutput {
            registry_present: true,
            valid,
            entries,
            untracked,
            incidences,
        })
    })();
    render(result, format, verify_text)
}

pub(crate) fn resolve_architecture_capability(
    context: &ManagedKnowledgeContext,
) -> anyhow::Result<ArchitectureCapabilityResolution> {
    let Some(registry) = load_registry(context)? else {
        return Ok(not_applicable(
            "architecture capability is not registered (knowledge registry is absent)",
            None,
            None,
        ));
    };
    let Some(capability) = registry
        .capabilities
        .iter()
        .find(|capability| capability.name == ARCHITECTURE_CAPABILITY)
    else {
        return Ok(not_applicable(
            "architecture capability is not registered",
            None,
            None,
        ));
    };
    if capability.authority != Authority::Trusted || capability.status != CapabilityStatus::Current
    {
        let reason = format!(
            "architecture capability is {:?}/{:?}, not trusted/current",
            capability.authority, capability.status
        )
        .to_ascii_lowercase();
        return Ok(not_applicable(
            &reason,
            Some(capability.capability_id.clone()),
            Some(capability.authority),
        ));
    }
    let catalog = resolve_resource(context, &registry, capability, "catalog")?;
    let baseline = resolve_resource(context, &registry, capability, "baseline")?;
    let (Some(catalog), Some(baseline)) = (catalog, baseline) else {
        return Ok(not_applicable(
            "architecture capability evidence is stale or incomplete",
            Some(capability.capability_id.clone()),
            Some(capability.authority),
        ));
    };
    let receipt_id = digest_id(
        "kr",
        format!(
            "{}:{}:{}",
            capability.capability_id,
            catalog.display(),
            baseline.display()
        )
        .as_bytes(),
    );
    Ok(ArchitectureCapabilityResolution {
        capability_id: Some(capability.capability_id.clone()),
        authority: Some(capability.authority),
        status: "current".to_owned(),
        reason: "registered capability evidence is trusted and current".to_owned(),
        receipt_id,
        catalog: Some(catalog),
        baseline: Some(baseline),
    })
}

fn resolve_context(
    root: &Path,
    scope: &str,
    remote: Option<String>,
    fallback_seed: Option<String>,
    environment: &CliEnvironment,
) -> anyhow::Result<ManagedKnowledgeContext> {
    resolve_managed_knowledge(root, scope, remote, fallback_seed, environment)
}

fn discover_candidates(
    root: &Path,
    source_commit: &str,
    tracked: &BTreeSet<PathBuf>,
    registry: &KnowledgeRegistry,
) -> anyhow::Result<Vec<ImportCandidate>> {
    let mut candidates = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(include_entry)
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let source_path = entry.path().strip_prefix(root)?.to_path_buf();
        let Some((kind, target_type, relation_base)) = classify(&source_path) else {
            continue;
        };
        let relation = relation_key(relation_base, &source_path, target_type);
        let bytes = fs::read(entry.path())?;
        let text = String::from_utf8_lossy(&bytes);
        let owner = detect_owner(&text);
        let existing = registry
            .entries
            .iter()
            .find(|known| known.source_path == source_path);
        let relation_conflict = registry
            .entries
            .iter()
            .find(|known| known.relation == relation && known.source_path != source_path);
        let hash = sha256(&bytes);
        let (disposition, reason) = if !tracked.contains(&source_path) {
            (
                Disposition::Quarantine,
                "source is not versioned by Git".to_owned(),
            )
        } else if git_committed_sha256(root, &source_path).as_deref() != Some(hash.as_str()) {
            (
                Disposition::Quarantine,
                "source content does not match the recorded Git commit".to_owned(),
            )
        } else if let Some(existing) = existing {
            if existing
                .versions
                .last()
                .is_some_and(|version| version.sha256 == hash)
            {
                (
                    Disposition::Unchanged,
                    "registered content is unchanged".to_owned(),
                )
            } else {
                (
                    Disposition::NeedsReview,
                    "registered content changed; compatibility requires review".to_owned(),
                )
            }
        } else if let Some(conflict) = relation_conflict {
            (
                Disposition::NeedsReview,
                format!(
                    "relation conflicts with registered entry {}",
                    conflict.entry_id
                ),
            )
        } else if owner.is_none() {
            (
                Disposition::Quarantine,
                "source has no declared owner".to_owned(),
            )
        } else {
            (
                Disposition::Import,
                "versioned source has owner and an unambiguous relation".to_owned(),
            )
        };
        candidates.push(ImportCandidate {
            entry_id: digest_id(
                "ke",
                format!("{}:{}", relation, source_path.display()).as_bytes(),
            ),
            source_path,
            source_commit: source_commit.to_owned(),
            line_start: 1,
            line_end: text.lines().count().max(1),
            sha256: hash,
            owner,
            kind,
            target_type,
            relation,
            links: detect_links(&text),
            existing_entry_id: existing
                .map(|entry| entry.entry_id.clone())
                .or_else(|| relation_conflict.map(|entry| entry.entry_id.clone())),
            disposition,
            reason,
        });
    }
    quarantine_duplicate_capability_resources(&mut candidates);
    candidates.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(candidates)
}

fn is_approvable_change(candidate: &ImportCandidate) -> bool {
    candidate.disposition == Disposition::NeedsReview
        && candidate.existing_entry_id.as_deref() == Some(candidate.entry_id.as_str())
        && candidate.reason.starts_with("registered content changed")
}

fn classify(path: &Path) -> Option<(KnowledgeKind, TargetType, &'static str)> {
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if matches!(
        name.as_str(),
        "architecture-rules.yaml" | "architecture-rules.yml"
    ) {
        return Some((
            KnowledgeKind::ArchitectureRules,
            TargetType::CapabilityResource,
            "architecture_rules.catalog",
        ));
    }
    if name == "baseline-dependency-entropy.json" {
        return Some((
            KnowledgeKind::ArchitectureBaseline,
            TargetType::CapabilityResource,
            "architecture_rules.baseline",
        ));
    }
    let supported = matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("md" | "adoc" | "asciidoc" | "yaml" | "yml" | "json" | "toml")
    );
    if !supported {
        return None;
    }
    if components
        .iter()
        .any(|part| matches!(part.as_str(), "adr" | "adrs"))
        || name.starts_with("adr-")
    {
        return Some((KnowledgeKind::Adr, TargetType::Adr, "decision"));
    }
    if components
        .iter()
        .any(|part| matches!(part.as_str(), "spec" | "specs" | "specifications"))
        || name.starts_with("spec-")
        || name.starts_with("req-")
    {
        return Some((
            KnowledgeKind::Specification,
            TargetType::Requirement,
            "requirement",
        ));
    }
    if components.iter().any(|part| part == "terms") || name.starts_with("term-") {
        return Some((KnowledgeKind::Term, TargetType::Term, "term"));
    }
    if components.iter().any(|part| part == "incidences") || name.starts_with("inc-") {
        return Some((KnowledgeKind::Incidence, TargetType::Incidence, "incidence"));
    }
    if name.contains("roadmap") {
        return Some((KnowledgeKind::Roadmap, TargetType::Evidence, "roadmap"));
    }
    if name.contains("manifest") {
        return Some((KnowledgeKind::Manifest, TargetType::Evidence, "manifest"));
    }
    if path.components().count() == 1
        && matches!(
            name.as_str(),
            "readme.md" | "context.md" | "agents.md" | "architecture.md"
        )
    {
        return Some((
            KnowledgeKind::RepositoryContext,
            TargetType::Evidence,
            "repository_context",
        ));
    }
    None
}

fn relation_key(base: &str, path: &Path, target_type: TargetType) -> String {
    if target_type == TargetType::CapabilityResource || base == "roadmap" {
        return base.to_owned();
    }
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase();
    let identity = if ["adr-", "spec-", "req-", "term-", "inc-"]
        .iter()
        .any(|prefix| stem.starts_with(prefix))
    {
        stem
    } else {
        path.with_extension("")
            .to_string_lossy()
            .replace('\\', "/")
            .to_ascii_lowercase()
    };
    format!("{base}:{identity}")
}

fn include_entry(entry: &DirEntry) -> bool {
    entry.depth() == 0
        || !matches!(
            entry.file_name().to_string_lossy().as_ref(),
            ".git" | ".sddk" | "target" | "node_modules" | "dist" | "build"
        )
}

fn detect_owner(text: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
        && let Some(owner) = value.get("owner").and_then(serde_json::Value::as_str)
        && !owner.trim().is_empty()
    {
        return Some(owner.trim().to_owned());
    }
    text.lines().find_map(|line| {
        line.strip_prefix("owner:")
            .map(|owner| owner.trim().trim_matches(['\'', '"']).to_owned())
            .filter(|owner| !owner.is_empty())
    })
}

fn detect_links(text: &str) -> Vec<String> {
    let mut links = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else {
            break;
        };
        let link = rest[..end].trim();
        if !link.is_empty() {
            links.insert(link.to_owned());
        }
        rest = &rest[end + 2..];
    }
    links.into_iter().collect()
}

fn quarantine_duplicate_capability_resources(candidates: &mut [ImportCandidate]) {
    for relation in ["architecture_rules.catalog", "architecture_rules.baseline"] {
        if candidates
            .iter()
            .filter(|candidate| candidate.relation == relation)
            .count()
            > 1
        {
            for candidate in candidates
                .iter_mut()
                .filter(|candidate| candidate.relation == relation)
            {
                candidate.disposition = Disposition::NeedsReview;
                candidate.reason = format!("multiple candidates claim relation {relation}");
            }
        }
    }
}

fn validate_plan(plan: &ImportPlan, context: &ManagedKnowledgeContext) -> anyhow::Result<()> {
    if plan.schema_version != SCHEMA_VERSION {
        anyhow::bail!("unsupported knowledge plan schema {}", plan.schema_version);
    }
    if plan.project_id != context.project_id || plan.source_root != context.root {
        anyhow::bail!("knowledge plan belongs to a different project or workspace");
    }
    if plan.plan_id != plan_id(&plan.project_id, &plan.source_commit, &plan.candidates)? {
        anyhow::bail!("knowledge plan integrity check failed");
    }
    if plan
        .candidates
        .iter()
        .any(|candidate| !safe_relative(&candidate.source_path))
    {
        anyhow::bail!("knowledge plan contains an unsafe source path");
    }
    let current_commit = git_commit(&context.root);
    if plan.source_commit != current_commit {
        anyhow::bail!("repository commit changed after scan; run `sddk knowledge scan` again");
    }
    Ok(())
}

fn upsert_entry(
    registry: &mut KnowledgeRegistry,
    candidate: &ImportCandidate,
    object_path: &Path,
    authority: Authority,
    now: &str,
    plan_id: &str,
) {
    let entry = match registry
        .entries
        .iter_mut()
        .find(|entry| entry.entry_id == candidate.entry_id)
    {
        Some(entry) => entry,
        None => {
            registry.entries.push(KnowledgeEntry {
                entry_id: candidate.entry_id.clone(),
                source_path: candidate.source_path.clone(),
                owner: candidate.owner.clone(),
                kind: candidate.kind,
                target_type: candidate.target_type,
                relation: candidate.relation.clone(),
                links: candidate.links.clone(),
                versions: Vec::new(),
                changelog: Vec::new(),
            });
            registry.entries.last_mut().expect("entry was inserted")
        }
    };
    if entry
        .versions
        .last()
        .is_some_and(|version| version.sha256 == candidate.sha256)
    {
        return;
    }
    let version_id = digest_id(
        "kv",
        format!(
            "{}:{}:{}",
            candidate.entry_id, candidate.source_commit, candidate.sha256
        )
        .as_bytes(),
    );
    if let Some(previous) = entry.versions.last_mut() {
        let previous_version_id = previous.version_id.clone();
        let previous_valid_from = previous.source_commit.clone();
        previous.authority = Authority::Superseded;
        entry.changelog.push(ChangelogEvent {
            recorded_at: now.to_owned(),
            valid_from: previous_valid_from,
            valid_to: Some(candidate.source_commit.clone()),
            action: "version_superseded".to_owned(),
            plan_id: plan_id.to_owned(),
            version_id: previous_version_id,
        });
    }
    entry.versions.push(KnowledgeVersion {
        version_id: version_id.clone(),
        source_commit: candidate.source_commit.clone(),
        line_start: candidate.line_start,
        line_end: candidate.line_end,
        sha256: candidate.sha256.clone(),
        object_path: object_path.to_path_buf(),
        imported_at: now.to_owned(),
        authority,
    });
    entry.changelog.push(ChangelogEvent {
        recorded_at: now.to_owned(),
        valid_from: candidate.source_commit.clone(),
        valid_to: None,
        action: format!("version_imported:{authority:?}").to_ascii_lowercase(),
        plan_id: plan_id.to_owned(),
        version_id,
    });
}

fn append_incidence(
    registry: &mut KnowledgeRegistry,
    candidate: &ImportCandidate,
    conflict: &str,
    now: &str,
) {
    let id = digest_id(
        "ki",
        format!("{}:{}", candidate.entry_id, conflict).as_bytes(),
    );
    if registry
        .incidences
        .iter()
        .any(|incidence| incidence.incidence_id == id)
    {
        return;
    }
    registry.incidences.push(KnowledgeIncidence {
        incidence_id: id,
        kind: "knowledge_conflict".to_owned(),
        source_path: candidate.source_path.clone(),
        conflicting_entry_id: conflict.to_owned(),
        detected_at: now.to_owned(),
        status: "open".to_owned(),
    });
}

fn rebuild_architecture_capability(registry: &mut KnowledgeRegistry, now: &str) {
    let catalog = trusted_relation_entry(registry, "architecture_rules.catalog");
    let baseline = trusted_relation_entry(registry, "architecture_rules.baseline");
    let mut resources = BTreeMap::new();
    if let Some(entry) = catalog {
        resources.insert("catalog".to_owned(), entry);
    }
    if let Some(entry) = baseline {
        resources.insert("baseline".to_owned(), entry);
    }
    let current = resources.len() == 2;
    let capability = KnowledgeCapability {
        capability_id: digest_id("kc", ARCHITECTURE_CAPABILITY.as_bytes()),
        name: ARCHITECTURE_CAPABILITY.to_owned(),
        authority: if current {
            Authority::Trusted
        } else {
            Authority::NeedsReview
        },
        status: if current {
            CapabilityStatus::Current
        } else {
            CapabilityStatus::NeedsReview
        },
        resources,
        registered_at: now.to_owned(),
    };
    registry
        .capabilities
        .retain(|known| known.name != ARCHITECTURE_CAPABILITY);
    registry.capabilities.push(capability);
}

fn trusted_relation_entry(registry: &KnowledgeRegistry, relation: &str) -> Option<String> {
    let matches = registry
        .entries
        .iter()
        .filter(|entry| {
            entry.relation == relation
                && entry
                    .versions
                    .last()
                    .is_some_and(|version| version.authority == Authority::Trusted)
        })
        .collect::<Vec<_>>();
    (matches.len() == 1).then(|| matches[0].entry_id.clone())
}

fn resolve_resource(
    context: &ManagedKnowledgeContext,
    registry: &KnowledgeRegistry,
    capability: &KnowledgeCapability,
    resource: &str,
) -> anyhow::Result<Option<PathBuf>> {
    let Some(entry_id) = capability.resources.get(resource) else {
        return Ok(None);
    };
    let Some(entry) = registry
        .entries
        .iter()
        .find(|entry| &entry.entry_id == entry_id)
    else {
        return Ok(None);
    };
    let Some(version) = entry.versions.last() else {
        return Ok(None);
    };
    if version.authority != Authority::Trusted {
        return Ok(None);
    }
    let source = context.root.join(&entry.source_path);
    let source_hash = fs::read(&source).ok().map(|bytes| sha256(&bytes));
    if source_hash.as_deref() != Some(version.sha256.as_str()) {
        return Ok(None);
    }
    if !safe_relative(&version.object_path) {
        anyhow::bail!(
            "invalid knowledge object path: {}",
            version.object_path.display()
        );
    }
    let object = context.vault_path.join(&version.object_path);
    let object_hash = fs::read(&object).ok().map(|bytes| sha256(&bytes));
    if object_hash.as_deref() != Some(version.sha256.as_str()) {
        return Ok(None);
    }
    Ok(Some(object))
}

fn not_applicable(
    reason: &str,
    capability_id: Option<String>,
    authority: Option<Authority>,
) -> ArchitectureCapabilityResolution {
    let receipt_id = digest_id(
        "kr",
        format!(
            "{}:{}",
            capability_id.as_deref().unwrap_or("absent"),
            reason
        )
        .as_bytes(),
    );
    ArchitectureCapabilityResolution {
        capability_id,
        authority,
        status: "not_applicable".to_owned(),
        reason: reason.to_owned(),
        receipt_id,
        catalog: None,
        baseline: None,
    }
}

fn load_registry(context: &ManagedKnowledgeContext) -> anyhow::Result<Option<KnowledgeRegistry>> {
    let path = registry_path(&context.vault_path);
    if !path.is_file() {
        return Ok(None);
    }
    let registry: KnowledgeRegistry = serde_json::from_slice(&fs::read(path)?)?;
    if registry.schema_version != SCHEMA_VERSION || registry.project_id != context.project_id {
        anyhow::bail!("knowledge registry schema or project identity mismatch");
    }
    if registry.entries.iter().any(|entry| {
        !safe_relative(&entry.source_path)
            || entry
                .versions
                .iter()
                .any(|version| !safe_relative(&version.object_path))
    }) {
        anyhow::bail!("knowledge registry contains an unsafe path");
    }
    Ok(Some(registry))
}

fn registry_path(vault: &Path) -> PathBuf {
    vault.join(INGESTION_ROOT).join("registry.json")
}

fn plan_path(vault: &Path, plan_id: &str) -> anyhow::Result<PathBuf> {
    let Some(hash) = plan_id.strip_prefix("kp-") else {
        anyhow::bail!("invalid knowledge plan id");
    };
    if hash.len() != 16 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("invalid knowledge plan id");
    }
    Ok(vault
        .join(INGESTION_ROOT)
        .join("plans")
        .join(format!("{plan_id}.json")))
}

fn object_path(sha256: &str) -> anyhow::Result<PathBuf> {
    let hash = sha256
        .strip_prefix("sha256:")
        .ok_or_else(|| anyhow::anyhow!("invalid SHA-256 reference"))?;
    if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!("invalid SHA-256 reference");
    }
    Ok(Path::new(INGESTION_ROOT).join("objects").join(hash))
}

fn safe_relative(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn plan_id(
    project_id: &str,
    source_commit: &str,
    candidates: &[ImportCandidate],
) -> anyhow::Result<String> {
    let material = serde_json::to_vec(&(project_id, source_commit, candidates))?;
    Ok(digest_id("kp", &material))
}

fn digest_id(prefix: &str, bytes: &[u8]) -> String {
    let digest = format!("{:x}", Sha256::digest(bytes));
    format!("{prefix}-{}", &digest[..16])
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn git_commit(root: &Path) -> String {
    git_output(root, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unversioned".to_owned())
}

fn git_tracked_files(root: &Path) -> BTreeSet<PathBuf> {
    let Some(output) = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    else {
        return BTreeSet::new();
    };
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| PathBuf::from(String::from_utf8_lossy(path).into_owned()))
        .collect()
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|output| !output.is_empty())
}

fn git_committed_sha256(root: &Path, path: &Path) -> Option<String> {
    let spec = format!("HEAD:{}", path.to_string_lossy().replace('\\', "/"));
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &spec])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| sha256(&output.stdout))
}

fn append_log(vault: &Path, message: &str) -> anyhow::Result<()> {
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(vault.join("_log.md"))?;
    writeln!(log, "- {} | {message}", crate::git_cmd::default_timestamp())?;
    Ok(())
}

fn count_disposition(candidates: &[ImportCandidate], disposition: Disposition) -> usize {
    candidates
        .iter()
        .filter(|candidate| candidate.disposition == disposition)
        .count()
}

fn render<T: Serialize>(
    result: anyhow::Result<T>,
    format: OutputFormat,
    text: fn(&T) -> String,
) -> CommandOutput {
    match result {
        Ok(output) => match format {
            OutputFormat::Text => CommandOutput {
                stdout: text(&output),
                ..CommandOutput::default()
            },
            OutputFormat::Json => match serde_json::to_string_pretty(&output) {
                Ok(json) => CommandOutput {
                    stdout: format!("{json}\n"),
                    ..CommandOutput::default()
                },
                Err(error) => crate::failure(error.to_string()),
            },
        },
        Err(error) => crate::failure(error.to_string()),
    }
}

fn scan_text(output: &ScanOutput) -> String {
    format!(
        "plan_id: {}\nplan_path: {}\nsource_commit: {}\ncandidates: {}\nimportable: {}\nunchanged: {}\nneeds_review: {}\nquarantined: {}\n",
        output.plan_id,
        output.plan_path.display(),
        output.source_commit,
        output.candidates,
        output.importable,
        output.unchanged,
        output.needs_review,
        output.quarantined
    )
}

fn import_text(output: &ImportOutput) -> String {
    format!(
        "plan_id: {}\nregistry_path: {}\nimported: {}\nunchanged: {}\nquarantined: {}\napproved: {}\ncapabilities_registered: {}\n",
        output.plan_id,
        output.registry_path.display(),
        output.imported,
        output.unchanged,
        output.quarantined,
        output.approved,
        output.capabilities_registered
    )
}

fn verify_text(output: &VerifyOutput) -> String {
    format!(
        "registry_present: {}\nvalid: {}\nentries: {}\nuntracked: {}\nincidences: {}\n",
        output.registry_present,
        output.valid,
        output.entries.len(),
        output.untracked.len(),
        output.incidences.len()
    )
}
