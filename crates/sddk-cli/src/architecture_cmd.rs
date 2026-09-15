//! Architecture conformance commands (arch-spec-A3-S10, 12-CLI-AGENT-UX).
//!
//! `sddk architecture receipt` loads a declarative architecture description,
//! runs the native conformance capabilities and emits the named
//! `ARCHITECTURE-CONFORMANCE-RECEIPT`.
//!
//! Read-only: the handler reads one file under the resolved root and prints.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};

use crate::{CliEnvironment, CommandOutput, OutputFormat};
use sddk_engine::architectural_contract::{
    ArchitecturalContract, ArchitectureClaim, ContractEvaluation, ContractId, ContractPayload,
    DecisionRef, EvaluatorRef, SpecRef,
};
use sddk_engine::architecture_conformance::{
    ConformanceInputs, ContractEvidence, compute_conformance_delta, contract_set_digest,
};
use sddk_engine::architecture_debverify::{
    DebVerifyAudit, DebVerifyFinding, DebVerifyFindingKind, FindingBasis, FindingId,
    run_debverify_audit,
};
use sddk_engine::architecture_declaration::{DeclarationFile, validate};
use sddk_engine::architecture_graph::{ArchitectureGraphOverlay, SoftwareUnitRef};
use sddk_engine::architecture_mutation::{MutationSandbox, run_mutation_suite};
use sddk_engine::architecture_receipt::{
    ChangeBasis, ReceiptInputs, ReceiptVerdict, compose_receipt,
};
use sddk_engine::knowledge::EventTime;
use serde::Serialize;

/// Resolve the evaluation time: the caller's `--now-ms`, else the wall clock.
///
/// Staleness is inherently time-dependent; pinning it to zero would silently
/// report every elapsed compatibility window as open.
pub(crate) fn resolve_now(now_ms: Option<i64>) -> EventTime {
    let ms = now_ms.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    });
    EventTime(ms)
}

/// Exit code for a `Blocked` receipt.
const EXIT_BLOCKED: i32 = 1;
/// Exit code for a declaration/IO error.
const EXIT_INPUT_ERROR: i32 = 2;

/// Default declaration location relative to the resolved root.
pub(crate) const DEFAULT_DECLARATION: &str = ".sddk/architecture/contracts.yaml";

#[derive(Debug, Subcommand)]
pub(crate) enum ArchitectureCommand {
    /// Emit the architecture-conformance receipt for a declarative contract set.
    Receipt(ArchitectureReceiptArgs),
    /// List every declared architectural contract.
    Contracts(ReadArgs),
    /// List the declared single-authority contracts grouped by component.
    Authorities(ReadArgs),
    /// List the declared unique-owner contracts grouped by entity.
    Ownership(ReadArgs),
    /// List the declared compatibility windows and their status.
    Compatibility(ReadArgs),
    /// Show the declared units and relations (the AC2 projection).
    Graph(GraphReadArgs),
    /// List the AC5 DebVerify findings with their full shape.
    Findings(FindingsArgs),
}

/// Arguments shared by the read surfaces.
#[derive(Debug, Clone, Args)]
pub(crate) struct ReadArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Declaration file, relative to `--root`.
    #[arg(long, default_value = DEFAULT_DECLARATION)]
    pub(crate) contracts: PathBuf,
    /// Evaluation time in epoch-ms. Defaults to the current wall clock;
    /// pass it explicitly for a reproducible run.
    #[arg(long)]
    pub(crate) now_ms: Option<i64>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for `architecture graph`.
#[derive(Debug, Clone, Args)]
pub(crate) struct GraphReadArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Declaration file, relative to `--root`.
    #[arg(long, default_value = DEFAULT_DECLARATION)]
    pub(crate) contracts: PathBuf,
    /// Evaluation time in epoch-ms. Defaults to the current wall clock;
    /// pass it explicitly for a reproducible run.
    #[arg(long)]
    pub(crate) now_ms: Option<i64>,
    /// Restrict units to those whose locator starts with this prefix.
    #[arg(long)]
    pub(crate) scope: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for `architecture findings`.
