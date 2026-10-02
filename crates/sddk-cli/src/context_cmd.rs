//! C3j — `context bootstrap`: application service de arranque de contexto
//! (SPEC-005 CTX-003/004/005, roadmap C3j objetivo 3).
//!
//! Una operación de alto nivel que, en este orden:
//! 1. resuelve project/workspace con la MISMA identidad que `adopt`
//!    (`resolve_project_identity` + `stable_workspace_id` + `resolve_xdg_paths`);
//! 2. converge adopción sin interacción si no hay conflicto (`apply_adoption`,
//!    que ya es no-op byte-estable sobre convergido — C3i obj 2);
//! 3. infiere el ciclo con degradación TIPADA: 1 lease ⇒ ese ciclo,
//!    0 ⇒ `no_active_cycle`, N ⇒ `ambiguous_cycle` con candidates;
//! 4. reconstruye el context basis desde la capsule durable (o `fresh` si no
//!    hay ninguna) y lo ata al binding de la sesión;
//! 5. persiste el binding y devuelve la representación del bootstrap.
//!
//! NO hay una segunda autoridad: identidad, adoption y leases salen de los
//! mismos resolvers que usan `adopt` y `cycle`; el servicio solo los compone.
//! El transcript del host nunca entra (CTX-011): el servicio persiste
//! únicamente el binding semántico.

use crate::cycle::{InferenceError, RuntimeArgs, resolve_cycle_context};
use crate::{CliEnvironment, OutputFormat, canonical_root, path_string, resolve_remote};
use sddk_domain::stable_workspace_id;
use sddk_engine::agentic_session_binding::{
    AgenticBinding, AgenticSessionRef, BindingTarget, ContextBasis,
};
use sddk_engine::context_bridge::{ContextBridge, ContextDelta};
use sddk_engine::context_capsule::{CapsuleTarget, CompilerPolicy, ContextCompiler};
use sddk_engine::durable_capsule_store::FilesystemCapsuleStore;
use sddk_engine::durable_delta_store::FilesystemDeltaStore;
use sddk_engine::durable_session_binding::{load_binding, save_binding};
use sddk_engine::retry::WallClock;
use sddk_engine::{
    AdoptionPaths, CapsuleStore, CycleFactSource, CycleFacts, CycleLedgerCapsuleInputs,
    XdgEnvironment,
};
use sddk_storage::Storage;
use serde::Serialize;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Arc;

/// Arguments of `sddk context bootstrap`.
#[derive(Debug, Clone)]
pub(crate) struct ContextBootstrapArgs {
    /// Checkout or worktree root (inferred when absent).
    pub root: Option<PathBuf>,
    /// Monorepo scope, using `.` for the repository root.
    pub scope: Option<String>,
    /// Stable host session identity (opaque to SDDK).
    pub session: String,
    /// Explicit cycle id, skipping inference.
    pub cycle: Option<String>,
    /// Output format.
    pub format: OutputFormat,
    /// Current wall-clock in milliseconds (injected for determinism).
    pub now_ms: i64,
}

/// Which side of the durable delta stream this invocation acts on (CTX-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeltaMode {
    /// Publish a new material change to the session's stream.
    Publish,
    /// Drain everything the session has not seen yet.
    Drain,
}

/// Arguments of `sddk context delta`.
#[derive(Debug, Clone)]
pub(crate) struct ContextDeltaArgs {
    /// Checkout or worktree root (inferred when absent).
    pub root: Option<PathBuf>,
    /// Monorepo scope, using `.` for the repository root.
    pub scope: Option<String>,
    /// Host session identity, must match an existing binding.
    pub session: String,
    /// `--publish`: the new content (advisory semantic refs, not transcript).
    pub add: Vec<String>,
    /// `--publish`: content that is no longer true for this session.
    pub remove: Vec<String>,
    /// `--publish`: why this change matters to the session.
    pub reason: Option<String>,
    /// `--publish`: new context revision this delta produces.
    pub to_revision: Option<String>,
    /// Publish or drain.
    pub mode: DeltaMode,
    /// Output format.
    pub format: OutputFormat,
}

/// Result of `sddk context delta` (CTX-008).
#[derive(Debug, Serialize)]
pub(crate) struct ContextDeltaResult {
    pub status: &'static str,
    pub project_id: String,
    pub session: String,
    /// Basis the session currently believes (from the durable binding).
    pub basis_revision: String,
    /// `published` or `drained`.
    pub operation: &'static str,
    /// Sequence assigned (publish) or highest sequence on disk (drain).
    pub last_seq: u64,
    /// Deltas applied to the rehydrated bridge.
    pub applied: usize,
    /// Deltas the bridge rejected: `[{seq, reason}]`.
    pub rejected: Vec<RejectedDeltaOut>,
    /// Delta files that could not be read or parsed.
    pub replay_skipped: Vec<String>,
    /// Content facts in the bridge after applying (advisory content only).
    pub facts: usize,
    pub advisory: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct RejectedDeltaOut {
    pub seq: u64,
    pub reason: String,
}

/// Typed cycle resolution outcome (CTX-004).
#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum BootstrapCycleState {
    Resolved {
        cycle_id: String,
    },
    NoActiveCycle {
        project_id: String,
        hint: String,
    },
    Ambiguous {
        project_id: String,
        candidates: Vec<CycleCandidateOut>,
    },
    Explicit {
        cycle_id: String,
    },
}

#[derive(Debug, Serialize)]
pub(crate) struct CycleCandidateOut {
    pub cycle_id: String,
    pub owner: String,
    pub expires_at_ms: i64,
}

/// Full bootstrap result (CTX-003 step 7).
#[derive(Debug, Serialize)]
pub(crate) struct ContextBootstrapResult {
    /// `complete` only when the CTX-003 capsule obligation was satisfied —
    /// that is, when a durable capsule was recovered and is now the basis.
    /// Otherwise `no_capsule_source`: the bootstrap read no capsule and
    /// compiled none, so CTX-003 step 5 is unsatisfied and the command must
    /// not claim success (INC-DEBT-042). Typed states for the cycle live in
    /// `cycle`, independently of this field.
    pub status: &'static str,
    pub project_id: String,
    pub workspace_id: String,
    /// Adoption state after convergence (CTX-005).
    pub adoption: &'static str,
    pub cycle: BootstrapCycleState,
    /// `recovered` when a durable capsule was reused, `fresh` otherwise.
    pub context_source: &'static str,
    pub basis_revision: String,
    pub capsule_id: Option<String>,
    /// Durable binding path (relative to the data dir) for traceability.
    pub binding_ref: String,
    /// True when this call wrote a binding; false on byte-stable replay.
    pub binding_written: bool,
}

/// Failure surface of the service.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ContextBootstrapError {
    #[error("io error: {0}")]
    Io(String),
    #[error("durable context error: {0}")]
    Durable(String),
    #[error("adoption error: {0}")]
    Adoption(String),
    /// An explicit `--cycle` named a cycle that does not exist in the ledger.
    ///
    /// Typed on purpose: the caller named ONE cycle, so this is a broken
    /// reference, not an ambiguity. It mirrors `STORAGE_NOT_FOUND` from
    /// `sddk cycle status --cycle`, because both surfaces answer the same
    /// question inside the same documented `cli_context` envelope and must
    /// not disagree about whether a reference resolves.
    #[error("cycle not found: {cycle_id}\n  recovery: create the record or fix the reference")]
    CycleNotFound { cycle_id: String },
}

/// Run the bootstrap service.
pub(crate) fn bootstrap(
    args: &ContextBootstrapArgs,
    environment: &CliEnvironment,
) -> Result<ContextBootstrapResult, ContextBootstrapError> {
    // ── 1. Resolve project/workspace identity (same resolver as adopt) ──
    let root = match &args.root {
        Some(root) => canonical_root(root).map_err(|e| ContextBootstrapError::Io(e.to_string()))?,
        None => std::env::current_dir()
            .map_err(|e| ContextBootstrapError::Io(e.to_string()))?
            .canonicalize()
            .map_err(|e| ContextBootstrapError::Io(e.to_string()))?,
    };
    let scope = args.scope.clone().unwrap_or_else(|| ".".to_string());
    let canonical = path_string(&root).map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    let (identity, remote, fallback_seed) = resolve_via_canonical(&root, &scope)?;
    let workspace_id = stable_workspace_id(&identity.project_id, &canonical);

    let xdg = xdg_of(environment);
    let paths = sddk_engine::resolve_xdg_paths(&xdg, identity.project_id.as_str(), &workspace_id)
        .map_err(|e| ContextBootstrapError::Io(e.to_string()))?;

    // ── 2. Converge adoption without interaction (CTX-005) ──
    let adoption_state = converge_adoption(&root, environment, &paths, &identity, args.now_ms)
        .map_err(|e| ContextBootstrapError::Adoption(e.to_string()))?;

    // ── 3. Infer the cycle with typed degradation (CTX-004) ──
    let runtime_args = RuntimeArgs {
        root: Some(root.clone()),
        scope: Some(scope.clone()),
        remote: remote.clone(),
        fallback_seed: fallback_seed.clone(),
        no_infer: false,
    };
    let cycle_state = match resolve_cycle_context(&runtime_args, environment, args.cycle.as_deref())
    {
        Ok(resolved) => {
            if args.cycle.is_some() {
                BootstrapCycleState::Explicit {
                    cycle_id: resolved.cycle_id.clone().unwrap_or_default(),
                }
            } else {
                // The inference layer ships the single active lease through
                // `cycle_id` (its `active_leases` field is left empty by
                // contract), and degrades to typed errors for zero (or
                // ambiguous) leases. Match on that contract, not on the
                // always-empty vec (CTA-003 step 3 wiring fix).
                match resolved.cycle_id.clone() {
                    Some(cycle_id) => BootstrapCycleState::Resolved { cycle_id },
                    None => BootstrapCycleState::NoActiveCycle {
                        project_id: identity.project_id.as_str().to_string(),
                        hint: "start or resume a cycle: sddk cycle start --root <path>".to_string(),
                    },
                }
            }
        }
        Err(InferenceError::NoActiveCycle { project_id, hint }) => {
            BootstrapCycleState::NoActiveCycle { project_id, hint }
        }
        Err(InferenceError::AmbiguousCycle {
            project_id,
            candidates,
        }) => BootstrapCycleState::Ambiguous {
            project_id,
            candidates: candidates
                .into_iter()
                .map(|c| CycleCandidateOut {
                    cycle_id: c.cycle_id,
                    owner: c.owner,
                    expires_at_ms: c.expires_at_ms,
                })
                .collect(),
        },
        Err(other) => return Err(ContextBootstrapError::Io(other.to_string())),
    };

    // ── 4/5. Rebuild the context basis from the durable capsule ──
    // CTX-003 step 5 ("compilar capsule") is a MUST that this code path does
    // not yet satisfy: it only *reads* a durable capsule, it never compiles
    // one. Producing no capsule is therefore an unsatisfied MUST, not a
    // successful no-op, and the status must say so (INC-DEBT-042).
    let capsule_store = FilesystemCapsuleStore::open(capsule_root(&paths.project_data))
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
    let cycle_key = cycle_key(&cycle_state);
    let (context_source, basis_revision, capsule_id, status) =
        match capsule_store.last_capsule(&cycle_key) {
            Some(capsule) => (
                "recovered",
                capsule.capsule_id.clone(),
                Some(capsule.capsule_id),
                "complete",
            ),
            None => {
                // CTX-003 step 5 (the MUST): COMPILE the capsule from the
                // real facts of the resolved cycle (ADR-0147 D2). Only a
                // resolved/explicit cycle with ledger facts can compile; a
                // project with no cycle stays honestly without a capsule.
                match compile_cycle_capsule(&cycle_state, identity.project_id.as_str(), &paths) {
                    Ok(Some(capsule)) => {
                        capsule_store.persist(&capsule);
                        (
                            "compiled",
                            capsule.capsule_id.clone(),
                            Some(capsule.capsule_id),
                            "complete",
                        )
                    }
                    Ok(None) => ("fresh", "empty".to_string(), None, "no_capsule_source"),
                    Err(e) => return Err(ContextBootstrapError::Durable(e)),
                }
            }
        };

    // ── 6. Bind the session to the resolved target and persist it ──
    let session = AgenticSessionRef::new(args.session.clone());

    // An EXPLICIT reference is never looked up anywhere else in this command:
    // inference is bypassed by construction, so this is the single point
    // where the argv id can be proven to name a real cycle. Binding it
    // unverified would persist a durable reference to a fiction — the caller
    // receives `state: explicit` plus that id, and an agent rebuilding
    // `cli_context` would carry the phantom forward. Resolved-by-inference is
    // exempt: the resolver already read it from a live lease row.
    if let BootstrapCycleState::Explicit { cycle_id } = &cycle_state {
        let storage = crate::Storage::open_read_only(&paths.ledger)
            .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
        let exists = storage
            .cycle_exists(cycle_id)
            .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
        if !exists {
            return Err(ContextBootstrapError::CycleNotFound {
                cycle_id: cycle_id.clone(),
            });
        }
    }

    let target = match &cycle_state {
        BootstrapCycleState::Resolved { cycle_id } | BootstrapCycleState::Explicit { cycle_id } => {
            BindingTarget::Run {
                run_ref: sddk_engine::agentic_session_binding::RunRef::new(cycle_id.clone()),
            }
        }
        _ => BindingTarget::Project {
            project_id: identity.project_id.as_str().to_string(),
        },
    };

    let existing = load_binding(&bindings_root(&paths.project_data), &session)
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
    let binding_written = match existing {
        Some(previous) => {
            let mut next = previous.clone();
            if next.target != target {
                next = next.rebind(target, format!("context-bootstrap-{}", args.now_ms));
            }
            next.context_basis = Some(next_basis(previous.context_basis.as_ref(), &basis_revision));
            let unchanged = next == previous;
            if !unchanged {
                save_binding(&bindings_root(&paths.project_data), &next)
                    .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
            }
            !unchanged
        }
        None => {
            let mut binding = AgenticBinding::attach(session.clone(), target);
            binding.context_basis = Some(ContextBasis {
                revision: basis_revision.clone(),
                seq: 0,
            });
            save_binding(&bindings_root(&paths.project_data), &binding)
                .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
            true
        }
    };

    Ok(ContextBootstrapResult {
        status,
        project_id: identity.project_id.as_str().to_string(),
        workspace_id,
        adoption: adoption_state,
        cycle: cycle_state,
        context_source,
        basis_revision,
        capsule_id,
        binding_ref: format!("sddk/context/bindings/{}.json", args.session),
        binding_written,
    })
}

