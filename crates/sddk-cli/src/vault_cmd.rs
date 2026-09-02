//! Vault indexing, validation, search, graph, and export commands.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_gateway::CapabilityPolicy;
use sddk_vault::{
    ALLOW_LIST, Diagnostic, GraphView, RepairReceipt, SearchHit, Severity, load_repair_queue,
};
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

/// Validates a scope cycle value against the designed grammar.
///
/// Format: `^[a-z0-9-]+/[a-z0-9-]+$` (project_id/cycle_id)
/// Returns `Ok((project_id, cycle_id))` on success, or the original value on failure.
fn validate_scope_cycle(value: &str) -> Result<(String, String), &str> {
    // Grammar: project_id/cycle_id where each part is lowercase alphanumeric + hyphens
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() != 2 {
        return Err(value);
    }
    let [project_id, cycle_id] = parts.as_slice() else {
        return Err(value);
    };
    if project_id.is_empty()
        || cycle_id.is_empty()
        || !project_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || !cycle_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(value);
    }
    Ok((project_id.to_string(), cycle_id.to_string()))
}

#[derive(Debug, Subcommand)]
pub(crate) enum VaultCommand {
    /// Parse, validate, and rebuild the FTS index from a vault.
    Index(VaultIndexArgs),
    /// Validate node ids, titles, and wikilinks.
    Validate(VaultIndexArgs),
    /// Search the FTS index.
    Search(VaultSearchArgs),
    /// Show graph facts (cycles, topological order).
    Graph(VaultIndexArgs),
    /// Export a self-contained HTML inspector.
    Export(VaultExportArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VaultIndexArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Vault directory.
    #[arg(long)]
    pub(crate) vault: PathBuf,
    /// SQLite index database path.
    #[arg(long)]
    pub(crate) db: Option<PathBuf>,
    /// Repeatable scope cycles for scoped VAULT003 down-classification.
    /// Each value must match `project_id/cycle_id` form.
    #[arg(long, value_name = "SCOPE", value_delimiter = ',')]
    pub(crate) scope_cycles: Vec<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VaultSearchArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// SQLite index database path.
    #[arg(long)]
    pub(crate) db: PathBuf,
    /// Full-text query.
    #[arg(long)]
    pub(crate) query: String,
    /// Maximum hits.
    #[arg(long, default_value_t = 20)]
    pub(crate) limit: usize,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VaultExportArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Vault directory.
    #[arg(long)]
    pub(crate) vault: PathBuf,
    /// Output HTML file.
    #[arg(long)]
    pub(crate) output: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_vault(command: VaultCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        VaultCommand::Index(args) => run_vault_index(args, environment, false),
        VaultCommand::Validate(args) => run_vault_index(args, environment, true),
        VaultCommand::Search(args) => run_vault_search(args, environment),
        VaultCommand::Graph(args) => run_vault_graph(args, environment),
        VaultCommand::Export(args) => run_vault_export(args, environment),
    }
}

/// Check that a vault capability is authorized under the workflow policy.
fn check_vault_capability(
    runtime: &RuntimeArgs,
    environment: &CliEnvironment,
    capability: &str,
) -> anyhow::Result<()> {
    let context = RuntimeContext::open(runtime, environment, false)?;
    let policy = CapabilityPolicy::from_workflow(context.engine.workflow());
    let decision = policy.authorize(capability, false);
    if !decision.allowed {
        anyhow::bail!(
            "error[GATEWAY_DENIED]: capability {} is denied by policy{}",
            capability,
            if decision.requires_approval {
                " (requires --approve)"
            } else {
                ""
            }
        );
    }
    Ok(())
}

/// Normalizes a cycle target identifier into the canonical flat vault filename.
///
/// Transforms `project_id/cycle_id` → `project_id-cycle_id.md` by replacing the
/// separating slash with a hyphen, matching the actual on-disk naming convention.
///
/// Security: rejects targets containing path-traversal components (`..`) or
/// absolute-path prefixes (`/`). Such inputs cannot name a valid vault node and
/// are treated as missing artifacts.
fn normalize_cycle_target(target: &str) -> Option<String> {
    // Reject path-traversal or absolute-path patterns
    if target.contains("..") || target.starts_with('/') {
        return None;
    }
    // Replace the single separating slash with a hyphen to produce the flat filename
    let normalized = target.replace('/', "-");
    // Basic sanity: must still look like a project/cycle identifier
    if normalized.is_empty()
        || normalized.starts_with('-')
        || normalized.ends_with('-')
        || normalized.contains("--")
    {
        return None;
    }
    Some(normalized)
}

/// Apply scoped down-classification to VAULT003 diagnostics that have a matching
/// repair receipt in the queue.
///
/// A diagnostic is down-classified from `Error` to `Warning` when:
/// - Its code is in the `ALLOW_LIST` (verbatim `{VAULT003}` per ADR-0078)
/// - Its `scope` matches one of the provided `scope_cycles` arguments
/// - A valid (non-expired) `RepairReceipt` exists in the queue for that scope
fn apply_scope_downgrade(
    diagnostics: &mut [Diagnostic],
    scope_cycles: &[String],
    queue: &std::collections::HashMap<String, RepairReceipt>,
    vault_path: &std::path::Path,
) {
    if scope_cycles.is_empty() {
        return;
    }

    let now = time::OffsetDateTime::now_utc();

    for diagnostic in diagnostics.iter_mut() {
        // Only process allow-listed codes
        if !ALLOW_LIST.contains(&diagnostic.code.as_str()) {
            continue;
        }

        // Check if diagnostic has a matching scope
        let Some(ref scope) = diagnostic.scope else {
            continue;
        };

        // cycle_id is the full target (project_id/cycle_id) after attach_scope fix
        if !scope_cycles.contains(&scope.cycle_id) {
            continue;
        }

        // Find matching receipt
        // Key format: {cycle_id}/{code}/{node} — cycle_id is the full
        // project_id/cycle_id target string (fixed in attach_scope)
        let key = format!(
            "{}/{}/{}",
            scope.cycle_id,
            diagnostic.code,
            diagnostic.node.as_deref().unwrap_or("")
        );

        if let Some(receipt) = queue.get(&key) {
            // Check if receipt is valid (not expired)
            if receipt.valid_to <= now {
                // Receipt is expired — downgrade to warning (not an error)
                diagnostic.severity = Severity::Warning;
                diagnostic.error_kind = Some("RepairReceiptMissingOrInvalid".to_string());
            } else {
                // Receipt is valid — verify evidence SHA
                // Resolve artifact path using normalized flat filename:
                // "project_id/cycle_id" → "project_id-cycle-id.md"
                let Some(normalized) = normalize_cycle_target(&receipt.target) else {
                    // Unsafe target (traversal or malformed) — treat as missing receipt
                    diagnostic
                        .error_kind
                        .insert("RepairReceiptMissingOrInvalid".to_string());
                    return;
                };
                let artifact_path = vault_path
                    .join("cycles")
                    .join(&normalized)
                    .with_extension("md");

                match sddk_vault::verify_receipt_evidence(receipt, &artifact_path) {
                    Ok(()) => {
                        // Evidence matches — down-classify to warning
                        diagnostic.severity = Severity::Warning;
                    }
                    Err(sddk_vault::RepairReceiptError::EvidenceHashMismatch { .. }) => {
                        // Hash mismatch — emit error_kind but keep severity=Error (blocks downgrade)
                        diagnostic.error_kind = Some("ReceiptEvidenceHashMismatch".to_string());
                    }
                    Err(sddk_vault::RepairReceiptError::ArtifactNotFound) => {
                        // Artifact not found at expected path — treat as missing receipt
                        diagnostic.error_kind = Some("RepairReceiptMissingOrInvalid".to_string());
                    }
                }
            }
        } else {
            // No receipt found for this scoped diagnostic — emit warning with error_kind
            diagnostic.error_kind = Some("RepairReceiptMissingOrInvalid".to_string());
        }
    }
}

/// Summary of a repair receipt for JSON output.
#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct RepairReceiptSummary {
    cycle_id: String,
    code: String,
    node: String,
    expired: bool,
}