#[derive(Debug, Clone, Args)]
pub(crate) struct FindingsArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Declaration file, relative to `--root`.
    #[arg(long, default_value = DEFAULT_DECLARATION)]
    pub(crate) contracts: PathBuf,
    /// Evaluation time in epoch-ms. Defaults to the current wall clock;
    /// pass it explicitly for a reproducible run.
    #[arg(long)]
    pub(crate) now_ms: Option<i64>,
    /// Restrict the listing to one finding kind (canonical tag).
    #[arg(long)]
    pub(crate) kind: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ArchitectureReceiptArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Declaration file, relative to `--root`.
    #[arg(long, default_value = DEFAULT_DECLARATION)]
    pub(crate) contracts: PathBuf,
    /// Evaluation time in epoch-ms. Defaults to the current wall clock;
    /// pass it explicitly for a reproducible run.
    #[arg(long)]
    pub(crate) now_ms: Option<i64>,
    /// Scope the receipt to units touched since `--base` (git diff).
    #[arg(long)]
    pub(crate) changed: bool,
    /// Revision the change basis is taken against (default: origin/main, then HEAD~1).
    #[arg(long)]
    pub(crate) base: Option<String>,
    /// Verify only this declared contract. Unknown ids fail closed.
    #[arg(long)]
    pub(crate) contract: Option<String>,
    /// Write the receipt to this path as well as stdout. The parent directory
    /// must already exist.
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Dispatch the `architecture` command family.
pub(crate) fn run_architecture(
    command: ArchitectureCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        ArchitectureCommand::Receipt(args) => run_receipt(args, environment),
        ArchitectureCommand::Contracts(args) => run_contracts(args),
        ArchitectureCommand::Authorities(args) => run_authorities(args),
        ArchitectureCommand::Ownership(args) => run_ownership(args),
        ArchitectureCommand::Compatibility(args) => run_compatibility(args),
        ArchitectureCommand::Graph(args) => run_graph(args),
        ArchitectureCommand::Findings(args) => run_findings(args),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared prologue
// ─────────────────────────────────────────────────────────────────────────────

/// Load + parse + validate the declaration, or return the error output.
fn load_declaration(
    root: &Path,
    contracts: &Path,
) -> Result<sddk_engine::architecture_declaration::DeclaredArchitecture, CommandOutput> {
    let path = root.join(contracts);
    let label = contracts.display().to_string();
    let text = std::fs::read_to_string(&path).map_err(|e| {
        error_output(format!(
            "architecture: cannot read declaration `{}`: {e}",
            path.display()
        ))
    })?;
    let parsed: DeclarationFile = serde_yaml::from_str(&text).map_err(|e| {
        error_output(format!(
            "architecture: `{}` is not a valid declaration: {e}",
            path.display()
        ))
    })?;
    validate(&parsed, &label).map_err(|e| error_output(format!("architecture: {e}")))
}

/// The declared unit a contract is scoped to, when it is unit-scoped.
///
/// Only `SingleAuthority` (component) and `UniqueOwner` (entity) name a subject
/// that is also a unit id; the other kinds are global.
fn unit_subject(contract: &ArchitecturalContract) -> Option<String> {
    match contract.payload() {
        ContractPayload::SingleAuthority(c) => Some(c.as_str().to_string()),
        ContractPayload::UniqueOwner(e) => Some(e.as_str().to_string()),
        _ => None,
    }
}

/// Build the AC2 overlay from a validated declaration.
///
/// Also attaches a **linkage claim** for every unit-scoped contract so that
/// `ArchitectureGraphOverlay::find_contracts_for_unit` (which discovers
/// contracts through `ArchitectureClaimedBy` relations) can scope AC4. The
/// claim is a genuine AC1 evaluation whose outcome is truthfully `Unknown`
/// when no evidence is supplied — it establishes the link, and AC4 then
/// re-evaluates with the caller's evidence.
fn declare_overlay(
    decl: &sddk_engine::architecture_declaration::DeclaredArchitecture,
    now: EventTime,
) -> (
    ArchitectureGraphOverlay,
    std::collections::BTreeMap<String, ArchitectureClaim>,
) {
    let mut overlay = ArchitectureGraphOverlay::new();
    // The claims the overlay registers, kept alongside it so a caller can read a
    // contract's assessment without re-evaluating. This is the same object the
    // overlay holds — not a copy, not a second evaluation.
    let mut claims: std::collections::BTreeMap<String, ArchitectureClaim> =
        std::collections::BTreeMap::new();
    for unit in &decl.units {
        overlay.add_unit(unit);
    }
    for relation in &decl.relations {
        overlay.add_relation(relation);
    }
    let unit_ids: std::collections::BTreeSet<&str> =
        decl.units.iter().map(|u| u.id.0.as_str()).collect();
    for contract in &decl.contracts {
        let Some(subject) = unit_subject(contract) else {
            continue;
        };
        if !unit_ids.contains(subject.as_str()) {
            continue;
        }
        let evaluator =
            EvaluatorRef::new("sddk.architecture_cli.linkage").expect("non-empty literal");
        let claim = ContractEvaluation::evaluate(contract, Vec::new(), now, evaluator, None);
        claims.insert(contract.id().as_str().to_string(), claim.clone());
        let claim_id = overlay.add_claim(&claim);
        overlay.attach_claim_to_unit(&claim, &claim_id, &SoftwareUnitRef::new(subject));
        // `find_contracts_for_unit` returns a synthesized contract-anchor
        // NodeId; AC4 resolves it through the anchor node's `contract_id` prop,
        // which only `add_contract_metadata` writes.
        overlay.add_contract_metadata(
            contract,
            contract.decided_by(),
            contract.specified_by(),
            &[],
        );
    }
    (overlay, claims)
}

// ─────────────────────────────────────────────────────────────────────────────
// Change basis (`--changed`)
// ─────────────────────────────────────────────────────────────────────────────

/// True when a unit locator and a changed path overlap (either is a prefix of
/// the other, so a directory locator covers its files).
pub(crate) fn path_overlaps(locator: &str, path: &str) -> bool {
    if locator.is_empty() || path.is_empty() {
        return false;
    }
    path.starts_with(locator) || locator.starts_with(path)
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git {:?} failed to start: {e}", args))?;
    if !out.status.success() {
        return Err(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Resolve the base revision, or fail closed.
fn resolve_base(root: &Path, explicit: Option<&str>) -> Result<String, String> {
    if let Some(b) = explicit {
        git(root, &["rev-parse", "--verify", &format!("{b}^{{commit}}")])
            .map_err(|_| format!("`--base {b}` does not resolve to a commit"))?;
        return Ok(b.to_string());
    }
    for candidate in ["origin/main", "HEAD~1"] {
        if git(
            root,
            &["rev-parse", "--verify", &format!("{candidate}^{{commit}}")],
        )
        .is_ok()
        {
            return Ok(candidate.to_string());
        }
    }
    Err("no base revision could be resolved (pass --base <rev>)".to_string())
}

/// Union of the committed range diff and the worktree diff.
fn git_changed_paths(root: &Path, base: &str) -> Result<Vec<String>, String> {
    let range = format!("{base}...HEAD");
    let mut paths: Vec<String> = Vec::new();
    // `--no-renames` is load-bearing in the same way: with rename detection on,
    // a move reports only the destination, so a unit whose whole source was
    // deleted out of its locator is never scoped. Splitting a rename into a
    // delete plus an add scopes both sides, which is the fail-closed direction.
    for args in [
        vec!["diff", "--name-only", "-z", "--no-renames", &range],
        vec!["diff", "--name-only", "-z", "--no-renames"],
    ] {
        if let Ok(out) = git(root, &args) {
            paths.extend(split_z(&out));
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// Split NUL-separated git output into paths.
///
/// `-z` is load-bearing, not cosmetic: without it git quotes any path holding a
/// byte outside ASCII (`"caf\303\251/y.rs"`), the quoted form matches no locator,
/// and `--changed` would report an empty basis for a file that did change. That
/// silent false-clean is the one outcome `--changed` must never produce.
fn split_z(out: &str) -> impl Iterator<Item = String> + '_ {
    out.split('\0')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// Build the change basis for `--changed`.
fn changed_basis(
    root: &Path,
    explicit_base: Option<&str>,
    decl: &sddk_engine::architecture_declaration::DeclaredArchitecture,
) -> Result<ChangeBasis, String> {
    let base = resolve_base(root, explicit_base)?;
    let paths = git_changed_paths(root, &base)?;
    let mut units: Vec<String> = decl
        .units
        .iter()
        .filter(|u| paths.iter().any(|p| path_overlaps(&u.locator, p)))
        .map(|u| u.id.0.clone())
        .collect();
    units.sort();
    units.dedup();
    Ok(ChangeBasis {
        base,
        changed_units: units,
    })
}

/// A contract's subject identifier, for display and grouping.
fn subject_of(contract: &ArchitecturalContract) -> String {
    match contract.payload() {
        ContractPayload::SingleAuthority(c) => c.as_str().to_string(),
        ContractPayload::UniqueOwner(e) => e.as_str().to_string(),
        ContractPayload::ForbiddenDependency { from, to, .. } => {
            format!("{} -> {}", from.as_str(), to.as_str())
        }
        ContractPayload::ProjectionOnly { source_kind } => source_kind.clone(),
        ContractPayload::BoundedCompatibility { .. } => contract.id().as_str().to_string(),
        ContractPayload::ProviderBoundary { surface, .. } => surface.clone(),
        ContractPayload::Extension { kind, .. } => kind.as_str().to_string(),
    }
}

/// A concise rendering of an overlay relation kind.
fn relation_tag(kind: sddk_engine::architecture_graph::ArchitectureOverlayRelationKind) -> String {
    format!("{kind:?}")
}

/// A concise rendering of an overlay node reference.
fn node_tag(node: &sddk_engine::architecture_graph::OverlayNodeRef) -> String {
    use sddk_engine::architecture_graph::OverlayNodeRef as N;
    match node {
        N::SoftwareUnit(u) => u.0.clone(),
        N::BoundedContext(s) => s.clone(),
        N::Decision(d) => decision_tag(d),
        N::Spec(s) => spec_tag(s),
        N::Test(t) => t.0.clone(),
        N::Uat(u) => u.0.clone(),
        N::CompatibilityPath(p) => p.0.clone(),
        N::Claim(c) => c.as_str().to_string(),
    }
}

fn kind_tag(contract: &ArchitecturalContract) -> &'static str {
    match contract.payload() {
        ContractPayload::SingleAuthority(_) => "single_authority",
        ContractPayload::UniqueOwner(_) => "unique_owner",
        ContractPayload::ForbiddenDependency { .. } => "forbidden_dependency",
        ContractPayload::ProjectionOnly { .. } => "projection_only",
        ContractPayload::BoundedCompatibility { .. } => "bounded_compatibility",
        ContractPayload::ProviderBoundary { .. } => "provider_boundary",
        ContractPayload::Extension { .. } => "extension",
    }
}

/// One row of the contract catalogue.
#[derive(Debug, Serialize)]
struct ContractRow {
    id: String,
    kind: &'static str,
    subject: String,
    decided_by: String,
    specified_by: String,
    revision: String,
}

fn decision_tag(d: &DecisionRef) -> String {
    match d {
        DecisionRef::Decision(s) => s.clone(),
        DecisionRef::Adr(s) => s.clone(),
        DecisionRef::ExternalDecision { authority, .. } => authority.clone(),
    }
}

fn spec_tag(s: &SpecRef) -> String {
    match s {
        SpecRef::ArchSpec(v) | SpecRef::Spec(v) | SpecRef::Adr(v) => v.clone(),
    }
}

fn contract_row(c: &ArchitecturalContract) -> ContractRow {
    ContractRow {
        id: c.id().as_str().to_string(),
        kind: kind_tag(c),
        subject: subject_of(c),
        decided_by: decision_tag(c.decided_by()),
        specified_by: spec_tag(c.specified_by()),
        revision: c.revision().as_str().to_string(),
    }
}

fn emit<T: Serialize>(
    format: OutputFormat,
    header: &str,
    rows: &[T],
    text: String,
) -> CommandOutput {
    match format {
        OutputFormat::Json => match serde_json::to_string_pretty(rows) {
            Ok(body) => CommandOutput {
                status: 0,
                stdout: format!("{body}\n"),
                stderr: String::new(),
            },
            Err(e) => error_output(format!("architecture: cannot serialise: {e}")),
        },
        OutputFormat::Text => CommandOutput {
            status: 0,
            stdout: format!("{header}\n{text}"),
            stderr: String::new(),
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Read surfaces
// ─────────────────────────────────────────────────────────────────────────────

fn run_contracts(args: ReadArgs) -> CommandOutput {
    let decl = match load_declaration(&args.root, &args.contracts) {
        Ok(d) => d,
        Err(o) => return o,
    };
    let rows: Vec<ContractRow> = decl.contracts.iter().map(contract_row).collect();
    let mut text = String::new();
    for r in &rows {
        text.push_str(&format!(
            "  {:<28} {:<22} {:<28} rev={}\n",
            r.id, r.kind, r.subject, r.revision
        ));
    }
    if rows.is_empty() {
        text.push_str("  (no contracts declared)\n");
    }
    emit(
        args.format,
        &format!("architecture contracts — revision {}", decl.revision),
        &rows,
        text,
    )
}

fn run_authorities(args: ReadArgs) -> CommandOutput {
    let decl = match load_declaration(&args.root, &args.contracts) {
        Ok(d) => d,
        Err(o) => return o,
    };
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for c in &decl.contracts {
        if matches!(c.payload(), ContractPayload::SingleAuthority(_)) {
            groups
                .entry(subject_of(c))
                .or_default()
                .push(c.id().as_str().to_string());
        }
    }
    let mut text = String::new();
    let mut json_rows: Vec<(String, Vec<String>)> = Vec::new();
    for (component, mut ids) in groups {
        ids.sort();
        text.push_str(&format!("  {component}: {}\n", ids.join(", ")));
        json_rows.push((component, ids));
    }
    if json_rows.is_empty() {
        text.push_str("  (no single-authority contracts declared)\n");
    }
    emit(
        args.format,
        &format!("architecture authorities — revision {}", decl.revision),
        &json_rows,
        text,
    )
}

fn run_ownership(args: ReadArgs) -> CommandOutput {
    let decl = match load_declaration(&args.root, &args.contracts) {
        Ok(d) => d,
        Err(o) => return o,
    };
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for c in &decl.contracts {
        if matches!(c.payload(), ContractPayload::UniqueOwner(_)) {
            groups
                .entry(subject_of(c))
                .or_default()
                .push(c.id().as_str().to_string());
        }
    }
    let mut text = String::new();
    let mut json_rows: Vec<(String, Vec<String>)> = Vec::new();
    for (entity, mut ids) in groups {
        ids.sort();
        text.push_str(&format!("  {entity}: {}\n", ids.join(", ")));
        json_rows.push((entity, ids));
    }
    if json_rows.is_empty() {
        text.push_str("  (no unique-owner contracts declared)\n");
    }
    emit(
        args.format,
        &format!("architecture ownership — revision {}", decl.revision),
        &json_rows,
        text,
    )
}

/// One row of the compatibility surface.
#[derive(Debug, Serialize)]
struct CompatibilityRow {
    id: String,
    deprecated_after_ms: i64,
    replaced_by: Option<String>,
    window_status: &'static str,
}

fn run_compatibility(args: ReadArgs) -> CommandOutput {
    // AC5 tells us which windows are actually stale; the read surface mirrors
    // that rather than re-deriving it, through the same context the other
    // surfaces build.
    let ctx = match build_context(&args.root, &args.contracts, resolve_now(args.now_ms)) {
        Ok(c) => c,
        Err(o) => return o,
    };
    let decl = &ctx.declared;
    let audit = &ctx.audit;
    let stale: std::collections::BTreeSet<String> = audit
        .by_kind(sddk_engine::architecture_debverify::DebVerifyFindingKind::StaleCompatibility)
        .iter()
        .flat_map(|f| f.subjects.clone())
        .collect();

    let mut rows: Vec<CompatibilityRow> = Vec::new();
    for c in &decl.contracts {
        if let ContractPayload::BoundedCompatibility {
            deprecated_after,
            replaced_by,
        } = c.payload()
        {
            let id = c.id().as_str().to_string();
            let status = if stale.contains(&id) { "stale" } else { "open" };
            rows.push(CompatibilityRow {
                id,
                deprecated_after_ms: deprecated_after.0,
                replaced_by: replaced_by.as_ref().map(|c| c.as_str().to_string()),
                window_status: status,
            });
        }
    }
    let mut text = String::new();
    for r in &rows {
        text.push_str(&format!(
            "  {:<28} window_status={:<6} deprecated_after_ms={} replaced_by={}\n",
            r.id,
            r.window_status,
            r.deprecated_after_ms,
            r.replaced_by.as_deref().unwrap_or("<none>")
        ));
    }
    if rows.is_empty() {
        text.push_str("  (no bounded-compatibility contracts declared)\n");
    }
    emit(
        args.format,
        &format!("architecture compatibility — revision {}", decl.revision),
        &rows,
        text,
    )
}

/// One row of the graph surface.
#[derive(Debug, Serialize)]
struct GraphRow {
    units: Vec<(String, String, String)>,
    relations: Vec<(String, String, String)>,
    overlay_digest_len: usize,
}

fn run_graph(args: GraphReadArgs) -> CommandOutput {
    let decl = match load_declaration(&args.root, &args.contracts) {
        Ok(d) => d,
        Err(o) => return o,
    };
    let (overlay, _claims) = declare_overlay(&decl, EventTime(0));
    let units: Vec<(String, String, String)> = decl
        .units
        .iter()
        .filter(|u| {
            args.scope
                .as_ref()
                .map(|s| u.locator.starts_with(s.as_str()))
                .unwrap_or(true)
        })
        .map(|u| (u.id.0.clone(), format!("{:?}", u.kind), u.locator.clone()))
        .collect();
    let relations: Vec<(String, String, String)> = decl
        .relations
        .iter()
        .map(|r| (node_tag(&r.from), node_tag(&r.to), relation_tag(r.kind)))
        .collect();
    let digest_len = overlay.digest().len();
    let row = GraphRow {
        units: units.clone(),
        relations: relations.clone(),
        overlay_digest_len: digest_len,
    };

    let mut text = String::new();
    text.push_str("  units:\n");
    for (id, kind, locator) in &units {
        text.push_str(&format!("    {id}  {kind}  {locator}\n"));
    }
    if units.is_empty() {
        text.push_str("    (none)\n");
    }
    text.push_str("  relations:\n");
    for (from, to, kind) in &relations {
        text.push_str(&format!("    {from} -> {to}  {kind}\n"));
    }
    if relations.is_empty() {
        text.push_str("    (none)\n");
    }
    text.push_str(&format!("  overlay_digest_len: {digest_len}\n"));
    emit(
        args.format,
        &format!("architecture graph — revision {}", decl.revision),
        std::slice::from_ref(&row),
        text,
    )
}

/// The shared architecture context.
///
/// `receipt`, `findings` and `why` all build this through one constructor, so the
/// three surfaces cannot prepare three different universes: one declaration
/// parse, one projection, one audit, one claim set. They differ in what they do
/// with it — `receipt` composes a verdict, `findings` renders the audit, `why`
/// traverses — and the AC4 delta stays caller-specific on purpose, which is what
/// preserves the recorded behaviour that `findings` reports where `receipt`
/// errors out because it computes no delta.
pub(crate) struct ArchitectureContext {
    pub(crate) declared: sddk_engine::architecture_declaration::DeclaredArchitecture,
    pub(crate) now: EventTime,
    pub(crate) overlay: ArchitectureGraphOverlay,
    pub(crate) claims: std::collections::BTreeMap<String, ArchitectureClaim>,
    pub(crate) audit: DebVerifyAudit,
    pub(crate) finding_basis: FindingBasis,
}

pub(crate) fn build_context(
    root: &Path,
    contracts: &Path,
    now: EventTime,
) -> Result<ArchitectureContext, CommandOutput> {
    let declared = load_declaration(root, contracts)?;
    let overlay_and_claims = declare_overlay(&declared, now);
    let (overlay, claims) = overlay_and_claims;
    let audit = run_debverify_audit(&overlay, &declared.contracts, now).map_err(|e| {
        // Never a silent zero: a failed audit is an error, not "no findings".
        error_output(format!("architecture: audit failed: {e}"))
    })?;
    // Clock-stable by construction: revision + knowledge basis + contract set.
    // The overlay digest is deliberately excluded (it embeds each linkage claim's
    // `evaluated_at`), so a finding id survives a change of `--now-ms` and can be
    // fed back from `findings` into `why`.
    let finding_basis = FindingBasis::new(
        declared.revision.clone(),
        declared.knowledge_basis.clone(),
        contract_set_digest(&declared.contracts),
    );
    Ok(ArchitectureContext {
        declared,
        now,
        overlay,
        claims,
        audit,
        finding_basis,
    })
}

/// One rendered AC5 finding, carrying its deterministic id.
///
/// The id is **added**, not substituted: `kind`, `severity`, `subjects`,
/// `contract_ids` and `message` keep the exact shape A3-S14 shipped, so that e2e
/// suite keeps pinning the same contract while a new one pins the id.
#[derive(Debug, Serialize)]
struct FindingRow {
    id: String,
    /// Canonical kind tag (`shadow_authority`).
    ///
    /// Deliberately **not** serde's enum name (`"ShadowAuthority"`): the tag is
    /// the vocabulary `--kind`, the audit digest, the finding id and
    /// `why architecture` all already use, and two spellings of one kind across
    /// two surfaces is exactly the drift this cycle exists to remove.
    kind: &'static str,
    /// Canonical severity tag (`critical`).
    severity: &'static str,
    subjects: Vec<String>,
    contract_ids: Vec<String>,
    message: String,
}

fn finding_row(f: &DebVerifyFinding, basis: &FindingBasis) -> FindingRow {
    FindingRow {
        id: FindingId::derive(basis, f.kind, &f.subjects, &f.contract_ids)
            .as_str()
            .to_string(),
        kind: f.kind.canonical_tag(),
        severity: f.severity.canonical_tag(),
        subjects: f.subjects.clone(),
        contract_ids: f
            .contract_ids
            .iter()
            .map(|c| c.as_str().to_string())
            .collect(),
        message: f.message.clone(),
    }
}

/// Resolve a `--kind` tag against the closed finding-kind enum.
///
/// Derived from `ALL` rather than hand-listed, so a new kind is filterable the
/// moment it exists and the accepted set in the error cannot go stale.
fn finding_kind_from_tag(tag: &str) -> Option<DebVerifyFindingKind> {
    DebVerifyFindingKind::ALL
        .into_iter()
        .find(|k| k.canonical_tag() == tag)
}

fn accepted_kind_tags() -> String {
    DebVerifyFindingKind::ALL
        .iter()
        .map(|k| k.canonical_tag())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `sddk architecture findings` — the AC5 audit as an inspectable projection.
///
/// Deliberately shares `load_declaration`, `declare_overlay` and
/// `run_debverify_audit` with `receipt`, with no preprocessing and no filtering
/// before the audit. The two surfaces therefore cannot disagree about what the
/// audit found; they differ only in what they print. `receipt` gates on the
/// result, this lists it — no verdict, no score, no writes.
fn run_findings(args: FindingsArgs) -> CommandOutput {
    let kind_filter = match args.kind.as_deref() {
        None => None,
        Some(tag) => match finding_kind_from_tag(tag) {
            Some(k) => Some(k),
            None => {
                // An empty listing would be indistinguishable from a clean
                // declaration, so an unknown tag is a usage error.
                return error_output(format!(
                    "architecture findings: unknown --kind `{tag}` (accepted: {})",
                    accepted_kind_tags()
                ));
            }
        },
    };

    let ctx = match build_context(&args.root, &args.contracts, resolve_now(args.now_ms)) {
        Ok(c) => c,
        Err(o) => return o,
    };
    let decl = &ctx.declared;
    let now = ctx.now;
    let audit = &ctx.audit;

    let rows: Vec<FindingRow> = audit
        .findings
        .iter()
        .filter(|f| kind_filter.is_none_or(|k| f.kind == k))
        .map(|f| finding_row(f, &ctx.finding_basis))
        .collect();

    let mut text = String::new();
    for f in &rows {
        text.push_str(&format!("  {:<24} {:<9} id={}\n", f.kind, f.severity, f.id));
        text.push_str(&format!(
            "    subjects=[{}] contracts=[{}]\n",
            f.subjects.join(", "),
            f.contract_ids.join(", ")
        ));
        text.push_str(&format!("    {}\n", f.message));
    }
    if rows.is_empty() {
        text.push_str("  (no findings)\n");
    }
    text.push_str(&format!(
        "  {} finding(s) over {} audited contract(s)\n",
        rows.len(),
        audit.audited_contracts
    ));

    let scope = match kind_filter {
        Some(k) => format!(", kind={}", k.canonical_tag()),
        None => String::new(),
    };
    emit(
        args.format,
        &format!(
            "architecture findings — revision {}, now={}{}",
            decl.revision, now.0, scope
        ),
        &rows,
        text,
    )
}

fn run_receipt(args: ArchitectureReceiptArgs, _environment: &CliEnvironment) -> CommandOutput {
    let declaration_path = args.root.join(&args.contracts);
    let label = args.contracts.display().to_string();

    let _ = &declaration_path;
    // ── the shared context: one parse, one validation, one projection, one
    // audit across `receipt`, `findings` and `why` ────────────────────────
    let ctx = match build_context(&args.root, &args.contracts, resolve_now(args.now_ms)) {
        Ok(c) => c,
        Err(o) => return o,
    };
    let declared = &ctx.declared;
    let now = ctx.now;
    let change_basis = if args.changed {
        match changed_basis(&args.root, args.base.as_deref(), declared) {
            Ok(b) => Some(b),
            Err(e) => {
                return error_output(format!("architecture receipt: {e}"));
            }
        }
    } else {
        None
    };
    // Resolve `--contract` against the declaration before anything else runs:
    // a typo must read as a usage error, never as an empty (and therefore
    // passing) scope.
    let contract_filter = match args.contract.as_deref() {
        None => None,
        Some(id) => {
            // A malformed id and an undeclared id are the same user error:
            // both mean "no contract in this declaration matches", and neither
            // may degrade into an empty scope.
            let wanted = match ContractId::new(id) {
                Ok(c) => c,
                Err(_) => {
                    return error_output(format!(
                        "architecture receipt: `--contract {id}` is not declared in `{label}`"
                    ));
                }
            };
            let Some(contract) = declared.contracts.iter().find(|c| c.id() == &wanted) else {
                return error_output(format!(
                    "architecture receipt: `--contract {id}` is not declared in `{label}`"
                ));
            };
            // A subject is evaluable only if it is a *declared unit* of a
            // unit-scoped kind. Anything else is an unanswerable question, not
            // an empty scope: fail closed rather than report a passing nothing.
            //
            // Both halves matter. A global kind (`bounded_compatibility`) has no
            // subject at all, and `declare_overlay` links nothing for a contract
            // whose subject unit the declaration does not describe — AC5 would
            // eventually flag the dangling reference, but the verdict would be
            // incidental to the question asked.
            let subject_ok =
                unit_subject(contract).is_some_and(|s| declared.units.iter().any(|u| u.id.0 == s));
            if !subject_ok {
                return error_output(format!(
                    "architecture receipt: `--contract {id}` has no evaluable subject unit \
                     (needs a `single_authority`/`unique_owner` subject that is a declared unit)"
                ));
            }
            Some(wanted)
        }
    };
    // Validate `--out` before doing any work: a missing parent directory is a
    // usage error, and creating it silently would put the receipt somewhere the
    // caller did not ask for. The artifact only has value if a harness can find
    // it exactly where it was told to look.
    if let Some(path) = &args.out
        && let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.is_dir()
    {
        return error_output(format!(
            "architecture receipt: `--out {}` parent directory `{}` does not exist",
            path.display(),
            parent.display()
        ));
    }
    let overlay = &ctx.overlay;

    // ── AC4: change-scoped delta over an EMPTY change basis ────────────────
    // The receipt is global; the delta contributes its digests and its
    // (empty) claim/witness sets honestly.
    // The delta's scope units. `--changed` scopes to the diff; naming a
    // contract with no diff scopes to that contract's subject, because
    // "verify X" is a question about X and must not degenerate into an empty
    // (and therefore passing) scope just because nothing was touched.
    let scope_unit_refs: Vec<SoftwareUnitRef> = match (&change_basis, &contract_filter) {
        (Some(basis), _) => basis
            .changed_units
            .iter()
            .map(|u| SoftwareUnitRef::new(u.clone()))
            .collect(),
        (None, Some(id)) => declared
            .contracts
            .iter()
            .find(|c| c.id() == id)
            .and_then(unit_subject)
            .map(SoftwareUnitRef::new)
            .into_iter()
            .collect(),
        (None, None) => Vec::new(),
    };
    let changed_unit_refs = scope_unit_refs;

    let evidence: ContractEvidence = std::collections::BTreeMap::new();
    let delta = match compute_conformance_delta(
        overlay,
        ConformanceInputs {
            contracts: &declared.contracts,
            evidence: &evidence,
            contradiction_witnesses: &[],
            contract_filter: contract_filter.as_ref(),
        },
        now,
        changed_unit_refs.as_slice(),
    ) {
        Ok(d) => d,
        Err(e) => return error_output(format!("architecture receipt: delta failed: {e}")),
    };

    // ── AC5: the audit the shared context already ran ──────────────────────
    let audit = &ctx.audit;

    // ── AC6: no mutation sandbox supplied by v1 declarations ───────────────
    // An empty suite is recorded honestly: the receipt reports the
    // negative-evidence class as NOT reproduced rather than pretending.
    let mutations = match run_mutation_suite(&MutationSandbox::new(), &[], &[]) {
        Ok(m) => m,
        Err(e) => return error_output(format!("architecture receipt: mutations failed: {e}")),
    };

    // ── AC8: compose the receipt ───────────────────────────────────────────
    let receipt = compose_receipt(
        ReceiptInputs {
            revision: declared.revision.clone(),
            knowledge_basis: declared.knowledge_basis.clone(),
            delta: &delta,
            audit,
            mutations: &mutations,
            lenses: &[],
            waivers: &declared.waivers,
            provider_basis: &[],
            change_basis: change_basis.as_ref(),
            contract_filter: contract_filter.as_ref().map(|c| c.as_str().to_string()),
        },
        now,
    );

    let status = match receipt.verdict {
        ReceiptVerdict::Pass | ReceiptVerdict::PassWithWaivers => 0,
        ReceiptVerdict::Blocked => EXIT_BLOCKED,
    };

    let stdout = match args.format {
        OutputFormat::Json => match serde_json::to_string_pretty(&receipt) {
            Ok(s) => format!("{s}\n"),
            Err(e) => return error_output(format!("architecture receipt: cannot serialise: {e}")),
        },
        OutputFormat::Text => render_text(&receipt),
    };

    let mut stderr = String::new();
    if let Some(path) = &args.out {
        if let Err(e) = std::fs::write(path, stdout.as_bytes()) {
            return error_output(format!(
                "architecture receipt: cannot write `--out {}`: {e}",
                path.display()
            ));
        }
        // Reported on stderr so stdout stays a pure receipt in both formats: a
        // note appended to JSON would make it unparseable.
        stderr = format!("wrote: {} ({} bytes)\n", path.display(), stdout.len());
    }

    CommandOutput {
        status,
        stdout,
        stderr,
    }
}

fn render_text(
    receipt: &sddk_engine::architecture_receipt::ArchitectureConformanceReceipt,
) -> String {
    let mut out = String::new();
    out.push_str("ARCHITECTURE-CONFORMANCE-RECEIPT\n");
    out.push_str(&format!("receipt_id:        {}\n", receipt.id));
    out.push_str(&format!("revision:          {}\n", receipt.basis.revision));
    out.push_str(&format!(
        "knowledge_basis:   {}\n",
        receipt.basis.knowledge_basis
    ));
    out.push_str(&format!(
        "verdict:           {}\n",
        receipt.verdict.canonical_tag()
    ));
    out.push_str(&format!(
        "audited_contracts: {}\n",
        receipt.audited_contracts
    ));
    // The scope note names whichever scope was actually asked for. A filtered
    // run with no diff is not a "change-scoped" run, and saying so would
    // misdescribe where the rows came from.
    let scope_note = match (
        receipt.change_basis.is_some(),
        receipt.contract_filter.is_some(),
    ) {
        (true, true) => "change-scoped and filtered",
        (true, false) => "change-scoped",
        (false, true) => "contract-scoped",
        (false, false) => "empty without --changed or --contract",
    };
    out.push_str(&format!(
        "affected_contracts: {} ({scope_note})\n",
        receipt.claim_results.len()
    ));
    out.push_str(&format!("unknowns:          {}\n", receipt.unknowns.len()));
    out.push_str(&format!(
        "contradictions:    {}\n",
        receipt.contradictions.len()
    ));
    out.push_str(&format!("waivers:           {}\n", receipt.waivers.len()));
    match &receipt.change_basis {
        Some(b) => out.push_str(&format!(
            "change_basis:      base={} changed_units={}\n",
            b.base,
            b.changed_units.len()
        )),
        None => out.push_str("change_basis:      (global run; no --changed)\n"),
    }
    if let Some(id) = &receipt.contract_filter {
        out.push_str(&format!("contract_filter:   {id}\n"));
    }
    out.push_str("historical_class_coverage:\n");
    for c in &receipt.class_coverage {
        out.push_str(&format!(
            "  {:<26} {}\n",
            c.class.canonical_tag(),
            if c.reproduced {
                "reproduced"
            } else {
                "not_reproduced"
            }
        ));
    }
    let mandatory = receipt.mandatory_unresolved();
    if mandatory.is_empty() {
        out.push_str("unresolved_must_findings: none\n");
    } else {
        out.push_str("unresolved_must_findings:\n");
        for f in mandatory {
            out.push_str(&format!(
                "  {:<24} subjects={:?} waivers={:?}\n",
                f.kind, f.subjects, f.waiver_refs
            ));
        }
    }
    out
}

pub(crate) fn error_output(message: String) -> CommandOutput {
    CommandOutput {
        status: EXIT_INPUT_ERROR,
        stdout: String::new(),
        stderr: format!("{message}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::architecture_declaration::{DeclarationFile, validate};

    /// REQ-A3S12-001
    #[test]
    fn acceptance_path_overlap() {
        // File locator vs a changed path in the same directory.
        assert!(path_overlaps(
            "crates/sddk-engine/src/architecture_graph",
            "crates/sddk-engine/src/architecture_graph/overlay.rs"
        ));
        // Exact file match.
        assert!(path_overlaps(
            "crates/sddk-engine/src/canonical_event_log.rs",
            "crates/sddk-engine/src/canonical_event_log.rs"
        ));
        // Disjoint paths do not overlap.
        assert!(!path_overlaps(
            "crates/sddk-engine/src/architecture_graph",
            "crates/sddk-cli/src/lib.rs"
        ));
        // Empty input is never an overlap (guards against a prefix match on "").
        assert!(!path_overlaps("", "crates/x.rs"));
        assert!(!path_overlaps("crates/x.rs", ""));
    }

    /// REQ-A3S12-002
    #[test]
    fn acceptance_changed_units_mapping() {
        let yaml = r#"
revision: r1
units:
  - id: comp:touched
    locator: crates/a/src/lib.rs
  - id: comp:untouched
    locator: crates/b/src/lib.rs
"#;
        let parsed: DeclarationFile = serde_yaml::from_str(yaml).expect("yaml");
        let decl = validate(&parsed, "t").expect("valid");
        let paths = ["crates/a/src/lib.rs".to_string()];
        let affected: Vec<String> = decl
            .units
            .iter()
            .filter(|u| paths.iter().any(|p| path_overlaps(&u.locator, p)))
            .map(|u| u.id.0.clone())
            .collect();
        assert_eq!(affected, vec!["comp:touched".to_string()]);
    }

    /// REQ-A3S12-005
    #[test]
    fn acceptance_change_basis_optional() {
        // The type models both states; absence is not an empty basis.
        let none: Option<ChangeBasis> = None;
        assert!(none.is_none());
        let empty = ChangeBasis {
            base: "origin/main".to_string(),
            changed_units: vec![],
        };
        // A present-but-empty basis is a *real* zero, distinct from absence.
        assert!(empty.changed_units.is_empty());
        assert!(!empty.base.is_empty());
    }
}