/// Derive the next basis from the previous one. The sequence advances ONLY
/// when the revision actually changes: replaying a bootstrap over an
/// unchanged basis must be byte-stable (CTX-005 idempotencia), otherwise the
/// binding churns on every call and a durable context is not observable.
fn next_basis(previous: Option<&ContextBasis>, revision: &str) -> ContextBasis {
    match previous {
        Some(previous) if previous.revision == revision => previous.clone(),
        Some(previous) => ContextBasis {
            revision: revision.to_string(),
            seq: previous.seq + 1,
        },
        None => ContextBasis {
            revision: revision.to_string(),
            seq: 0,
        },
    }
}

/// Durable capsule key for a cycle. MUST stay colon-free: the store keys
/// capsules by `<workflow_run>:<node>:<attempt>.json` and matches the run by
/// the first `:` component, so a colon inside the key can never resolve.
fn cycle_key(state: &BootstrapCycleState) -> String {
    match state {
        BootstrapCycleState::Resolved { cycle_id } | BootstrapCycleState::Explicit { cycle_id } => {
            format!("cycle-{cycle_id}")
        }
        // A project with no resolvable cycle has NO capsule of its own. It
        // deliberately reads nothing: binding the project scope to whatever
        // capsule happens to be lying around would import an unrelated
        // context (progressive disclosure stays per-target).
        _ => UNREACHABLE_CYCLE_KEY.to_string(),
    }
}

/// Capsule key for the project scope when no cycle resolves. `:` is rejected
/// by the `<workflow_run>:<node>:<attempt>` capsule key, so this name can
/// never collide with a real cycle (`cycle-<id>`).
const UNREACHABLE_CYCLE_KEY: &str = "no-active-cycle";

/// The cycle id behind a resolvable bootstrap state, if any.
fn resolvable_cycle_id(state: &BootstrapCycleState) -> Option<String> {
    match state {
        BootstrapCycleState::Resolved { cycle_id } | BootstrapCycleState::Explicit { cycle_id } => {
            Some(cycle_id.clone())
        }
        _ => None,
    }
}

/// Projects the REAL facts of a cycle from the canonical ledger (ADR-0147
/// D2): goal placeholder from the manifest, work items by state, decisions
/// by kind. Read-only over `Storage`.
struct StorageCycleFactSource {
    ledger_path: PathBuf,
}

impl std::fmt::Debug for StorageCycleFactSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageCycleFactSource")
            .field("ledger", &self.ledger_path.display().to_string())
            .finish()
    }
}

impl CycleFactSource for StorageCycleFactSource {
    fn cycle_facts(&self, cycle_id: &str) -> Option<CycleFacts> {
        let storage = Storage::open_read_only(&self.ledger_path).ok()?;
        let record = storage.get_cycle(cycle_id).ok()?;
        let work_items = storage
            .list_work_items_by_cycle(cycle_id)
            .unwrap_or_default();
        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        for wi in &work_items {
            let decisions = storage
                .list_decision_records_by_work_item(wi.id.as_ref())
                .unwrap_or_default();
            for d in decisions {
                let entry = format!("{} | {}", d.id, d.rationale);
                match d.kind {
                    sddk_domain::DecisionKind::Accept => accepted.push(entry),
                    sddk_domain::DecisionKind::Reject => rejected.push(entry),
                    // Defer/Escalate are open questions, not verdicts: they
                    // are not compiled as accepted/rejected either way.
                    _ => {}
                }
            }
        }
        Some(CycleFacts {
            cycle_id: cycle_id.to_string(),
            // The manifest's display_name is the only canonical, human-named
            // summary the cycle carries; the work items carry the rest.
            goal: Some(record.manifest.display_name),
            work_items: work_items
                .into_iter()
                .map(|wi: sddk_domain::WorkItemRecord| {
                    (
                        wi.id.clone(),
                        wi.title,
                        serde_json::to_string(&wi.status)
                            .map(|s| s.trim_matches('"').to_string())
                            .unwrap_or_else(|_| "active".to_string()),
                    )
                })
                .collect(),
            accepted_decisions: accepted,
            rejected_decisions: rejected,
        })
    }
}

/// CTX-003 step 5: compile the cycle's capsule from real ledger facts.
/// Returns `Ok(None)` when there is no cycle to compile (honest absence),
/// and a typed error when a resolvable cycle fails to compile (never a
/// silent skip).
fn compile_cycle_capsule(
    cycle_state: &BootstrapCycleState,
    _project_id: &str,
    paths: &sddk_engine::AdoptionPaths,
) -> Result<Option<sddk_engine::ContextCapsule>, String> {
    let Some(cycle_id) = resolvable_cycle_id(cycle_state) else {
        return Ok(None);
    };
    let source = StorageCycleFactSource {
        ledger_path: paths.ledger.clone(),
    };
    let Some(inputs) = CycleLedgerCapsuleInputs::from_source(&source, &cycle_id) else {
        // No ledger facts for this cycle (unknown id, or a --cycle reference
        // that does not exist yet). The bootstrap still binds — the target is
        // a reference — but compiles nothing: absence of facts is never
        // papered over with a placeholder capsule (ADR-0147 D1/D2).
        return Ok(None);
    };
    let compiler = ContextCompiler::with_policy(
        ContextCompiler::new(std::sync::Arc::new(inputs), std::sync::Arc::new(WallClock)),
        CompilerPolicy::default(),
    );
    let capsule = compiler
        .compile(CapsuleTarget {
            workflow_run: format!("cycle-{cycle_id}"),
            node_run: "bootstrap".into(),
            attempt: "cold-start".into(),
        })
        .map_err(|e| format!("capsule compile failed for cycle {cycle_id}: {e}"))?;
    Ok(Some(capsule))
}

fn xdg_of(environment: &CliEnvironment) -> XdgEnvironment {
    XdgEnvironment {
        home: environment.home.clone(),
        data_home: environment.data_home.clone(),
        sddk_data_dir: environment.sddk_data_dir.clone(),
        state_home: environment.state_home.clone(),
        sddk_state_home: environment.sddk_state_home.clone(),
        cache_home: environment.cache_home.clone(),
    }
}

/// Durable capsule directory under the project data dir — the SAME resolver
/// (`resolve_xdg_paths`) that adoption uses, never a second path authority.
fn capsule_root(project_data: &Path) -> PathBuf {
    project_data.join("context").join("capsules")
}

