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
use sddk_engine::durable_capsule_store::FilesystemCapsuleStore;
use sddk_engine::durable_session_binding::{load_binding, save_binding};
use sddk_engine::{AdoptionPaths, CapsuleStore, XdgEnvironment};
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
    /// Always `complete` on success; typed states live in `cycle`.
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
    let remote =
        resolve_remote(&root, None).map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    let canonical = path_string(&root).map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    let fallback_seed = if remote.is_none() {
        Some(sddk_domain::stable_fallback_seed(&canonical))
    } else {
        None
    };
    let identity =
        sddk_domain::resolve_project_identity(remote.as_deref(), &scope, fallback_seed.as_deref())
            .map_err(|e| ContextBootstrapError::Io(e.to_string()))?;
    let workspace_id = stable_workspace_id(&identity.project_id, &canonical);

    let xdg = xdg_of(environment);
    let paths = sddk_engine::resolve_xdg_paths(&xdg, identity.project_id.as_str(), &workspace_id)
        .map_err(|e| ContextBootstrapError::Io(e.to_string()))?;

    // ── 2. Converge adoption without interaction (CTX-005) ──
    let adoption_state = converge_adoption(
        &root,
        &scope,
        environment,
        &paths,
        &identity,
        remote.as_deref(),
        &canonical,
        args.now_ms,
    )
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
                match resolved.active_leases.len() {
                    0 => BootstrapCycleState::NoActiveCycle {
                        project_id: identity.project_id.as_str().to_string(),
                        hint: "start or resume a cycle: sddk cycle start --root <path>".to_string(),
                    },
                    1 => BootstrapCycleState::Resolved {
                        cycle_id: resolved.active_leases[0].cycle_id.clone(),
                    },
                    _ => BootstrapCycleState::Ambiguous {
                        project_id: identity.project_id.as_str().to_string(),
                        candidates: resolved
                            .active_leases
                            .iter()
                            .map(|c| CycleCandidateOut {
                                cycle_id: c.cycle_id.clone(),
                                owner: c.owner.clone(),
                                expires_at_ms: c.expires_at_ms,
                            })
                            .collect(),
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
    let capsule_store = FilesystemCapsuleStore::open(capsule_root(&paths.project_data))
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;
    let cycle_key = cycle_key(&cycle_state);
    let (context_source, basis_revision, capsule_id) = match capsule_store.last_capsule(&cycle_key)
    {
        Some(capsule) => (
            "recovered",
            capsule.capsule_id.clone(),
            Some(capsule.capsule_id),
        ),
        None => ("fresh", "empty".to_string(), None),
    };

    // ── 6. Bind the session to the resolved target and persist it ──
    let session = AgenticSessionRef::new(args.session.clone());
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
        status: "complete",
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

fn bindings_root(project_data: &Path) -> PathBuf {
    project_data.join("context").join("bindings")
}

/// Converge adoption (CTX-005). Reuses the adoption application service:
/// a repeated bootstrap over an adopted project is a semantic no-op (C3i
/// obj 2 byte-stable receipt).
#[allow(clippy::too_many_arguments)]
fn converge_adoption(
    root: &Path,
    scope: &str,
    environment: &CliEnvironment,
    paths: &AdoptionPaths,
    identity: &sddk_domain::ResolvedProjectIdentity,
    remote: Option<&str>,
    canonical: &str,
    now_ms: i64,
) -> Result<&'static str, anyhow::Error> {
    let timestamp = format_rfc3339(now_ms);
    let plan = sddk_engine::plan_adoption(sddk_engine::AdoptionPlanInput {
        remote_url: remote.map(str::to_string),
        scope: scope.to_string(),
        fallback_seed: if remote.is_none() {
            Some(sddk_domain::stable_fallback_seed(canonical))
        } else {
            None
        },
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

    /// 1st bootstrap on a never-seen project: identity resolves, adoption
    /// converges, there is no lease so the degradation is typed
    /// (`no_active_cycle`) and the binding lands on the project target.
    #[test]
    fn bootstrap_without_active_cycle_creates_project_binding() {
        let tmp = tempdir("no-cycle");
        let environment = environment(&tmp);
        let result = bootstrap(&args_at(&tmp, "s-1"), &environment).expect("bootstrap");

        assert_eq!(result.status, "complete");
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

    /// Explicit `--cycle` bypasses inference and binds the session to the
    /// Run (session ≠ run: the target is a reference, not a merge).
    #[test]
    fn explicit_cycle_binds_run_target_without_inference() {
        let tmp = tempdir("explicit");
        let environment = environment(&tmp);
        bootstrap(&args_at(&tmp, "s-6"), &environment).expect("seed");
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
        bootstrap(&args_at(&tmp, "s-8"), &environment).expect("seed");
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
}