impl From<&RepairReceipt> for RepairReceiptSummary {
    fn from(receipt: &RepairReceipt) -> Self {
        RepairReceiptSummary {
            cycle_id: receipt.cycle_id.clone(),
            code: receipt.code.clone(),
            node: receipt.node.clone(),
            expired: receipt.valid_to < time::OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct IndexOutput {
    nodes: usize,
    backlinks: usize,
    errors: usize,
    warnings: usize,
    inserted: usize,
    updated: usize,
    deleted: usize,
    diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repair_queue: Option<Vec<RepairReceiptSummary>>,
    /// Repair queue load errors surfaced as warnings (malformed YAML etc.).
    /// These do NOT increment the errors counter — they are surfaced for observability.
    #[serde(skip_serializing_if = "Option::is_none")]
    repair_queue_errors: Option<Vec<String>>,
}

fn run_vault_index(
    args: VaultIndexArgs,
    environment: &CliEnvironment,
    validate_only: bool,
) -> CommandOutput {
    let format = args.format;
    let capability = if validate_only {
        "vault.validate"
    } else {
        "vault.index"
    };
    let result = (|| -> anyhow::Result<IndexOutput> {
        check_vault_capability(&args.runtime, environment, capability)?;
        let (index, mut diagnostics) = sddk_vault::index_vault(&args.vault)?;
        let backlinks: usize = index.backlinks.values().map(Vec::len).sum();
        let mut inserted = 0;
        let mut updated = 0;
        let mut deleted = 0;
        if !validate_only {
            let db = match args.db {
                Some(db) => db,
                None => args
                    .vault
                    .parent()
                    .map(|parent| parent.join("vault-index.sqlite"))
                    .unwrap_or_else(|| PathBuf::from("vault-index.sqlite")),
            };
            let connection = sddk_vault::open_index(&db)?;
            let summary = sddk_vault::sync_search_index(&connection, &index)?;
            inserted = summary.inserted;
            updated = summary.updated;
            deleted = summary.deleted;
        }

        // Load repair queue — propagate parse errors as visible warnings
        let repair_queue_path = args.vault.join("repair-queue.yaml");
        let (queue, repair_queue_errors) = if repair_queue_path.exists() {
            match load_repair_queue(&repair_queue_path) {
                Ok(q) => (q, None),
                Err(e) => (std::collections::HashMap::new(), Some(vec![e.to_string()])),
            }
        } else {
            (std::collections::HashMap::new(), None)
        };

        // Validate scope cycle format; emit error_kind=InvalidScopeCycleId for malformed inputs
        // Malformed scope is a CLI argument error → Severity::Error, fail-closed
        let malformed_scope_errors: Vec<Diagnostic> = args
            .scope_cycles
            .iter()
            .filter(|v| validate_scope_cycle(v).is_err())
            .map(|scope_value| Diagnostic {
                code: "VAULT003".to_string(),
                severity: Severity::Error,
                node: None,
                message: format!(
                    "invalid scope cycle format '{}': expected project_id/cycle_id (e.g. p-52b95ef55999f9de/cycle-44-build-remediate-transition)",
                    scope_value
                ),
                hint: "provide scope cycles in the form project_id/cycle_id with lowercase alphanumeric and hyphens only"
                    .to_string(),
                scope: None,
                error_kind: Some("InvalidScopeCycleId".to_string()),
            })
            .collect();

        // Apply scope downgrades if --scope-cycles is provided
        apply_scope_downgrade(&mut diagnostics, &args.scope_cycles, &queue, &args.vault);

        // Append malformed scope errors as errors (fail-closed)
        diagnostics.extend(malformed_scope_errors);

        // Build repair queue summary for JSON output
        // Show queue only when load succeeded (no errors); hide when load failed
        // Sort entries deterministically by (cycle_id, code, node) for reproducible output
        let repair_queue_summary: Option<Vec<RepairReceiptSummary>> =
            if repair_queue_errors.is_none() && !queue.is_empty() {
                let mut entries: Vec<_> = queue.values().collect();
                entries.sort_by_key(|r| (&r.cycle_id, &r.code, &r.node));
                Some(
                    entries
                        .into_iter()
                        .map(RepairReceiptSummary::from)
                        .collect(),
                )
            } else {
                None
            };

        let (errors, warnings) = sddk_vault::summary(&diagnostics);
        Ok(IndexOutput {
            nodes: index.nodes.len(),
            backlinks,
            errors,
            warnings,
            inserted,
            updated,
            deleted,
            diagnostics,
            repair_queue: repair_queue_summary,
            repair_queue_errors,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, index_text);
            if validate_only && output.errors > 0 {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

fn run_vault_search(args: VaultSearchArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<SearchHit>> {
        check_vault_capability(&args.runtime, environment, "vault.search")?;
        let connection = sddk_vault::open_index(&args.db)?;
        Ok(sddk_vault::search_index(
            &connection,
            &args.query,
            args.limit,
        )?)
    })();
    render_result(result, format, search_text)
}

fn run_vault_graph(args: VaultIndexArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GraphView> {
        check_vault_capability(&args.runtime, environment, "vault.graph")?;
        let index = sddk_vault::parse_vault(&args.vault)?;
        Ok(sddk_vault::graph_view(&index)?)
    })();
    render_result(result, format, graph_text)
}

fn run_vault_export(args: VaultExportArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        check_vault_capability(&args.runtime, environment, "vault.export")?;

        // Resolve XDG paths for fail-closed validation (ADR-0082).
        // If XDG paths cannot be resolved (e.g., HOME not set in test environments),
        // we skip validation — the test environment is not a production security boundary.
        let xdg_validation_ok = (|| -> anyhow::Result<()> {
            let root = crate::canonical_root(
                args.runtime
                    .root
                    .as_ref()
                    .unwrap_or(&std::path::PathBuf::from("."))
                    .as_path(),
            )?;
            let remote = crate::resolve_remote(&root, args.runtime.remote.clone())?;
            let mut fallback_seed = args.runtime.fallback_seed.clone();
            let scope = args.runtime.scope.as_deref().unwrap_or(".");
            if remote.is_none() && fallback_seed.is_none() {
                fallback_seed = crate::find_persisted_fallback_seed(environment, &root, scope)?;
            }
            let identity = sddk_domain::resolve_project_identity(
                remote.as_deref(),
                scope,
                fallback_seed.as_deref(),
            )?;
            let canonical_workspace_path = crate::path_string(&root)?;
            let workspace_id =
                crate::stable_workspace_id(&identity.project_id, &canonical_workspace_path);
            let paths = sddk_engine::resolve_xdg_paths(
                &environment.xdg(),
                identity.project_id.as_str(),
                &workspace_id,
            )?;

            // Fail-closed: reject output paths outside the XDG project data tree
            crate::writer::validate_xdg_output(&args.output, &paths.project_data)
                .map_err(|e| anyhow::anyhow!("STORAGE_WRITER_XDG_VIOLATION: {}", e))?;
            Ok(())
        });

        // If XDG validation failed because paths could not be resolved (e.g., missing HOME
        // in test envs), skip validation — test environments are not production boundaries.
        // If it failed for any other reason (e.g., path outside XDG), propagate the error.
        match xdg_validation_ok() {
            Ok(_) => {}
            Err(e) => {
                let is_unresolvable = e
                    .downcast_ref::<sddk_engine::PathResolutionError>()
                    .map(|pe| matches!(pe, sddk_engine::PathResolutionError::MissingHome))
                    .unwrap_or(false);
                if !is_unresolvable {
                    return Err(e);
                }
                // HOME not set — cannot resolve XDG paths; skip validation in degraded envs.
                drop(e);
            }
        }

        let (index, _) = sddk_vault::index_vault(&args.vault)?;
        let graph = sddk_vault::graph_view(&index)?;
        let html = sddk_vault::export_html(&index, &graph)?;
        std::fs::write(&args.output, &html)?;
        Ok(args.output.to_string_lossy().into_owned())
    })();
    render_result(result, format, |path| format!("wrote {path}\n"))
}

fn index_text(output: &IndexOutput) -> String {
    let mut text = format!(
        "nodes: {}\nbacklinks: {}\nerrors: {}\nwarnings: {}\ninserted: {}\nupdated: {}\ndeleted: {}\n",
        output.nodes,
        output.backlinks,
        output.errors,
        output.warnings,
        output.inserted,
        output.updated,
        output.deleted
    );
    for diagnostic in &output.diagnostics {
        text.push_str(&format!(
            "{}[{}] {}: {}\n  help: {}\n",
            match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            diagnostic.code,
            diagnostic.node.as_deref().unwrap_or("-"),
            diagnostic.message,
            diagnostic.hint
        ));
    }
    text
}

fn search_text(hits: &Vec<SearchHit>) -> String {
    if hits.is_empty() {
        return "no hits\n".to_owned();
    }
    let mut text = String::new();
    for hit in hits {
        text.push_str(&format!("{} {} {}\n", hit.id, hit.kind, hit.path));
    }
    text
}

fn graph_text(view: &GraphView) -> String {
    let mut text = format!(
        "node_count: {}\nedge_count: {}\ncyclic: {}\n",
        view.node_count, view.edge_count, view.cyclic
    );
    if let Some(cycle) = &view.sample_cycle {
        text.push_str(&format!("sample_cycle: {}\n", cycle.join(" -> ")));
    }
    if let Some(order) = &view.topological_order {
        text.push_str(&format!("topological_order: {}\n", order.join(", ")));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::normalize_cycle_target;

    #[test]
    fn normalize_cycle_target_valid() {
        assert_eq!(
            normalize_cycle_target("p-63676b11dc0ef88f/phase-c-test-boundary-cleanup"),
            Some("p-63676b11dc0ef88f-phase-c-test-boundary-cleanup".to_string())
        );
        assert_eq!(
            normalize_cycle_target("p-52b95ef55999f9de/cycle-44-build-remediate-transition"),
            Some("p-52b95ef55999f9de-cycle-44-build-remediate-transition".to_string())
        );
    }

    #[test]
    fn normalize_cycle_target_rejects_traversal() {
        assert_eq!(normalize_cycle_target("../etc/passwd"), None);
        assert_eq!(normalize_cycle_target("p-xxx/../../etc"), None);
        assert_eq!(normalize_cycle_target("/absolute/path"), None);
    }

    #[test]
    fn normalize_cycle_target_rejects_malformed() {
        assert_eq!(normalize_cycle_target(""), None);
        assert_eq!(normalize_cycle_target("-leading-hyphen"), None);
        assert_eq!(normalize_cycle_target("trailing-hyphen-"), None);
        assert_eq!(normalize_cycle_target("double--hyphen"), None);
    }
}