/// Run the durable delta operation (C3j objetivo 5, CTX-008).
///
/// `Publish` appends a material change to the session's durable stream;
/// `Drain` rehydrates a `ContextBridge` from the session's **durable binding
/// basis** and replays everything on disk onto it. Rejections are reported,
/// not hidden: a delta whose `from_revision` no longer matches the basis is
/// the CTX-008 stale case and the caller has to see it.
pub(crate) fn delta(
    args: &ContextDeltaArgs,
    environment: &CliEnvironment,
) -> Result<ContextDeltaResult, ContextBootstrapError> {
    let identity = resolve_identity(args.root.as_deref(), args.scope.as_deref())?;
    let paths = sddk_engine::resolve_xdg_paths(
        &xdg_of(environment),
        identity.project_id.as_str(),
        &identity.workspace_id,
    )
    .map_err(|e| ContextBootstrapError::Io(e.to_string()))?;

    let session = AgenticSessionRef::new(args.session.clone());
    // The binding is the authority on what the session believes. A delta
    // against a session with no binding has no basis to bind to, so this is a
    // typed failure rather than a silent no-op.
    let binding = load_binding(&bindings_root(&paths.project_data), &session)
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?
        .ok_or_else(|| {
            ContextBootstrapError::Durable(format!(
                "no durable binding for session {}: run `sddk context bootstrap --session {}` first",
                args.session, args.session
            ))
        })?;
    let basis_revision = binding
        .context_basis
        .as_ref()
        .map_or_else(|| "empty".to_string(), |basis| basis.revision.clone());

    let store = FilesystemDeltaStore::open(&deltas_root(&paths.project_data, &args.session))
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;

    // A publish's `from_revision` is the session's CURRENT basis, so a
    // publish is a forward step from what the session believes.
    let mut published_seq = store.last_seq();
    if args.mode == DeltaMode::Publish {
        let (to_revision, additions, removals) = publish_payload(args, &basis_revision)?;
        let delta = ContextDelta {
            from_revision: basis_revision.clone(),
            to_revision,
            relevance_reason: args
                .reason
                .clone()
                .unwrap_or_else(|| "material change for the active session".to_string()),
            additions,
            deletions: removals,
            seq: 0, // allocated by the store
            // Advisory-only: a delta never becomes instruction authority
            // (CDD-004). Making it a fact is a separate, gated decision.
            advisory_only: true,
        };
        let stored = store
            .append(delta)
            .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
        published_seq = stored.seq;
    }

    // Rehydration rewinds to where the stream STARTED, not to where the
    // session currently is. Bootstrapping at the current basis would make
    // every already-consumed delta look stale and rebuild an empty bridge.
    let origin = store
        .origin_basis()
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
    let (mut bridge, _bootstrap_context) =
        ContextBridge::bootstrap(&binding, origin.unwrap_or_else(|| basis_revision.clone()));

    let outcome = store
        .apply_to(&mut bridge)
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;

    // The basis only moves when the delivered stream actually advanced it.
    // That keeps `context delta` as honest as `context bootstrap`: a
    // rejected stream leaves the session believing what it believed.
    let new_basis = bridge.current_revision().to_string();
    if new_basis != basis_revision {
        let mut next = binding.clone();
        next.context_basis = Some(next_basis(binding.context_basis.as_ref(), &new_basis));
        if next != binding {
            save_binding(&bindings_root(&paths.project_data), &next)
                .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
        }
    }

    Ok(ContextDeltaResult {
        status: "complete",
        project_id: identity.project_id.as_str().to_string(),
        session: args.session.clone(),
        basis_revision: new_basis,
        operation: match args.mode {
            DeltaMode::Publish => "published",
            DeltaMode::Drain => "drained",
        },
        last_seq: published_seq.max(outcome.last_seq_on_disk),
        applied: outcome.applied.len(),
        rejected: outcome
            .rejected
            .iter()
            .map(|r| RejectedDeltaOut {
                seq: r.seq,
                reason: r.reason.clone(),
            })
            .collect(),
        replay_skipped: outcome.replay_skipped,
        facts: bridge.facts().len(),
        advisory: bridge.advisory().len(),
    })
}

/// Build the payload of a publish.
///
/// A delta with neither additions nor deletions is refused: it would occupy a
/// sequence slot and advance the basis without changing any context, which is
/// indistinguishable from corruption once persisted.
fn publish_payload(
    args: &ContextDeltaArgs,
    basis_revision: &str,
) -> Result<(String, Vec<String>, Vec<String>), ContextBootstrapError> {
    if args.add.is_empty() && args.remove.is_empty() {
        return Err(ContextBootstrapError::Durable(
            "publish needs at least one --add or --remove".to_string(),
        ));
    }
    // The revision is named by the caller, not invented here: an empty
    // `to_revision` would make every delta advance the basis to the same
    // value and stop being distinguishable.
    let to_revision = args.to_revision.clone().ok_or_else(|| {
        ContextBootstrapError::Durable(format!(
            "publish needs --to-revision (the new content revision; current basis is {basis_revision})"
        ))
    })?;
    if to_revision == basis_revision {
        return Err(ContextBootstrapError::Durable(format!(
            "to_revision {to_revision} equals the current basis: a delta must change the basis"
        )));
    }
    Ok((to_revision, args.add.clone(), args.remove.clone()))
}

/// Arguments of `sddk context expand` (C3j objetivo 4, CTX-UAT-015).
#[derive(Debug, Clone)]
pub(crate) struct ContextExpandArgs {
    /// Checkout or worktree root (inferred when absent).
    pub root: Option<PathBuf>,
    /// Monorepo scope, using `.` for the repository root.
    pub scope: Option<String>,
    /// Host session identity; must already have a durable cycle binding.
    pub session: String,
    /// The capsule reference to expand (`work-item:<id>`, a bare decision
    /// id, or `cycle:<id>`), exactly as it appears in the capsule.
    pub r#ref: String,
    /// Output format.
    pub format: OutputFormat,
}

/// Failure surface of `context expand`. Every variant says WHY the
/// reference did not expand; none of them invents content.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ContextExpandError {
    #[error("io error: {0}")]
    Io(String),
    #[error("durable context error: {0}")]
    Durable(String),
    #[error(
        "no durable binding for session {session}: run `sddk context bootstrap --session {session}` first"
    )]
    NoBinding { session: String },
    #[error("session {session} is not bound to a cycle: there is no capsule to expand")]
    NoCycleBound { session: String },
    #[error(
        "no durable capsule for the bound cycle {cycle_id}: run `sddk context bootstrap` again"
    )]
    NoCapsule { cycle_id: String },
    /// The requested reference is not part of the capsule. The available
    /// refs travel in the error (the candidates pattern): the caller can
    /// list what COULD be expanded instead of guessing.
    #[error(
        "ref not found in the capsule: {reference}\n  available refs:\n{}",
        available.iter().map(|r| format!("    {r}")).collect::<Vec<_>>().join("\n")
    )]
    RefNotFound {
        reference: String,
        available: Vec<String>,
    },
    /// The capsule carries the ref but the ledger no longer does: the
    /// capsule is stale relative to the ledger, and saying so is the honest
    /// degradation (never synthesize content the ledger lost).
    #[error(
        "capsule ref {reference} no longer resolves in the ledger (the capsule is stale): run `sddk context bootstrap` to recompile"
    )]
    StaleRef { reference: String },
}

/// Result of one successful expand. `content` is read from the LEDGER, not
/// from the capsule: the capsule names the ref, the ledger owns the truth.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ContextExpandResult {
    pub status: String,
    pub session: String,
    #[serde(rename = "ref")]
    pub r#ref: String,
    /// `work-item` | `decision` | `cycle`.
    pub kind: String,
    pub content: String,
    pub content_sha256: String,
    pub capsule_id: String,
    pub basis_revision: Option<String>,
    /// Number of records in this session's read log after this expand.
    pub reads_recorded: usize,
}

/// Durable read-log directory: one JSON file per session under the project
/// data dir (same XDG resolver as capsules/bindings — no second authority).
fn reads_root(project_data: &Path) -> PathBuf {
    project_data.join("context").join("reads")
}

/// Upper bound on records kept per session read log (SPEC-011 §4: traces
/// stay bounded). Oldest records are dropped first.
const READ_LOG_CAP: usize = 100;

/// Run the expand operation (C3j objetivo 4, CTX-UAT-015).
///
/// Progressive disclosure, minimum viable slice: the bootstrap envelope
/// carries the capsule's REFS (never its full content); this command is how
/// a session reads exactly ONE of those refs. The content comes from the
/// ledger (the authority), the read is recorded in a durable per-session
/// `ContextReadRecord` log, and a ref the capsule does not contain is a
/// typed error that lists what WOULD have been available.
pub(crate) fn expand(
    args: &ContextExpandArgs,
    environment: &CliEnvironment,
) -> Result<ContextExpandResult, ContextExpandError> {
    let identity =
        resolve_identity(args.root.as_deref(), args.scope.as_deref()).map_err(|e| match e {
            ContextBootstrapError::Io(msg) => ContextExpandError::Io(msg),
            other => ContextExpandError::Durable(other.to_string()),
        })?;
    let paths = sddk_engine::resolve_xdg_paths(
        &xdg_of(environment),
        identity.project_id.as_str(),
        &identity.workspace_id,
    )
    .map_err(|e| ContextExpandError::Io(e.to_string()))?;

    // 1. The binding is the authority on what this session may see.
    let session = AgenticSessionRef::new(args.session.clone());
    let binding = load_binding(&bindings_root(&paths.project_data), &session)
        .map_err(|e| ContextExpandError::Durable(e.to_string()))?
        .ok_or_else(|| ContextExpandError::NoBinding {
            session: args.session.clone(),
        })?;
    // Session ≠ run: only a Run-bound session carries a cycle capsule.
    let cycle_id = match &binding.target {
        BindingTarget::Run { run_ref } => run_ref.0.clone(),
        _ => {
            return Err(ContextExpandError::NoCycleBound {
                session: args.session.clone(),
            });
        }
    };

    // 2. The capsule of the bound cycle names what exists to expand.
    let store = FilesystemCapsuleStore::open(capsule_root(&paths.project_data))
        .map_err(|e| ContextExpandError::Durable(e.to_string()))?;
    let capsule = store
        .last_capsule(&format!("cycle-{cycle_id}"))
        .ok_or_else(|| ContextExpandError::NoCapsule {
            cycle_id: cycle_id.clone(),
        })?;

    // 3. Find the ref among the capsule's named references. The KEY of an
    //    entry is everything before " | " (the title/rationale is prose
    //    shipped alongside the key, not part of the identity).
    let key_of = |entry: &str| entry.split(" | ").next().unwrap_or(entry).to_string();
    let mut entries: Vec<(String, String)> = Vec::new(); // (key, kind)
    for entry in capsule
        .artifacts
        .must_read
        .iter()
        .chain(capsule.artifacts.relevant.iter())
    {
        let key = key_of(entry);
        // The cycle-level capsule (ADR-0147 D2) only produces these two
        // typed prefixes; a bare path comes from a recovery capsule and is
        // recognized as a key but expands only if a kind supports it.
        let kind = if key.starts_with("work-item:") {
            "work-item".to_string()
        } else if key.starts_with("cycle:") {
            "cycle".to_string()
        } else {
            "other".to_string()
        };
        entries.push((key, kind));
    }
    // Decisions travel as "<id> | <rationale>" with a BARE id (no prefix).
    for entry in capsule
        .decisions
        .accepted
        .iter()
        .chain(capsule.decisions.rejected.iter())
    {
        entries.push((key_of(entry), "decision".to_string()));
    }
    let requested = args.r#ref.as_str();
    let matched_kind = entries
        .iter()
        .find(|(key, _)| key == requested)
        .map(|(_, kind)| kind.clone());
    let Some(kind) = matched_kind else {
        let mut available: Vec<String> = entries.iter().map(|(k, _)| k.clone()).collect();
        available.sort();
        available.dedup();
        return Err(ContextExpandError::RefNotFound {
            reference: requested.to_string(),
            available,
        });
    };

    // 4. Resolve the content from the LEDGER. The capsule names the ref;
    //    the ledger owns the truth. A ref the ledger lost is staleness and
    //    is reported, never papered over with capsule prose.
    let storage = Storage::open_read_only(&paths.ledger)
        .map_err(|e| ContextExpandError::Durable(e.to_string()))?;
    let content = match kind.as_str() {
        "work-item" => {
            let id = requested.strip_prefix("work-item:").unwrap_or(requested);
            let wi = storage
                .get_work_item(id)
                .map_err(|e| ContextExpandError::Durable(e.to_string()))?
                .ok_or_else(|| ContextExpandError::StaleRef {
                    reference: requested.to_string(),
                })?;
            let status = serde_json::to_string(&wi.status)
                .map_err(|e| ContextExpandError::Durable(e.to_string()))?
                .trim_matches('"')
                .to_string();
            format!(
                "#{} [{}] {}\n{}",
                wi.id.as_str(),
                status,
                wi.title,
                if wi.description.is_empty() {
                    "(no description)".to_string()
                } else {
                    wi.description.clone()
                }
            )
        }
        "decision" => {
            let d = storage
                .get_decision_record(requested)
                .map_err(|e| ContextExpandError::Durable(e.to_string()))?
                .ok_or_else(|| ContextExpandError::StaleRef {
                    reference: requested.to_string(),
                })?;
            format!(
                "#{} [{:?}] decision on work item {}\n{}",
                d.id, d.kind, d.work_item_id, d.rationale
            )
        }
        "cycle" => {
            let id = requested.strip_prefix("cycle:").unwrap_or(requested);
            let record = match storage.get_cycle(id) {
                Ok(record) => record,
                Err(_) => {
                    return Err(ContextExpandError::StaleRef {
                        reference: requested.to_string(),
                    });
                }
            };
            format!(
                "#{} [{}] phase={}\n{}",
                record.manifest.cycle_id,
                serde_json::to_string(&record.manifest.status)
                    .map_err(|e| ContextExpandError::Durable(e.to_string()))?
                    .trim_matches('"'),
                serde_json::to_string(&record.manifest.phase)
                    .map_err(|e| ContextExpandError::Durable(e.to_string()))?
                    .trim_matches('"'),
                record.manifest.display_name
            )
        }
        other => {
            return Err(ContextExpandError::Durable(format!(
                "unsupported ref kind {other:?}: this slice expands work-item/decision/cycle refs"
            )));
        }
    };

    // 5. Record the read (SPEC-011 §3): bookkeeping, bounded, durable.
    let content_sha256 = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    };
    let mut recorder = sddk_domain::ContextReadRecorder::default_cap();
    recorder.record(requested);
    recorder.add_category(&kind);
    let mut record = recorder.finish(&args.session, None);
    record.content_hashes = vec![content_sha256.clone()];
    let reads_dir = reads_root(&paths.project_data);
    std::fs::create_dir_all(&reads_dir).map_err(|e| ContextExpandError::Io(e.to_string()))?;
    let reads_path = reads_dir.join(format!("{}.json", args.session));
    let mut log: Vec<sddk_domain::ContextReadRecord> = std::fs::read(&reads_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    log.push(record);
    if log.len() > READ_LOG_CAP {
        let excess = log.len() - READ_LOG_CAP;
        log.drain(0..excess);
    }
    std::fs::write(
        &reads_path,
        serde_json::to_vec_pretty(&log).map_err(|e| ContextExpandError::Io(e.to_string()))?,
    )
    .map_err(|e| ContextExpandError::Io(e.to_string()))?;

    Ok(ContextExpandResult {
        status: "expanded".to_string(),
        session: args.session.clone(),
        r#ref: requested.to_string(),
        kind,
        content,
        content_sha256,
        capsule_id: capsule.capsule_id.clone(),
        basis_revision: binding.context_basis.as_ref().map(|b| b.revision.clone()),
        reads_recorded: log.len(),
    })
}

/// Resolve the project identity through the **one** resolver, and hand back the
/// remote and seed that produced it, because the callers display and forward
/// both.
///
/// This function exists because this module had **two** resolution sites, and
/// both called `sddk_domain::resolve_project_identity` directly — so neither
/// applied the project alias. `sddk context bootstrap` therefore reported the
/// **retired** `project_id` while `sddk project resolve` on the same checkout
/// reported the surviving one, and — worse than the disagreement — it answered
/// `adoption: complete` about an adoption held under the retired id and wrote a
/// durable session binding into that id's data dir, where nothing would read it
/// afterwards. INC-DEBT-059.
///
/// The doc on the old `resolve_identity` said it resolved "with the SAME
/// resolver as `adopt`", which was literally true and exactly the problem: both
/// bypassed the authority in the same way, and agreement between two bypasses
/// is not convergence.
fn resolve_via_canonical(
    root: &Path,
    scope: &str,
) -> Result<
    (
        sddk_domain::ResolvedProjectIdentity,
        Option<String>,
        Option<String>,
    ),
    ContextBootstrapError,
> {
    let remote =
        resolve_remote(root, None).map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    let canonical = path_string(root).map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    let fallback_seed = if remote.is_none() {
        Some(sddk_domain::stable_fallback_seed(&canonical))
    } else {
        None
    };
    let identity =
        crate::resolve_identity_honoring_pin(root, scope, remote.clone(), fallback_seed.clone())
            .map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    Ok((identity, remote, fallback_seed))
}

/// Resolve project/workspace identity through the canonical resolver.
fn resolve_identity(
    root: Option<&Path>,
    scope: Option<&str>,
) -> Result<ResolvedContextIdentity, ContextBootstrapError> {
    let root = match root {
        Some(root) => canonical_root(root).map_err(|e| ContextBootstrapError::Io(e.to_string()))?,
        None => std::env::current_dir()
            .map_err(|e| ContextBootstrapError::Io(e.to_string()))?
            .canonicalize()
            .map_err(|e| ContextBootstrapError::Io(e.to_string()))?,
    };
    let scope = scope.unwrap_or(".").to_string();
    let canonical = path_string(&root).map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    let (identity, _remote, _fallback_seed) = resolve_via_canonical(&root, &scope)?;
    let workspace_id = stable_workspace_id(&identity.project_id, &canonical);
    Ok(ResolvedContextIdentity {
        project_id: identity.project_id,
        workspace_id,
    })
}

struct ResolvedContextIdentity {
    project_id: sddk_domain::ProjectId,
    workspace_id: String,
}

fn deltas_root(project_data: &Path, session: &str) -> PathBuf {
    project_data.join("context").join("deltas").join(session)
}

fn bindings_root(project_data: &Path) -> PathBuf {
    project_data.join("context").join("bindings")
}

/// Converge adoption (CTX-005). Reuses the adoption application service:
/// a repeated bootstrap over an adopted project is a semantic no-op (C3i
/// obj 2 byte-stable receipt).
///
/// `scope`, `remote` and `fallback_seed` are gone from this signature on
/// purpose. They existed only to feed the derivation that used to happen
/// inside the engine; with the identity arriving resolved there is nothing
/// left for them to do. They are removed rather than left as `_`-prefixed
/// parameters, because a dead parameter in a signature reads like meaning and
/// the next caller wires it back up.
fn converge_adoption(
    root: &Path,
    environment: &CliEnvironment,
    paths: &AdoptionPaths,
    identity: &sddk_domain::ResolvedProjectIdentity,
    now_ms: i64,
) -> Result<&'static str, anyhow::Error> {
    let timestamp = format_rfc3339(now_ms);
    let plan = sddk_engine::plan_adoption(sddk_engine::AdoptionPlanInput {
        // The identity comes in already resolved, pin and alias both applied.
        //
        // It used to be re-derived here, and forwarded through
        // `pinned_project_id` **only when `identity_source == Pinned`**. That
        // conditional was the bug, not the fix: a checkout with an alias and
        // *without* a pin took the `None` branch, the engine derived the
        // pre-alias id, and the bootstrap adopted a second time under the
        // retired one. Reusing the pin channel to carry a resolved identity
        // also loses `alias_hops` and `identity_source` one level deeper, so
        // the status could not have declared the redirect even if the id had
        // been right. INC-DEBT-059.
        identity: identity.clone(),
        canonical_workspace_path: root.to_path_buf(),
        display_name: root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "workspace".to_string()),
        xdg: xdg_of(environment),
        sddk_version: "3.6".into(),
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        timestamp,
        actor: environment
            .sddk_actor
            .clone()
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk".to_string()),
    })?;
    let _ = (paths, identity);
    let mut storage = Storage::open(&plan.paths.ledger)?;
    let status = sddk_engine::apply_adoption(&plan, &mut storage)?;
    Ok(match status.status {
        sddk_engine::AdoptionStatusKind::Complete => "complete",
        _ => "degraded",
    })
}

/// Deterministic RFC 3339 rendering of the injected clock (civil-from-days,
/// Howard Hinnant's algorithm).
fn format_rfc3339(now_ms: i64) -> String {
    let secs = now_ms.div_euclid(1000);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (h, minute, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{minute:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::ContextCapsule;
    use sddk_engine::agentic_session_binding::RunRef;
    use sddk_engine::context_capsule::{CapsuleTarget, ContextCompiler, InMemoryCapsuleInputs};
    use sddk_engine::durable_session_binding::load_all;
    use sddk_engine::retry::MockClock;

    fn environment(root: &Path) -> CliEnvironment {
        let data = root.join("xdg/data");
        let state = root.join("xdg/state");
        let cache = root.join("xdg/cache");
        let home = root.join("home");
        for dir in [&data, &state, &cache, &home] {
            std::fs::create_dir_all(dir).expect("create xdg dir");
        }
        CliEnvironment {
            home: Some(home),
            data_home: Some(data),
            sddk_data_dir: Some(root.join("sddk-data")),
            framework_dir: Some(root.join("sddk-framework")),
            state_home: Some(state),
            sddk_state_home: Some(root.join("sddk-state")),
            cache_home: Some(cache),
            sddk_actor: Some("tester".into()),
            user: Some("tester".into()),
        }
    }

    fn args(session: &str) -> ContextBootstrapArgs {
        ContextBootstrapArgs {
            root: None,
            scope: None,
            session: session.to_string(),
            cycle: None,
            format: OutputFormat::Json,
            now_ms: 1_760_000_000_000,
        }
    }

    /// Args rooted at an isolated checkout: identity, ledger and durable
    /// context all resolve inside the temp dir (never the real CWD).
    fn args_at(root: &Path, session: &str) -> ContextBootstrapArgs {
        ContextBootstrapArgs {
            root: Some(root.to_path_buf()),
            ..args(session)
        }
    }

    /// Compile a real capsule through the canonical compiler (not a hand-made
    /// struct literal) so the store test exercises production shapes.
    fn capsule(id_target: &str) -> ContextCapsule {
        let inputs = InMemoryCapsuleInputs::new()
            .with_objective("cerrar C3j")
            .with_dod(vec!["tests verdes".into()])
            .with_must_read(vec!["docs/roadmap/ROADMAP.md".into()])
            .with_relevant(vec!["crates/sddk-cli/src/context_cmd.rs".into()]);
        let clock = MockClock::new(1_760_000_000_000);
        let compiler = ContextCompiler::new(Arc::new(inputs), Arc::new(clock));
        compiler
            .compile(CapsuleTarget {
                workflow_run: id_target.to_string(),
                node_run: "node".into(),
                attempt: "1".into(),
            })
            .expect("compile capsule")
    }

    fn plant(store: &FilesystemCapsuleStore, capsule: &ContextCapsule) {
        store.persist(capsule);
    }

    /// A bootstrap that delivered no capsule MUST NOT report `complete`.
    ///
    /// SPEC-005 CTX-003 step 5 is a literal MUST ("compilar capsule"). The
    /// bootstrap only ever *reads* a durable capsule
    /// (`FilesystemCapsuleStore::last_capsule`) and never compiles one, so
    /// with no pre-existing capsule the command used to return
    /// `status: "complete"` with `capsule_id: null`, `context_source:
    /// fresh` and `basis_revision: empty`, and the CLI mapped that to exit
    /// code 0 unconditionally. That is a false success: `complete` is the
    /// signal an orchestrator reads to decide the context is ready.
    ///
    /// This pins INC-DEBT-042 option (a): degrade the typed state and the
    /// exit code so the omission is observable. It does NOT close the gap
    /// against the MUST — compiling a capsule still needs the ledger
    /// adapter and the unresolved `frontier` model decision
    /// (INC-DEBT-039) — so the assertion is that the command stops
    /// claiming it succeeded.
    #[test]
    fn bootstrap_without_capsule_does_not_report_complete() {
        let tmp = tempdir("no-capsule-honesty");
        let environment = environment(&tmp);
        let result = bootstrap(&args_at(&tmp, "s-honest"), &environment).expect("bootstrap");

        // The precondition that makes this a defect and not a design:
        // no capsule was compiled, so the MUST was not satisfied.
        assert!(
            result.capsule_id.is_none(),
            "precondition: this test is only meaningful when no capsule exists"
        );

        // The defect: reporting `complete` while delivering nothing.
        assert_ne!(
            result.status, "complete",
            "bootstrap must not report complete when it compiled no capsule (INC-DEBT-042)"
        );
        assert_eq!(
            result.status, "no_capsule_source",
            "the degradation must be typed so a consumer can branch on it"
        );
    }

    /// The same omission must be observable from the *CLI surface*, not only
    /// from the service struct: a consumer that only reads the process exit
    /// code must be able to tell that nothing was compiled.
    #[test]
    fn bootstrap_without_capsule_exits_nonzero_at_the_cli_boundary() {
        let tmp = tempdir("no-capsule-exit");
        let environment = environment(&tmp);
        let result = bootstrap(&args_at(&tmp, "s-exit"), &environment).expect("bootstrap");
        assert!(result.capsule_id.is_none(), "precondition: no capsule");

        let output = crate::run_context(
            crate::ContextCommand::Bootstrap(crate::ContextBootstrapArgsCli {
                root: Some(tmp.clone()),
                scope: None,
                session: "s-exit".into(),
                cycle: None,
                format: OutputFormat::Json,
            }),
            &environment,
        );

        assert_ne!(
            output.status, 0,
            "exit code must not be 0 when bootstrap compiled no capsule (INC-DEBT-042)"
        );
        assert!(
            output.stdout.contains("no_capsule_source"),
            "the typed state must reach the rendered output; got: {}",
            output.stdout
        );
    }

    /// A bootstrap that *did* recover a durable capsule keeps reporting
    /// `complete`: the degradation must not swallow the success path.
    #[test]
    fn bootstrap_with_recovered_capsule_still_reports_complete() {
        let tmp = tempdir("capsule-complete");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-ok"), &environment).expect("seed");
        let paths = project_paths(&tmp, &environment);
        let store = FilesystemCapsuleStore::open(capsule_root(&paths)).expect("open store");
        plant(&store, &capsule(UNREACHABLE_CYCLE_KEY));

        let recovered = bootstrap(&args_at(&tmp, "s-ok"), &environment).expect("recovered");
        assert!(
            recovered.capsule_id.is_some(),
            "precondition: a durable capsule was planted"
        );
        assert_eq!(
            recovered.status, "complete",
            "recovering a durable capsule is the success path and must stay complete"
        );
    }

    /// 1st bootstrap on a never-seen project: identity resolves, adoption
    /// converges, there is no lease so the degradation is typed
    /// (`no_active_cycle`) and the binding lands on the project target.
    ///
    /// `status` is `no_capsule_source`, not `complete`: this bootstrap found
    /// no durable capsule and compiled none, so CTX-003 step 5 is
    /// unsatisfied. The test previously asserted `complete` here, which is
    /// the false success INC-DEBT-042 opened — it accepted a `complete`
    /// whose own sibling assertion (`capsule_id.is_none()`) contradicted it.
    /// Everything else about the binding behaviour is unchanged.
    #[test]
    fn bootstrap_without_active_cycle_creates_project_binding() {
        let tmp = tempdir("no-cycle");
        let environment = environment(&tmp);
        let result = bootstrap(&args_at(&tmp, "s-1"), &environment).expect("bootstrap");

        assert_eq!(result.status, "no_capsule_source");
        assert_eq!(result.adoption, "complete");
        assert_eq!(result.context_source, "fresh");
        assert_eq!(result.basis_revision, "empty");
        assert!(result.capsule_id.is_none());
        assert!(matches!(
            result.cycle,
            BootstrapCycleState::NoActiveCycle { .. }
        ));
        assert!(result.binding_written);
        assert!(!result.project_id.is_empty());
        assert!(result.workspace_id.starts_with("w-"));
    }

    /// Replaying the same session must be byte-stable: no second write, same
    /// basis revision and same target (CTX-005 idempotencia).
    #[test]
    fn repeated_bootstrap_is_idempotent() {
        let tmp = tempdir("idempotent");
        let environment = environment(&tmp);
        let first = bootstrap(&args_at(&tmp, "s-2"), &environment).expect("first");
        assert!(first.binding_written);

        let second = bootstrap(&args_at(&tmp, "s-2"), &environment).expect("second");
        assert!(!second.binding_written, "replay must not rewrite");
        assert_eq!(first.basis_revision, second.basis_revision);
        assert_eq!(first.project_id, second.project_id);
        assert_eq!(first.workspace_id, second.workspace_id);
        assert_eq!(first.binding_ref, second.binding_ref);
    }

    /// A durable capsule must turn `fresh` into `recovered` and its id must
    /// become the basis revision (CTX-006).
    #[test]
    fn durable_capsule_is_recovered_as_basis() {
        let tmp = tempdir("capsule");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-3"), &environment).expect("seed");
        let paths = project_paths(&tmp, &environment);
        let store = FilesystemCapsuleStore::open(capsule_root(&paths)).expect("open store");
        plant(&store, &capsule(UNREACHABLE_CYCLE_KEY));

        let recovered = bootstrap(&args_at(&tmp, "s-3c"), &environment).expect("recovered");
        assert_eq!(recovered.context_source, "recovered");
        let capsule_id = recovered.capsule_id.clone().expect("capsule id");
        assert_eq!(recovered.basis_revision, capsule_id);
    }

    /// With no lease, the capsule key is `cycle:none`: a capsule belonging to a
    /// resolved cycle must NOT be adopted as the project basis.
    #[test]
    fn capsule_of_another_cycle_is_not_adopted() {
        let tmp = tempdir("isolation");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-4"), &environment).expect("seed");
        let paths = project_paths(&tmp, &environment);
        let store = FilesystemCapsuleStore::open(capsule_root(&paths)).expect("open store");
        plant(&store, &capsule("cycle-other"));

        let result = bootstrap(&args_at(&tmp, "s-4b"), &environment).expect("bootstrap");
        assert_eq!(result.context_source, "fresh");
        assert_eq!(result.basis_revision, "empty");
    }

    /// CTX-003 step 5 (the MUST), now satisfied: a bootstrap over a project
    /// with ONE active cycle lease compiles the cycle's capsule from REAL
    /// ledger facts (ADR-0147 D2) and reports `complete` with origin
    /// `compiled` — without any pre-planted capsule.
    #[test]
    fn bootstrap_with_active_cycle_compiles_capsule_from_ledger_facts() {
        use sddk_domain::{
            DECISION_RECORD_SCHEMA_VERSION, DecisionKind, DecisionRecordRecord,
            WORK_ITEM_SCHEMA_VERSION, WorkItemRecord, WorkItemStatus,
        };
        use sddk_testkit::CycleBuilder;

        let tmp = tempdir("cycle-compile");
        let environment = environment(&tmp);

        // 1. Seed identity + adoption exactly as the first bootstrap sees it.
        bootstrap(&args_at(&tmp, "s-c0"), &environment).expect("seed");
        let canonical = std::fs::canonicalize(&tmp)
            .expect("canonicalize")
            .to_string_lossy()
            .to_string();
        let project_id = sddk_domain::stable_fallback_project_id(
            &sddk_domain::stable_fallback_seed(&canonical),
            ".",
        );

        // 2. Plant the ledger: project row, cycle, 3 work items (done/active/paused)
        //    and 2 decisions (accept + reject) attached to the first item.
        let identity_for_paths = sddk_domain::resolve_project_identity(
            None,
            ".",
            Some(&sddk_domain::stable_fallback_seed(&canonical)),
        )
        .expect("identity");
        let workspace_id = stable_workspace_id(&identity_for_paths.project_id, &canonical);
        let paths = sddk_engine::resolve_xdg_paths(
            &xdg_of(&environment),
            identity_for_paths.project_id.as_str(),
            &workspace_id,
        )
        .expect("paths");
        let mut storage = crate::Storage::open(&paths.ledger).expect("open ledger");
        storage
            .register_project_workspace(
                &sddk_domain::ProjectRecord {
                    project_id: project_id.clone(),
                    display_name: "ctx-cycle".into(),
                    remote_url: None,
                    scope: ".".into(),
                    created_at: "2026-09-30T00:00:00Z".into(),
                },
                &sddk_domain::WorkspaceRecord {
                    // Same id the adoption convergence already registered for
                    // this canonical path; re-registering is then a no-op.
                    workspace_id: workspace_id.clone(),
                    project_id: project_id.clone(),
                    canonical_path: canonical.clone(),
                    created_at: "2026-09-30T00:00:00Z".into(),
                },
            )
            .expect("register project");

        let cycle_id = "c-ctx-cycle-under-test";
        let cycle_record = CycleBuilder::new(sddk_domain::CyclePath::AFull)
            .with_id(cycle_id)
            .with_project(&project_id)
            .build();
        // The CycleBuilder hardcodes workspace_id "ws-test"; the FK against
        // workspaces(project_id, workspace_id) needs the REAL one.
        let mut cycle_record = cycle_record;
        cycle_record.manifest.workspace_id = workspace_id.to_string();
        storage.insert_cycle(&cycle_record).expect("insert cycle");

        let now = 1_760_000_000;
        let work_item = |id: &str, title: &str, status: WorkItemStatus| WorkItemRecord {
            id: id.to_string(),
            cycle_id: cycle_id.to_string(),
            title: title.to_string(),
            description: String::new(),
            status,
            actor_ref_kind: None,
            actor_ref_id: None,
            actor_ref_label: None,
            created_at: now,
            schema_version: WORK_ITEM_SCHEMA_VERSION,
            spine_order: None,
            spine_horizon: None,
            spine_status: None,
            exit_gate: None,
        };
        storage
            .insert_work_item(&work_item("wi-1", "cerrar C3j", WorkItemStatus::Done))
            .expect("wi-1");
        storage
            .insert_work_item(&work_item(
                "wi-2",
                "release v2.2.35",
                WorkItemStatus::Active,
            ))
            .expect("wi-2");
        storage
            .insert_work_item(&work_item("wi-3", "adoptar C4", WorkItemStatus::Paused))
            .expect("wi-3");
        let decision = |id: &str, kind: DecisionKind, why: &str| DecisionRecordRecord {
            id: id.to_string(),
            work_item_id: "wi-1".to_string(),
            kind,
            rationale: why.to_string(),
            actor_ref_kind: None,
            actor_ref_id: None,
            actor_ref_label: None,
            schema_version: DECISION_RECORD_SCHEMA_VERSION,
        };
        storage
            .insert_decision_record(&decision(
                "d-1",
                DecisionKind::Accept,
                "D2: compilar a nivel ciclo",
            ))
            .expect("d-1");
        storage
            .insert_decision_record(&decision(
                "d-2",
                DecisionKind::Reject,
                "D1: frontier vacio sin run",
            ))
            .expect("d-2");

        // 3. Acquire the ONE active lease that makes the cycle resolvable.
        //    Wall-clock now in MILLISECONDS (the lease API is ms-based): a
        //    fixture timestamp would read as expired at inference time.
        let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        storage
            .acquire_cycle_lease(cycle_id, "tester", now_ms, now_ms + 3_600_000)
            .expect("lease");
        drop(storage);

        // 4. A DIFFERENT session bootstraps: no capsule exists yet, so the
        //    command must COMPILE one from the facts above.
        let result = bootstrap(&args_at(&tmp, "s-c1"), &environment).expect("bootstrap");
        assert_eq!(
            result.status, "complete",
            "the cycle capsule MUST have been compiled (CTX-003 step 5)"
        );
        assert_eq!(result.context_source, "compiled");
        let capsule_id = result.capsule_id.clone().expect("compiled capsule id");
        assert_eq!(result.basis_revision, capsule_id);

        // 5. The durable capsule carries the REAL facts, not placeholders.
        let store = FilesystemCapsuleStore::open(capsule_root(&paths.project_data))
            .expect("reopen capsule store");
        let durable = store
            .last_capsule(&format!("cycle-{cycle_id}"))
            .expect("capsule persisted under the cycle key");
        let objective = format!(
            "{}\n{}\n{}\n{}\n{}",
            durable.objective,
            durable.artifacts.must_read.join("\n"),
            durable.artifacts.relevant.join("\n"),
            durable.decisions.accepted.join("\n"),
            durable.definition_of_done.join("\n")
        );
        assert!(
            objective.contains("c-ctx-cycle-under-test"),
            "objective/must_read must carry the cycle ref; got: {}",
            objective
        );
        assert!(
            objective.contains("cerrar C3j"),
            "relevant work must carry the done work item title"
        );
        assert!(
            objective.contains("d-1"),
            "must_read must carry the accepted decision"
        );
        assert!(
            objective.contains("wi-3"),
            "must_read must carry the blocked work item"
        );
    }

    /// The session is bound, but the binding carries NO transcript: only
    /// semantic references (ASB-004 / CTX-011).
    #[test]
    fn persisted_binding_carries_no_transcript() {
        let tmp = tempdir("no-transcript");
        let environment = environment(&tmp);
        let result = bootstrap(&args_at(&tmp, "s-5"), &environment).expect("bootstrap");
        let paths = project_paths(&tmp, &environment);
        let session = AgenticSessionRef::new("s-5");
        let binding = load_binding(&bindings_root(&paths), &session)
            .expect("load")
            .expect("binding exists");
        assert_eq!(binding.session, session);
        assert!(binding.semantic_refs.is_empty());
        assert_eq!(
            binding.context_basis.map(|b| b.revision),
            Some(result.basis_revision)
        );
        assert!(
            load_all(&bindings_root(&paths))
                .expect("load all")
                .contains_key(&session)
        );
    }

    /// A `--cycle` that names NO cycle in the ledger must fail closed.
    ///
    /// Explicit `--cycle` bypasses inference, so nothing else in the command
    /// ever looks the reference up: the id travelled straight from argv into
    /// `BootstrapCycleState::Explicit`, into `BindingTarget::Run`, and into a
    /// DURABLE session binding (`binding_written: true`). A stale or typo'd id
    /// therefore produced a response that structurally looked resolved —
    /// `state: explicit`, `cycle_id: <the fiction>` — while carrying no
    /// context at all (`basis_revision: "empty"`, `context_source: "fresh"`,
    /// `capsule_id: null`), and a caller that only reads the exit code or the
    /// `cycle` object would carry a phantom cycle into its `cli_context`.
    ///
    /// This is the same defect class as INC-DEBT-039/042 one level up, and the
    /// sibling surface already behaves correctly: `sddk cycle status --cycle
    /// <unknown>` exits 1 with `STORAGE_NOT_FOUND`. Two surfaces that
    /// `skills/sddk-cycle-resume/SKILL.md` documents in the SAME `cli_context`
    /// envelope must not disagree on whether a reference resolves.
    ///
    /// Invariant: a reference is a *reference* — it points at something. The
    /// fix is not to stop binding explicit cycles (session ≠ run is the
    /// documented contract); it is to refuse to bind one that does not exist.
    #[test]
    fn explicit_nonexistent_cycle_fails_closed_without_binding() {
        let tmp = tempdir("explicit-missing");
        let environment = environment(&tmp);
        // A real cycle exists, so "not found" cannot be an artefact of an
        // empty ledger: the reference is wrong, not the project.
        let project_data = plant_real_cycle(&tmp, &environment, "cycle-real");
        let paths = project_paths(&tmp, &environment);

        let mut explicit = args_at(&tmp, "s-missing");
        explicit.cycle = Some("cycle-does-not-exist".into());
        let error = bootstrap(&explicit, &environment)
            .expect_err("a --cycle that names no cycle must not resolve");

        // The error must be TYPED and carry the offending id, so a consumer
        // can branch on it and report which reference failed — matching the
        // `recovery:` affordance of STORAGE_NOT_FOUND.
        let rendered = error.to_string();
        assert!(
            rendered.contains("cycle-does-not-exist"),
            "the error must name the unresolvable reference; got: {rendered}"
        );
        assert!(
            !rendered.to_lowercase().contains("ambiguous"),
            "an unknown id is not an ambiguity: the caller named one cycle \
             explicitly and it is absent, got: {rendered}"
        );

        // No phantom binding: the session must not be bound to a cycle that
        // does not exist. `s-missing` never got a binding at all.
        assert!(
            load_binding(&bindings_root(&paths), &AgenticSessionRef::new("s-missing"))
                .expect("load bindings")
                .is_none(),
            "a failed explicit --cycle must not persist a binding to a \
             non-existent cycle"
        );
        // The real cycle planted by the fixture is untouched: the failure is
        // scoped to the bad reference, it does not poison the project.
        let store = FilesystemCapsuleStore::open(capsule_root(&project_data)).expect("open store");
        assert!(
            store.last_capsule("cycle-cycle-real").is_none(),
            "the rejected reference must not have compiled a capsule"
        );
    }

    /// The refusal must be observable from the process boundary, not only from
    /// the service return type: a caller that reads only the exit code has to
    /// be able to tell the bad reference from a good one.
    #[test]
    fn explicit_nonexistent_cycle_exits_nonzero_at_the_cli_boundary() {
        let tmp = tempdir("explicit-missing-exit");
        let environment = environment(&tmp);
        plant_real_cycle(&tmp, &environment, "cycle-real");

        let output = crate::run_context(
            crate::ContextCommand::Bootstrap(crate::ContextBootstrapArgsCli {
                root: Some(tmp.clone()),
                scope: Some(".".into()),
                session: "s-missing-exit".into(),
                cycle: Some("cycle-does-not-exist".into()),
                format: OutputFormat::Json,
            }),
            &environment,
        );
        assert_ne!(
            output.status, 0,
            "an unresolvable --cycle must not exit 0 (got stdout: {})",
            output.stdout
        );
        assert!(
            output.stdout.is_empty(),
            "a failed bootstrap must not render a resolved envelope: {}",
            output.stdout
        );
    }

    /// C3j objetivo 4: expandir una referencia de la capsule devuelve el
    /// contenido real del ledger y registra la lectura.
    ///
    /// FIXTURE: ciclo real + un work item abierto (debe aterrizar en
    /// must_read como `work-item:wi-1 | ...`) + una decisión accept (debe
    /// aterrizar en decisions.accepted como `d-1 | ...`).
    fn plant_cycle_with_facts(tmp: &Path, environment: &CliEnvironment, cycle_id: &str) -> PathBuf {
        let project_data = plant_real_cycle(tmp, environment, cycle_id);
        let canonical = std::fs::canonicalize(tmp)
            .expect("canonicalize")
            .to_string_lossy()
            .to_string();
        let identity = sddk_domain::resolve_project_identity(
            None,
            ".",
            Some(&sddk_domain::stable_fallback_seed(&canonical)),
        )
        .expect("identity");
        let workspace_id = stable_workspace_id(&identity.project_id, &canonical);
        let paths = sddk_engine::resolve_xdg_paths(
            &xdg_of(environment),
            identity.project_id.as_str(),
            &workspace_id,
        )
        .expect("paths");
        let storage = crate::Storage::open(&paths.ledger).expect("open ledger");
        storage
            .insert_work_item(&sddk_domain::WorkItemRecord {
                id: "wi-1".into(),
                cycle_id: cycle_id.to_string(),
                title: "cerrar C3j objetivo 4".into(),
                description: "expand devuelve contenido real del ledger".into(),
                status: sddk_domain::WorkItemStatus::Active,
                actor_ref_kind: None,
                actor_ref_id: None,
                actor_ref_label: None,
                created_at: 1_760_000_000,
                schema_version: sddk_domain::WORK_ITEM_SCHEMA_VERSION,
                spine_order: None,
                spine_horizon: None,
                spine_status: None,
                exit_gate: None,
            })
            .expect("insert work item");
        storage
            .insert_decision_record(&sddk_domain::DecisionRecordRecord {
                id: "d-1".into(),
                work_item_id: "wi-1".into(),
                kind: sddk_domain::DecisionKind::Accept,
                rationale: "expandir por referencia, no volcar la capsule".into(),
                actor_ref_kind: None,
                actor_ref_id: None,
                actor_ref_label: None,
                schema_version: sddk_domain::DECISION_RECORD_SCHEMA_VERSION,
            })
            .expect("insert decision");
        drop(storage);
        project_data
    }

    fn expand_args(tmp: &Path, session: &str, r#ref: &str) -> ContextExpandArgs {
        ContextExpandArgs {
            root: Some(tmp.to_path_buf()),
            scope: None,
            session: session.to_string(),
            r#ref: r#ref.to_string(),
            format: OutputFormat::Json,
        }
    }

    /// CTX-UAT-015 (parte 1): expandir el ref de un work item devuelve el
    /// contenido REAL del ledger y deja un ContextReadRecord persistido.
    #[test]
    fn expand_work_item_returns_ledger_content_and_records_the_read() {
        let tmp = tempdir("expand-work-item");
        let environment = environment(&tmp);
        plant_cycle_with_facts(&tmp, &environment, "cycle-expand-1");

        // El binding nace del bootstrap explícito (sesión ≠ run).
        let mut boot = args_at(&tmp, "s-exp-1");
        boot.cycle = Some("cycle-expand-1".into());
        bootstrap(&boot, &environment).expect("bootstrap");

        let result = expand(
            &expand_args(&tmp, "s-exp-1", "work-item:wi-1"),
            &environment,
        )
        .expect("expand");
        assert_eq!(result.status, "expanded");
        assert!(
            result.content.contains("cerrar C3j objetivo 4"),
            "content must carry the ledger title; got: {}",
            result.content
        );
        assert!(
            result.content.contains("active"),
            "content must carry the ledger status; got: {}",
            result.content
        );
        assert_eq!(result.kind, "work-item");
        // La lectura queda registrada: el record existe y nombra el ref.
        let paths = project_paths(&tmp, &environment);
        let reads = std::fs::read_to_string(reads_root(&paths).join("s-exp-1.json"))
            .expect("reads file persisted");
        assert!(
            reads.contains("work-item:wi-1"),
            "ContextReadRecord must name the expanded ref; got: {reads}"
        );
        assert!(
            reads.contains(result.content_sha256.as_str()),
            "ContextReadRecord must carry the content hash; got: {reads}"
        );
    }

    /// CTX-UAT-015 (parte 2): "ContextReadRecord actualizado" — cada expand
    /// AÑADE una lectura; el log crece entre invocaciones.
    #[test]
    fn expand_appends_to_the_read_log_across_invocations() {
        let tmp = tempdir("expand-append");
        let environment = environment(&tmp);
        plant_cycle_with_facts(&tmp, &environment, "cycle-expand-2");
        let mut boot = args_at(&tmp, "s-exp-2");
        boot.cycle = Some("cycle-expand-2".into());
        bootstrap(&boot, &environment).expect("bootstrap");

        expand(
            &expand_args(&tmp, "s-exp-2", "work-item:wi-1"),
            &environment,
        )
        .expect("first expand");
        let second =
            expand(&expand_args(&tmp, "s-exp-2", "d-1"), &environment).expect("second expand");
        assert_eq!(
            second.kind, "decision",
            "a bare id matching a decision expands as decision"
        );
        assert!(
            second.content.contains("expandir por referencia"),
            "decision content must carry the ledger rationale; got: {}",
            second.content
        );
        let paths = project_paths(&tmp, &environment);
        let reads =
            std::fs::read_to_string(reads_root(&paths).join("s-exp-2.json")).expect("reads file");
        assert_eq!(
            reads.matches("\"object_ids\"").count(),
            2,
            "two expands must have recorded two reads; got: {reads}"
        );
    }

    /// Una referencia que la capsule no contiene NO se inventa: error tipado
    /// con la lista de refs disponibles (el patrón candidates de la
    /// ambigüedad, aplicado a la resolución de refs).
    #[test]
    fn expand_unknown_ref_fails_typed_listing_available_refs() {
        let tmp = tempdir("expand-unknown");
        let environment = environment(&tmp);
        plant_cycle_with_facts(&tmp, &environment, "cycle-expand-3");
        let mut boot = args_at(&tmp, "s-exp-3");
        boot.cycle = Some("cycle-expand-3".into());
        bootstrap(&boot, &environment).expect("bootstrap");

        let error = expand(
            &expand_args(&tmp, "s-exp-3", "work-item:wi-999"),
            &environment,
        )
        .expect_err("unknown ref must not resolve");
        let rendered = error.to_string();
        assert!(
            rendered.contains("work-item:wi-999"),
            "the error must name the requested ref; got: {rendered}"
        );
        assert!(
            rendered.contains("cycle:cycle-expand-3"),
            "the error must list the available refs; got: {rendered}"
        );
    }

    /// Sin binding durable no hay nada que expandir: la sesión no inventa
    /// contexto (la misma regla que el delta ya aplica).
    #[test]
    fn expand_without_binding_fails_typed() {
        let tmp = tempdir("expand-nobinding");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-seed"), &environment).expect("seed");
        let error = expand(&expand_args(&tmp, "s-unbound", "cycle:x"), &environment)
            .expect_err("unbound session must not expand");
        let rendered = error.to_string();
        assert!(
            rendered.contains("s-unbound"),
            "the error must name the session; got: {rendered}"
        );
    }

    /// Sesión bindeada a PROYECTO (sin ciclo): no hay capsule que expandir.
    #[test]
    fn expand_on_project_bound_session_fails_typed() {
        let tmp = tempdir("expand-project");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-proj"), &environment).expect("bootstrap");
        let error = expand(&expand_args(&tmp, "s-proj", "cycle:x"), &environment)
            .expect_err("project-bound session has no capsule");
        let rendered = error.to_string();
        assert!(
            rendered.contains("no capsule") || rendered.contains("no cycle"),
            "the error must say the session has no cycle-bound capsule; got: {rendered}"
        );
    }

    /// Explicit `--cycle` bypasses inference and binds the session to the
    /// Run (session ≠ run: the target is a reference, not a merge).
    ///
    /// The cycle is REAL: the intent under test is that an explicit reference
    /// binds a Run target without consulting leases, not that an arbitrary
    /// string can be bound. Asserting it against a phantom id made the test
    /// agree with a fail-open it never meant to cover.
    #[test]
    fn explicit_cycle_binds_run_target_without_inference() {
        let tmp = tempdir("explicit");
        let environment = environment(&tmp);
        plant_real_cycle(&tmp, &environment, "cycle-explicit");
        let mut explicit = args_at(&tmp, "s-6b");
        explicit.cycle = Some("cycle-explicit".into());
        let result = bootstrap(&explicit, &environment).expect("bootstrap");

        assert!(matches!(
            result.cycle,
            BootstrapCycleState::Explicit { ref cycle_id } if cycle_id == "cycle-explicit"
        ));
        let paths = project_paths(&tmp, &environment);
        let binding = load_binding(&bindings_root(&paths), &AgenticSessionRef::new("s-6b"))
            .expect("load")
            .expect("binding");
        assert_eq!(
            binding.target,
            BindingTarget::Run {
                run_ref: RunRef::new("cycle-explicit")
            }
        );
        assert!(result.binding_written, "target changed ⇒ rebind");
    }

    /// A capsule whose key is the explicit cycle becomes the basis: explicit
    /// cycles read their own capsules, not `cycle:none`.
    #[test]
    fn explicit_cycle_reads_its_own_capsule() {
        let tmp = tempdir("explicit-capsule");
        let environment = environment(&tmp);
        plant_real_cycle(&tmp, &environment, "explicit");
        let paths = project_paths(&tmp, &environment);
        let store = FilesystemCapsuleStore::open(capsule_root(&paths)).expect("open store");
        plant(&store, &capsule("cycle-explicit"));

        let mut explicit = args_at(&tmp, "s-8b");
        explicit.cycle = Some("explicit".into());
        let result = bootstrap(&explicit, &environment).expect("bootstrap");
        assert_eq!(result.context_source, "recovered");
        assert!(
            result
                .capsule_id
                .as_deref()
                .is_some_and(|id| id.starts_with("cycle-explicit:"))
        );
    }

    /// The injected clock drives the adoption timestamp: a different `now_ms`
    /// must not change the derived identity (identity is clock-independent).
    #[test]
    fn identity_is_independent_of_the_injected_clock() {
        let tmp = tempdir("clock");
        let environment = environment(&tmp);
        let first = bootstrap(&args_at(&tmp, "s-7"), &environment).expect("first");
        let mut later = args_at(&tmp, "s-7b");
        later.now_ms += 86_400_000;
        let second = bootstrap(&later, &environment).expect("second");
        assert_eq!(first.project_id, second.project_id);
        assert_eq!(first.workspace_id, second.workspace_id);
    }

    /// The RFC 3339 rendering of the injected clock is correct for known
    /// instants (civil-from-days), so receipts are deterministic in tests.
    #[test]
    fn rfc3339_rendering_is_correct() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339(1_760_000_000_000), "2025-10-09T08:53:20Z");
        assert_eq!(format_rfc3339(951_782_400_000), "2000-02-29T00:00:00Z");
    }

    /// The bootstrap MUST converge adoption as a side effect: after the call
    /// the adoption receipt exists on disk. Without this observation the
    /// convergence step could be elided and nothing would notice.
    #[test]
    fn bootstrap_converges_adoption_on_disk() {
        let tmp = tempdir("converge");
        let environment = environment(&tmp);
        let before = adoption_receipt(&tmp, &environment);
        assert!(!before.exists(), "fixture must start unadopted: {before:?}");

        let result = bootstrap(&args_at(&tmp, "s-9"), &environment).expect("bootstrap");
        assert_eq!(result.adoption, "complete");

        let after = adoption_receipt(&tmp, &environment);
        assert!(
            after.exists(),
            "adoption receipt was not written: {after:?}"
        );
        let first = std::fs::read(&after).expect("read receipt");
        let first_hash = hash(&first);

        // Replay converges again and stays byte-stable (C3i obj 2).
        bootstrap(&args_at(&tmp, "s-9"), &environment).expect("replay");
        assert_eq!(
            hash(&std::fs::read(&after).expect("read receipt")),
            first_hash
        );
    }

    /// Delta args rooted at the same isolated checkout as the bootstrap.
    fn delta_args(root: &Path, session: &str) -> ContextDeltaArgs {
        ContextDeltaArgs {
            root: Some(root.to_path_buf()),
            scope: None,
            session: session.to_string(),
            add: Vec::new(),
            remove: Vec::new(),
            reason: None,
            to_revision: None,
            mode: DeltaMode::Drain,
            format: OutputFormat::Json,
        }
    }

    /// Publish a change that the test controls completely.
    fn publish(root: &Path, session: &str, to_revision: &str, add: &[&str]) -> ContextDeltaArgs {
        ContextDeltaArgs {
            add: add.iter().map(|s| (*s).to_string()).collect(),
            to_revision: Some(to_revision.to_string()),
            mode: DeltaMode::Publish,
            ..delta_args(root, session)
        }
    }

    /// Deltas on disk for this checkout, so a test can inspect the raw
    /// sequence instead of trusting the service's own counters.
    ///
    /// Panics loudly if the directory does not exist. A test that corrupts or
    /// injects a delta must never be able to write into the current working
    /// directory because a path resolved to something unexpected.
    fn deltas_dir(tmp: &Path, environment: &CliEnvironment, session: &str) -> PathBuf {
        let canonical = std::fs::canonicalize(tmp)
            .expect("canonicalize tmp")
            .to_string_lossy()
            .to_string();
        let identity = sddk_domain::resolve_project_identity(
            None,
            ".",
            Some(&sddk_domain::stable_fallback_seed(&canonical)),
        )
        .expect("identity");
        let workspace_id = stable_workspace_id(&identity.project_id, &canonical);
        let paths = sddk_engine::resolve_xdg_paths(
            &xdg_of(environment),
            identity.project_id.as_str(),
            &workspace_id,
        )
        .expect("xdg paths");
        let dir = deltas_root(&paths.project_data, session);
        assert!(
            dir.is_dir(),
            "deltas dir does not exist: {dir:?} (refusing to write into an unexpected path)"
        );
        dir
    }

    /// CTX-008: a delta published by one process must be visible to the next
    /// one. Two separate service calls in the same test stand in for two
    /// processes, because the only shared state is the filesystem.
    #[test]
    fn delta_survives_between_invocations() {
        let tmp = tempdir("delta-survive");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-d"), &environment).expect("bootstrap");

        let published = delta(
            &publish(&tmp, "s-d", "r1", &["the ledger is locked"]),
            &environment,
        )
        .expect("publish");
        assert_eq!(published.operation, "published");
        assert_eq!(published.applied, 1);
        assert_eq!(published.basis_revision, "r1");

        // A brand new service invocation: the stream is rehydrated from disk.
        let drained = delta(&delta_args(&tmp, "s-d"), &environment).expect("drain");
        assert_eq!(drained.operation, "drained");
        assert_eq!(drained.applied, 1, "delta was not replayed from disk");
        assert_eq!(drained.advisory, 1, "advisory content was lost");
        assert_eq!(drained.basis_revision, "r1");
    }

    /// The basis must only move when the stream actually advanced it. A drain
    /// over an empty stream leaves the session believing what it believed.
    #[test]
    fn drain_over_empty_stream_keeps_basis() {
        let tmp = tempdir("delta-empty");
        let environment = environment(&tmp);
        let booted = bootstrap(&args_at(&tmp, "s-e"), &environment).expect("bootstrap");

        let drained = delta(&delta_args(&tmp, "s-e"), &environment).expect("drain");
        assert_eq!(drained.applied, 0);
        assert_eq!(drained.basis_revision, booted.basis_revision);
    }

    /// A delta against a session with no durable binding has no basis to bind
    /// to. Failing is the honest answer; inventing a basis is not.
    #[test]
    fn delta_without_binding_is_refused() {
        let tmp = tempdir("delta-nobind");
        let environment = environment(&tmp);
        let error = delta(
            &publish(&tmp, "s-unknown", "r1", &["anything"]),
            &environment,
        )
        .expect_err("must refuse without a binding");
        assert!(
            error.to_string().contains("no durable binding"),
            "unhelpful error: {error}"
        );
    }

    /// A publish with no content would burn a sequence slot and advance the
    /// basis without changing anything.
    #[test]
    fn publish_without_payload_is_refused() {
        let tmp = tempdir("delta-nopayload");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-n"), &environment).expect("bootstrap");
        let error = delta(&publish(&tmp, "s-n", "r1", &[]), &environment)
            .expect_err("must refuse an empty delta");
        assert!(
            error.to_string().contains("at least one --add or --remove"),
            "unhelpful error: {error}"
        );
    }

    /// A delta that does not change the revision is not a change.
    #[test]
    fn publish_must_change_the_revision() {
        let tmp = tempdir("delta-samerev");
        let environment = environment(&tmp);
        let booted = bootstrap(&args_at(&tmp, "s-s"), &environment).expect("bootstrap");
        let error = delta(
            &publish(&tmp, "s-s", &booted.basis_revision, &["same revision"]),
            &environment,
        )
        .expect_err("must refuse a no-op revision");
        assert!(
            error.to_string().contains("must change the basis"),
            "unhelpful error: {error}"
        );
    }

    /// The monotonic sequence is a real contract, not an implementation
    /// detail: a second delta must be seq 2, and both must survive.
    #[test]
    fn published_deltas_are_monotonic() {
        let tmp = tempdir("delta-seq");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-q"), &environment).expect("bootstrap");

        let first =
            delta(&publish(&tmp, "s-q", "r1", &["first change"]), &environment).expect("first");
        assert_eq!(first.last_seq, 1);
        let second = delta(
            &publish(&tmp, "s-q", "r2", &["second change"]),
            &environment,
        )
        .expect("second");
        assert_eq!(second.last_seq, 2);
        assert_eq!(second.advisory, 2, "the first change was lost");
        assert_eq!(second.basis_revision, "r2");
    }

    /// A delta is never instruction authority (CDD-004). Every published delta
    /// is advisory-only, so `facts` must stay at whatever the binding carried.
    #[test]
    fn deltas_stay_advisory_only() {
        let tmp = tempdir("delta-advisory");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-a"), &environment).expect("bootstrap");
        let published = delta(
            &publish(&tmp, "s-a", "r1", &["something material"]),
            &environment,
        )
        .expect("publish");
        assert_eq!(published.advisory, 1);
        assert_eq!(published.facts, 0, "a delta became a fact");
    }

    /// Corruption on disk must surface as skipped, never as delivered
    /// context. A truncated delta file is the cheapest honest corruption.
    #[test]
    fn corrupted_delta_is_skipped_not_delivered() {
        let tmp = tempdir("delta-corrupt");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-x"), &environment).expect("bootstrap");
        delta(&publish(&tmp, "s-x", "r1", &["first change"]), &environment).expect("first");
        let dir = deltas_dir(&tmp, &environment, "s-x");
        std::fs::write(dir.join("delta-2.json"), b"{ not json").expect("corrupt a delta file");

        let drained = delta(&delta_args(&tmp, "s-x"), &environment).expect("drain");
        assert_eq!(
            drained.replay_skipped,
            vec!["delta-2.json".to_string()],
            "corruption must be reported, not silently applied"
        );
        assert_eq!(drained.applied, 1);
        assert_eq!(drained.advisory, 1);
    }

    /// CTX-008 stale case: a delta whose `from_revision` no longer matches the
    /// basis is rejected and reported, not applied on top of a moving target.
    #[test]
    fn stale_delta_is_rejected_and_reported() {
        let tmp = tempdir("delta-stale");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-st"), &environment).expect("bootstrap");
        delta(
            &publish(&tmp, "s-st", "r1", &["first change"]),
            &environment,
        )
        .expect("first");

        // Hand-write a delta that claims to start from a revision the
        // session never had. Only the store can be blamed for this, and only
        // the store can catch it.
        let dir = deltas_dir(&tmp, &environment, "s-st");
        let bogus = ContextDelta {
            from_revision: "not-a-revision".to_string(),
            to_revision: "r9".to_string(),
            relevance_reason: "hand written".to_string(),
            additions: vec!["stale content".to_string()],
            deletions: Vec::new(),
            seq: 2,
            advisory_only: true,
        };
        std::fs::write(
            dir.join("delta-2.json"),
            serde_json::to_vec_pretty(&bogus).expect("serialize"),
        )
        .expect("write stale delta");

        let drained = delta(&delta_args(&tmp, "s-st"), &environment).expect("drain");
        assert_eq!(drained.rejected.len(), 1, "stale delta was not rejected");
        assert_eq!(drained.rejected[0].seq, 2);
        assert_eq!(drained.advisory, 1, "stale content was delivered");
        assert_eq!(drained.basis_revision, "r1");
    }

    /// Adoption receipt path for this checkout, via the shared resolver.
    fn adoption_receipt(tmp: &Path, environment: &CliEnvironment) -> PathBuf {
        let canonical = std::fs::canonicalize(tmp)
            .expect("canonicalize tmp")
            .to_string_lossy()
            .to_string();
        let identity = sddk_domain::resolve_project_identity(
            None,
            ".",
            Some(&sddk_domain::stable_fallback_seed(&canonical)),
        )
        .expect("identity");
        let workspace_id = stable_workspace_id(&identity.project_id, &canonical);
        sddk_engine::resolve_xdg_paths(
            &xdg_of(environment),
            identity.project_id.as_str(),
            &workspace_id,
        )
        .expect("paths")
        .receipt
    }

    /// FNV-1a 64: small, dependency-free, enough to detect byte changes.
    fn hash(bytes: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
        hash
    }

    /// Project data dir of this checkout, resolved with the SAME resolver that
    /// adoption uses.
    fn project_paths(tmp: &Path, environment: &CliEnvironment) -> PathBuf {
        let canonical = std::fs::canonicalize(tmp)
            .expect("canonicalize tmp")
            .to_string_lossy()
            .to_string();
        let identity = sddk_domain::resolve_project_identity(
            None,
            ".",
            Some(&sddk_domain::stable_fallback_seed(&canonical)),
        )
        .expect("identity");
        let workspace_id = stable_workspace_id(&identity.project_id, &canonical);
        sddk_engine::resolve_xdg_paths(
            &xdg_of(environment),
            identity.project_id.as_str(),
            &workspace_id,
        )
        .expect("paths")
        .project_data
    }

    fn tempdir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sddk-context-cmd-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }

    /// Seed adoption and plant a REAL cycle row for `cycle_id`.
    ///
    /// The explicit-`--cycle` fixtures used to assert behaviour against ids
    /// that were never inserted in the ledger, which is exactly why the
    /// fail-open described in `explicit_nonexistent_cycle_fails_closed`
    /// survived: the only explicit fixtures were phantom cycles, so nothing
    /// could tell "binds a reference" from "binds a fiction". Returns the
    /// project data paths so callers can inspect durable state.
    fn plant_real_cycle(tmp: &Path, environment: &CliEnvironment, cycle_id: &str) -> PathBuf {
        use sddk_testkit::CycleBuilder;

        bootstrap(&args_at(tmp, "s-plant-seed"), environment).expect("seed adoption");
        let canonical = std::fs::canonicalize(tmp)
            .expect("canonicalize")
            .to_string_lossy()
            .to_string();
        let identity = sddk_domain::resolve_project_identity(
            None,
            ".",
            Some(&sddk_domain::stable_fallback_seed(&canonical)),
        )
        .expect("identity");
        let workspace_id = stable_workspace_id(&identity.project_id, &canonical);
        let paths = sddk_engine::resolve_xdg_paths(
            &xdg_of(environment),
            identity.project_id.as_str(),
            &workspace_id,
        )
        .expect("paths");
        let mut storage = crate::Storage::open(&paths.ledger).expect("open ledger");
        // The adoption convergence above already registered this project +
        // workspace pair, so re-registering the SAME ids is a no-op and only
        // exists to satisfy the foreign keys of the cycle row.
        storage
            .register_project_workspace(
                &sddk_domain::ProjectRecord {
                    project_id: identity.project_id.as_str().to_string(),
                    display_name: "ctx-explicit".into(),
                    remote_url: None,
                    scope: ".".into(),
                    created_at: "2026-09-30T00:00:00Z".into(),
                },
                &sddk_domain::WorkspaceRecord {
                    workspace_id: workspace_id.to_string(),
                    project_id: identity.project_id.as_str().to_string(),
                    canonical_path: canonical,
                    created_at: "2026-09-30T00:00:00Z".into(),
                },
            )
            .expect("register project");
        let mut record = CycleBuilder::new(sddk_domain::CyclePath::AFull)
            .with_id(cycle_id)
            .with_project(identity.project_id.as_str())
            .build();
        // CycleBuilder hardcodes workspace_id "ws-test"; the FK against
        // workspaces(project_id, workspace_id) needs the resolved one.
        record.manifest.workspace_id = workspace_id.to_string();
        storage.insert_cycle(&record).expect("insert cycle");
        drop(storage);
        paths.project_data
    }
}
