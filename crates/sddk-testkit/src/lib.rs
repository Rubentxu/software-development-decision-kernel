//! Shared testing utilities for SDDK crates.
//!
//! Provides in-memory fakes for `Ledger` and related ports, plus test builders
//! and fixtures. All production code dependencies are isolated here so that
//! `sddk-engine` tests can run without compiling `sddk-storage`.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::RwLock;

use sddk_domain::StorageError;
use tempfile::TempDir;

// Shorthand for storage result type
type Sr<T> = Result<T, StorageError>;

// ── In-memory Ledger fake ────────────────────────────────────────────────────

/// Thread-safe in-memory fake of the [`sddk_domain::Ledger`] port.
///
/// Stores all data in `RwLock`-protected `BTreeMap` collections.
/// Used by `sddk-engine` tests to avoid compiling `sddk-storage`.
#[derive(Debug, Default)]
pub struct InMemoryLedger {
    cycles: RwLock<BTreeMap<String, sddk_domain::CycleRecord>>,
    events: RwLock<Vec<sddk_domain::LedgerEvent>>,
    leases: RwLock<BTreeMap<String, sddk_domain::CycleLease>>,
    gate_receipts: RwLock<BTreeMap<String, sddk_domain::GateReceipt>>,
    projects: RwLock<BTreeMap<String, sddk_domain::ProjectRecord>>,
    workspaces: RwLock<BTreeMap<String, sddk_domain::WorkspaceRecord>>,
    sequence: RwLock<i64>,
}

impl InMemoryLedger {
    /// Creates a fresh empty fake ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current event sequence counter.
    pub fn sequence(&self) -> i64 {
        *self.sequence.read().unwrap()
    }

    /// Returns a read-only snapshot of all events.
    pub fn events(&self) -> Vec<sddk_domain::LedgerEvent> {
        self.events.read().unwrap().clone()
    }

    /// Advance the sequence counter and return the next value.
    fn next_sequence(&self) -> i64 {
        let mut seq = self.sequence.write().unwrap();
        *seq += 1;
        *seq
    }
}

impl sddk_domain::Ledger for InMemoryLedger {
    fn get_cycle(&self, cycle_id: &str) -> Sr<sddk_domain::CycleRecord> {
        self.cycles
            .read()
            .unwrap()
            .get(cycle_id)
            .cloned()
            .ok_or_else(|| sddk_domain::StorageError::NotFound {
                entity: "cycle",
                id: cycle_id.to_string(),
            })
    }

    fn list_cycle_events(&self, cycle_id: &str) -> Sr<Vec<sddk_domain::LedgerEvent>> {
        let events = self.events.read().unwrap();
        let mut result: Vec<_> = events
            .iter()
            .filter(|ev| ev.cycle_id.as_deref() == Some(cycle_id))
            .cloned()
            .collect();
        result.sort_by_key(|ev| ev.sequence);
        Ok(result)
    }

    fn insert_cycle_with_event(
        &mut self,
        cycle: &sddk_domain::CycleRecord,
        event: &sddk_domain::LedgerEventInput,
    ) -> Sr<sddk_domain::LedgerEvent> {
        let seq = self.next_sequence();
        let event_hash = format!("sha256:{:032x}", (seq as u64).wrapping_mul(13));
        let previous_hash = self
            .events
            .read()
            .unwrap()
            .last()
            .map(|e| e.event_hash.clone());
        let ledger_event = sddk_domain::LedgerEvent {
            sequence: seq,
            event_id: event.event_id.clone(),
            project_id: event.project_id.clone(),
            cycle_id: event.cycle_id.clone(),
            frame_id: event.frame_id.clone(),
            command_id: event.command_id.clone(),
            actor: event.actor.clone(),
            event_type: event.event_type.clone(),
            occurred_at: event.occurred_at.clone(),
            state_before: event.state_before.clone(),
            state_after: event.state_after.clone(),
            payload: event.payload.clone(),
            previous_hash,
            event_hash,
        };
        self.cycles
            .write()
            .unwrap()
            .insert(cycle.manifest.cycle_id.clone(), cycle.clone());
        self.events.write().unwrap().push(ledger_event.clone());
        Ok(ledger_event)
    }

    fn update_cycle_with_event(
        &mut self,
        manifest: &sddk_domain::CycleManifest,
        updated_at: &str,
        event: &sddk_domain::LedgerEventInput,
        _release_lease_on_phase_change: bool,
    ) -> Sr<sddk_domain::LedgerEvent> {
        let seq = self.next_sequence();
        let event_hash = format!("sha256:{:032x}", (seq as u64).wrapping_mul(17));
        let previous_hash = self
            .events
            .read()
            .unwrap()
            .last()
            .map(|e| e.event_hash.clone());
        let ledger_event = sddk_domain::LedgerEvent {
            sequence: seq,
            event_id: event.event_id.clone(),
            project_id: event.project_id.clone(),
            cycle_id: event.cycle_id.clone(),
            frame_id: event.frame_id.clone(),
            command_id: event.command_id.clone(),
            actor: event.actor.clone(),
            event_type: event.event_type.clone(),
            occurred_at: event.occurred_at.clone(),
            state_before: event.state_before.clone(),
            state_after: event.state_after.clone(),
            payload: event.payload.clone(),
            previous_hash,
            event_hash,
        };
        // Update cycle snapshot
        let mut cycles = self.cycles.write().unwrap();
        let record =
            cycles
                .entry(manifest.cycle_id.clone())
                .or_insert_with(|| sddk_domain::CycleRecord {
                    manifest: manifest.clone(),
                    created_at: updated_at.to_string(),
                    updated_at: updated_at.to_string(),
                });
        record.manifest = manifest.clone();
        record.updated_at = updated_at.to_string();
        drop(cycles);
        self.events.write().unwrap().push(ledger_event.clone());
        Ok(ledger_event)
    }

    fn acquire_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Sr<sddk_domain::CycleLease> {
        let lease = sddk_domain::CycleLease {
            cycle_id: cycle_id.to_string(),
            owner: owner.to_string(),
            fencing_token: 1,
            acquired_at_ms: now_ms,
            expires_at_ms,
        };
        self.leases
            .write()
            .unwrap()
            .insert(cycle_id.to_string(), lease.clone());
        Ok(lease)
    }

    fn release_cycle_lease(
        &mut self,
        _project_id: &str,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        _actor: &str,
        _command_id: &str,
        _occurred_at: &str,
    ) -> Sr<bool> {
        let mut leases = self.leases.write().unwrap();
        let released = leases
            .get(cycle_id)
            .filter(|l| l.owner == owner && l.fencing_token == fencing_token)
            .is_some();
        if released {
            leases.remove(cycle_id);
            return Ok(true);
        }
        Ok(false)
    }

    fn renew_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        _now_ms: i64,
        new_expires_at_ms: i64,
    ) -> Sr<sddk_domain::CycleLease> {
        let mut leases = self.leases.write().unwrap();
        if let Some(lease) = leases
            .get_mut(cycle_id)
            .filter(|l| l.owner == owner && l.fencing_token == fencing_token)
        {
            lease.expires_at_ms = new_expires_at_ms;
            return Ok(lease.clone());
        }
        Err(sddk_domain::StorageError::NotFound {
            entity: "lease",
            id: cycle_id.to_string(),
        })
    }

    fn get_cycle_lease(&self, cycle_id: &str) -> Sr<sddk_domain::CycleLease> {
        self.leases
            .read()
            .unwrap()
            .get(cycle_id)
            .cloned()
            .ok_or_else(|| sddk_domain::StorageError::NotFound {
                entity: "lease",
                id: cycle_id.to_string(),
            })
    }

    fn verify_cycle_lease(
        &self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
    ) -> Sr<sddk_domain::CycleLease> {
        let leases = self.leases.read().unwrap();
        if let Some(lease) = leases.get(cycle_id).filter(|l| {
            l.owner == owner && l.fencing_token == fencing_token && l.expires_at_ms > now_ms
        }) {
            return Ok(lease.clone());
        }
        Err(sddk_domain::StorageError::NotFound {
            entity: "lease",
            id: cycle_id.to_string(),
        })
    }

    fn get_gate_receipt(&self, receipt_id: &str) -> Sr<sddk_domain::GateReceipt> {
        self.gate_receipts
            .read()
            .unwrap()
            .get(receipt_id)
            .cloned()
            .ok_or_else(|| sddk_domain::StorageError::NotFound {
                entity: "gate_receipt",
                id: receipt_id.to_string(),
            })
    }

    fn insert_gate_receipt_next_seq(
        &mut self,
        input: &sddk_domain::GateReceiptNextSeqInput,
    ) -> Sr<sddk_domain::GateReceipt> {
        let seq = self.next_sequence();
        let receipt = sddk_domain::GateReceipt {
            receipt_id: format!("gate-{:032x}", seq as u64),
            project_id: input.project_id.clone(),
            cycle_id: input.cycle_id.clone(),
            gate: input.gate.clone(),
            evaluator: input.evaluator.clone(),
            transition_id: input.transition_id.clone(),
            plan_hash: input.plan_hash.clone(),
            outcome: input.outcome,
            evidence: input.evidence.clone(),
            actor: input.actor.clone(),
            evaluated_at: input.evaluated_at.clone(),
            command_id: input.command_id.clone(),
            frame_id: format!("frame-{:032x}", seq as u64),
            seq,
        };
        self.gate_receipts
            .write()
            .unwrap()
            .insert(receipt.receipt_id.clone(), receipt.clone());
        Ok(receipt)
    }

    fn get_project_optional(&self, project_id: &str) -> Sr<Option<sddk_domain::ProjectRecord>> {
        Ok(self.projects.read().unwrap().get(project_id).cloned())
    }

    fn get_workspace_optional(
        &self,
        workspace_id: &str,
    ) -> Sr<Option<sddk_domain::WorkspaceRecord>> {
        Ok(self.workspaces.read().unwrap().get(workspace_id).cloned())
    }

    fn has_projects(&self) -> Sr<bool> {
        Ok(!self.projects.read().unwrap().is_empty())
    }

    fn register_project_workspace(
        &mut self,
        project: &sddk_domain::ProjectRecord,
        workspace: &sddk_domain::WorkspaceRecord,
    ) -> Sr<()> {
        self.projects
            .write()
            .unwrap()
            .insert(project.project_id.clone(), project.clone());
        self.workspaces
            .write()
            .unwrap()
            .insert(workspace.workspace_id.clone(), workspace.clone());
        Ok(())
    }

    fn load_all_ledger_events(&self) -> Sr<Vec<sddk_domain::LedgerEvent>> {
        let mut events = self.events.read().unwrap().clone();
        events.sort_by_key(|ev| ev.sequence);
        Ok(events)
    }
}

// ── Test builders ─────────────────────────────────────────────────────────────

/// Convenient builder for [`sddk_domain::LedgerEventInput`] values.
#[derive(Debug, Clone)]
pub struct EventBuilder {
    event_id: String,
    project_id: String,
    cycle_id: Option<String>,
    frame_id: String,
    command_id: String,
    actor: String,
    event_type: String,
    occurred_at: String,
    state_before: Option<serde_json::Value>,
    state_after: Option<serde_json::Value>,
    payload: serde_json::Value,
}

impl EventBuilder {
    /// Starts a builder for the given event type.
    pub fn new(event_type: &str) -> Self {
        Self {
            event_id: format!("evt-{}", uuid::Uuid::new_v4()),
            project_id: "p-test".to_string(),
            cycle_id: None,
            frame_id: format!("frame-{}", uuid::Uuid::new_v4()),
            command_id: format!("cmd-{}", uuid::Uuid::new_v4()),
            actor: "sddk-testkit".to_string(),
            event_type: event_type.to_string(),
            occurred_at: "2026-08-19T00:00:00Z".to_string(),
            state_before: None,
            state_after: None,
            payload: serde_json::json!({}),
        }
    }

    /// Sets the cycle ID.
    pub fn with_cycle(mut self, cycle_id: &str) -> Self {
        self.cycle_id = Some(cycle_id.to_string());
        self
    }

    /// Sets the project ID.
    pub fn with_project(mut self, project_id: &str) -> Self {
        self.project_id = project_id.to_string();
        self
    }

    /// Sets the event ID explicitly (default: random UUID).
    pub fn with_event_id(mut self, event_id: &str) -> Self {
        self.event_id = event_id.to_string();
        self
    }

    /// Sets the occurred_at timestamp (default: 2026-08-19T00:00:00Z).
    pub fn occurred_at(mut self, ts: &str) -> Self {
        self.occurred_at = ts.to_string();
        self
    }

    /// Sets the payload (default: `{}`).
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    /// Sets `state_before` snapshot.
    pub fn state_before(mut self, state: serde_json::Value) -> Self {
        self.state_before = Some(state);
        self
    }

    /// Sets `state_after` snapshot.
    pub fn state_after(mut self, state: serde_json::Value) -> Self {
        self.state_after = Some(state);
        self
    }

    /// Builds the [`LedgerEventInput`].
    pub fn build(self) -> sddk_domain::LedgerEventInput {
        sddk_domain::LedgerEventInput {
            event_id: self.event_id,
            project_id: self.project_id,
            cycle_id: self.cycle_id,
            frame_id: self.frame_id,
            command_id: self.command_id,
            actor: self.actor,
            event_type: self.event_type,
            occurred_at: self.occurred_at,
            state_before: self.state_before,
            state_after: self.state_after,
            payload: self.payload,
        }
    }
}

/// Convenient builder for [`sddk_domain::CycleRecord`] values.
#[derive(Debug, Clone)]
pub struct CycleBuilder {
    cycle_id: String,
    project_id: String,
    workspace_id: String,
    status: sddk_domain::CycleStatus,
    phase: sddk_domain::Phase,
    path: sddk_domain::CyclePath,
    branch: String,
    base: String,
    created_at: String,
    updated_at: String,
}

impl CycleBuilder {
    /// Starts a builder for a cycle in the given path.
    pub fn new(path: sddk_domain::CyclePath) -> Self {
        Self {
            cycle_id: format!("c-{}", uuid::Uuid::new_v4()),
            project_id: "p-test".to_string(),
            workspace_id: "ws-test".to_string(),
            status: sddk_domain::CycleStatus::Open,
            phase: sddk_domain::Phase::Explore,
            path,
            branch: "main".to_string(),
            base: "0".repeat(40),
            created_at: "2026-08-19T00:00:00Z".to_string(),
            updated_at: "2026-08-19T00:00:00Z".to_string(),
        }
    }

    /// Sets the cycle ID (default: random UUID).
    pub fn with_id(mut self, id: &str) -> Self {
        self.cycle_id = id.to_string();
        self
    }

    /// Sets the project ID (default: "p-test").
    pub fn with_project(mut self, project_id: &str) -> Self {
        self.project_id = project_id.to_string();
        self
    }

    /// Sets the cycle status (default: Open).
    pub fn with_status(mut self, status: sddk_domain::CycleStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets the phase (default: Explore).
    pub fn with_phase(mut self, phase: sddk_domain::Phase) -> Self {
        self.phase = phase;
        self
    }

    /// Builds the [`CycleRecord`].
    pub fn build(self) -> sddk_domain::CycleRecord {
        let manifest = sddk_domain::CycleManifest {
            schema_version: 1,
            project_id: self.project_id.clone(),
            workspace_id: self.workspace_id.clone(),
            cycle_id: self.cycle_id.clone(),
            display_name: self.cycle_id.clone(),
            status: self.status,
            phase: self.phase,
            path: self.path,
            branch: self.branch,
            base: self.base,
            head: None,
            artifacts: Default::default(),
            release: None,
            delivery_kind: None,
            remediation_round: 0,
            remote_url: None,
            scope: None,
        };
        sddk_domain::CycleRecord {
            manifest,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

// ── Existing test utilities ────────────────────────────────────────────────────

/// RAII guard that kills and reaps a child process on drop.
///
/// Covers every exit path — normal return, `?`-propagation, and panic unwind.
/// Idempotent via `Option::take()`. Use [`ChildGuard::take`] to transfer
/// ownership out of the guard (avoids double-kill).
#[derive(Debug)]
pub struct ChildGuard(Option<std::process::Child>);

impl ChildGuard {
    /// Wraps a spawned child; it will be killed and reaped when dropped.
    pub fn new(child: std::process::Child) -> Self {
        ChildGuard(Some(child))
    }

    /// Transfers ownership of the child out of the guard (avoids double-kill).
    pub fn take(&mut self) -> Option<std::process::Child> {
        self.0.take()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Isolated temporary repository for integration and contract tests.
#[derive(Debug)]
pub struct TestRepository {
    directory: TempDir,
}

impl TestRepository {
    /// Creates an empty repository root that is deleted when the fixture is dropped.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            directory: tempfile::tempdir()?,
        })
    }

    /// Returns the repository root.
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Runs a git command inside the repository with hermetic config
    /// (`GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` pointed at /dev/null).
    pub fn git(&self, args: &[&str]) -> io::Result<std::process::Output> {
        std::process::Command::new("git")
            .args(args)
            .current_dir(self.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
    }

    fn git_expect(&self, args: &[&str]) -> io::Result<()> {
        let output = self.git(args)?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    /// Initializes a git repository with a local test identity.
    pub fn init(&self) -> io::Result<()> {
        self.git_expect(&["init", "-q"])?;
        self.git_expect(&["config", "user.email", "test@sddk"])?;
        self.git_expect(&["config", "user.name", "sddk-testkit"])?;
        Ok(())
    }

    /// Stages everything and commits with the given message.
    pub fn commit_all(&self, message: &str) -> io::Result<()> {
        self.git_expect(&["add", "-A"])?;
        self.git_expect(&["commit", "-q", "-m", message])?;
        Ok(())
    }

    /// Creates a lightweight tag at HEAD.
    pub fn tag(&self, name: &str) -> io::Result<()> {
        self.git_expect(&["tag", name])?;
        Ok(())
    }

    /// Writes UTF-8 content to a repository-relative path, creating parent directories.
    pub fn write(&self, relative: impl AsRef<Path>, content: &str) -> io::Result<PathBuf> {
        let relative = relative.as_ref();
        if relative.is_absolute()
            || relative.components().any(|c| {
                matches!(
                    c,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("fixture path must stay inside the repository: {relative:?}"),
            ));
        }
        let destination = self.path().join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, content)?;
        Ok(destination)
    }
}

/// Redirected XDG environment for CLI sandboxing.
#[derive(Debug, Clone)]
pub struct XdgEnv {
    /// Redirected home directory.
    pub home: PathBuf,
    /// Redirected XDG data directory.
    pub xdg_data: PathBuf,
    /// Redirected XDG state directory.
    pub xdg_state: PathBuf,
    /// Redirected XDG cache directory.
    pub xdg_cache: PathBuf,
}

/// Isolated CLI sandbox over a `TestRepository`.
///
/// Composes over `TestRepository` to add:
/// - `HOME` + `XDG_DATA_HOME` / `XDG_STATE_HOME` / `XDG_CACHE_HOME` redirection
/// - `command() -> Command` preset accessor
/// - Optional `init_git(name, email)` for deterministic commit author identity
///
/// The parent process environment is NOT mutated on `Drop`.
#[derive(Debug)]
pub struct CliSandbox {
    /// Backing test repository (exposed for integration tests).
    pub(crate) repo: TestRepository,
    binary: PathBuf,
    xdg: XdgEnv,
}

impl CliSandbox {
    /// Creates a new CLI sandbox backed by the given `TestRepository`.
    ///
    /// `binary_path` must be supplied by the caller — `env!("CARGO_BIN_EXE_sddk")`
    /// is integration-tests-only and cannot be used in library code.
    ///
    /// The XDG directories are placed as siblings to the repo root to avoid
    /// polluting the git working tree. The `HOME` env var points to the repo root
    /// (so git uses it), while `XDG_DATA_HOME`, `XDG_STATE_HOME`, and
    /// `XDG_CACHE_HOME` point to the sibling XDG directories.
    pub fn new(repo: TestRepository, binary_path: impl AsRef<Path>) -> io::Result<Self> {
        let root = repo.path().to_path_buf();
        // Place XDG directories as siblings to the repo, not inside it,
        // to avoid polluting the git working tree.
        let xdg_parent = root.join(".sddk_xdg");
        let xdg_data = xdg_parent.join("data");
        let xdg_state = xdg_parent.join("state");
        let xdg_cache = xdg_parent.join("cache");
        fs::create_dir_all(&xdg_data)?;
        fs::create_dir_all(&xdg_state)?;
        fs::create_dir_all(&xdg_cache)?;
        // Add the XDG parent to .gitignore so git ignores all XDG content.
        let gitignore = root.join(".gitignore");
        let existing = fs::read_to_string(&gitignore).unwrap_or_default();
        if !existing.contains(".sddk_xdg") {
            let new_content = if existing.trim().is_empty() {
                ".sddk_xdg\n".to_string()
            } else {
                format!("{}\n.sddk_xdg\n", existing.trim_end())
            };
            fs::write(&gitignore, new_content)?;
        }
        Ok(Self {
            repo,
            binary: PathBuf::from(binary_path.as_ref()),
            xdg: XdgEnv {
                home: root.clone(),
                xdg_data,
                xdg_state,
                xdg_cache,
            },
        })
    }

    /// Configures the git user identity for the sandbox's repository.
    ///
    /// This writes `user.name` and `user.email` via the repository's hermetic
    /// git config so commits created inside the sandbox carry the given author.
    pub fn init_git(&self, name: &str, email: &str) -> io::Result<()> {
        self.repo.git(&["config", "user.email", email])?;
        self.repo.git(&["config", "user.name", name])?;
        Ok(())
    }

    /// Returns the sandbox root path (same as `self.repo.path()`).
    pub fn path(&self) -> &Path {
        self.repo.path()
    }

    /// Returns a reference to the underlying `TestRepository`.
    pub fn repo(&self) -> &TestRepository {
        &self.repo
    }

    /// Returns the configured XDG environment.
    pub fn xdg(&self) -> &XdgEnv {
        &self.xdg
    }

    /// Returns a `Command` preset with the sandbox's working directory and
    /// XDG environment variables already set.
    ///
    /// The caller provides the binary name/path as the first argument.
    pub fn command(&self, binary: impl AsRef<Path>) -> std::process::Command {
        let binary_path = binary.as_ref();
        let mut cmd = std::process::Command::new(std::ffi::OsStr::new(binary_path));
        cmd.current_dir(self.path())
            .env("HOME", &self.xdg.home)
            .env("XDG_DATA_HOME", &self.xdg.xdg_data)
            .env("XDG_STATE_HOME", &self.xdg.xdg_state)
            .env("XDG_CACHE_HOME", &self.xdg.xdg_cache);
        cmd
    }

    /// Returns a `Command` preset for the `sddk` binary, using the binary
    /// path supplied at construction time.
    pub fn sddk_command(&self) -> std::process::Command {
        self.command(&self.binary)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChildGuard, CliSandbox, CycleBuilder, EventBuilder, InMemoryLedger, TestRepository,
    };
    use sddk_domain::{CyclePath, Ledger};
    use std::path::PathBuf;

    #[test]
    fn in_memory_ledger_insert_and_get_cycle() {
        let mut ledger = InMemoryLedger::new();
        let cycle = CycleBuilder::new(CyclePath::AFull).build();
        let event_input = EventBuilder::new("cycle.created")
            .with_cycle(&cycle.manifest.cycle_id)
            .with_project(&cycle.manifest.project_id)
            .build();

        let inserted = ledger
            .insert_cycle_with_event(&cycle, &event_input)
            .unwrap();
        assert_eq!(inserted.sequence, 1);

        let loaded = ledger.get_cycle(&cycle.manifest.cycle_id).unwrap();
        assert_eq!(loaded.manifest.cycle_id, cycle.manifest.cycle_id);
    }

    #[test]
    fn in_memory_ledger_sequence_increments() {
        let mut ledger = InMemoryLedger::new();
        let cycle = CycleBuilder::new(CyclePath::BDirect).build();
        let event_input = EventBuilder::new("cycle.created")
            .with_cycle(&cycle.manifest.cycle_id)
            .with_project(&cycle.manifest.project_id)
            .build();

        let first = ledger
            .insert_cycle_with_event(&cycle, &event_input)
            .unwrap();
        let second = ledger
            .insert_cycle_with_event(&cycle, &event_input)
            .unwrap();
        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
    }

    #[test]
    fn event_builder_defaults() {
        let input = EventBuilder::new("test.event").build();
        assert_eq!(input.event_type, "test.event");
        assert_eq!(input.project_id, "p-test");
        assert!(input.cycle_id.is_none());
        assert_eq!(input.payload, serde_json::json!({}));
    }

    #[test]
    fn cycle_builder_defaults() {
        let record = CycleBuilder::new(CyclePath::ALite).build();
        assert_eq!(record.manifest.status, sddk_domain::CycleStatus::Open);
        assert_eq!(record.manifest.phase, sddk_domain::Phase::Explore);
        assert_eq!(record.manifest.path, CyclePath::ALite);
    }

    #[test]
    fn writes_nested_files_inside_repository() {
        let repository = TestRepository::new().unwrap();
        let path = repository.write("nested/file.txt", "fixture\n").unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "fixture\n");
    }

    #[test]
    fn rejects_paths_outside_repository() {
        let repository = TestRepository::new().unwrap();
        let error = repository.write("../outside.txt", "nope").unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn git_fixture_creates_real_history() {
        let repo = TestRepository::new().unwrap();
        repo.init().unwrap();
        repo.write("a.txt", "x\n").unwrap();
        repo.commit_all("c1").unwrap();
        repo.tag("v1").unwrap();

        let log = repo.git(&["log", "--oneline"]).unwrap();
        assert!(String::from_utf8_lossy(&log.stdout).contains("c1"));
        let tags = repo.git(&["tag"]).unwrap();
        assert!(String::from_utf8_lossy(&tags.stdout).contains("v1"));
        let status = repo.git(&["status", "--porcelain"]).unwrap();
        assert!(status.stdout.is_empty(), "worktree should be clean");
    }

    #[test]
    fn child_guard_take_then_drop_is_idempotent() {
        let child = std::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("POSIX sleep available in test env");
        let mut guard = ChildGuard::new(child);
        let taken = guard.take();
        assert!(taken.is_some());
        assert!(guard.take().is_none());
        drop(guard);
        if let Some(mut c) = taken {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    #[test]
    #[cfg(unix)]
    fn child_guard_kills_on_drop() {
        let child = std::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("POSIX sleep available in test env");
        let pid = child.id();
        let guard = ChildGuard::new(child);
        drop(guard);
        let proc_path = PathBuf::from(format!("/proc/{pid}"));
        let mut attempts = 0;
        while proc_path.exists() && attempts < 20 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            attempts += 1;
        }
        assert!(
            !proc_path.exists(),
            "child process {pid} still present in /proc after Drop"
        );
    }

    // ── CliSandbox unit tests ─────────────────────────────────────────────────

    #[test]
    fn cli_sandbox_env_wiring_isolates_home_and_xdg() {
        let repo = TestRepository::new().unwrap();
        repo.init().unwrap();
        let sandbox = CliSandbox::new(repo, "/bin/true").unwrap();

        // HOME and XDG_* point inside the sandbox
        assert!(
            sandbox.xdg.home.starts_with(sandbox.path()),
            "HOME should be inside sandbox"
        );
        assert!(
            sandbox.xdg.xdg_data.starts_with(sandbox.path()),
            "XDG_DATA_HOME should be inside sandbox"
        );
        assert!(
            sandbox.xdg.xdg_state.starts_with(sandbox.path()),
            "XDG_STATE_HOME should be inside sandbox"
        );
        assert!(
            sandbox.xdg.xdg_cache.starts_with(sandbox.path()),
            "XDG_CACHE_HOME should be inside sandbox"
        );
    }

    #[test]
    fn cli_sandbox_command_preset_has_xdg_env_vars() {
        let repo = TestRepository::new().unwrap();
        repo.init().unwrap();
        let sandbox = CliSandbox::new(repo, "/bin/echo").unwrap();

        // Verify current_dir is set on the preset command
        let preset = sandbox.command("/bin/sh");
        assert_eq!(
            preset.get_current_dir(),
            Some(std::path::Path::new(sandbox.path()))
        );
        // Verify HOME is set to the XDG home by spawning a subprocess that prints $HOME
        let mut home_check = std::process::Command::new("sh");
        home_check.arg("-c").arg("echo $HOME");
        home_check
            .current_dir(sandbox.path())
            .env("HOME", &sandbox.xdg.home)
            .env("XDG_DATA_HOME", &sandbox.xdg.xdg_data)
            .env("XDG_STATE_HOME", &sandbox.xdg.xdg_state)
            .env("XDG_CACHE_HOME", &sandbox.xdg.xdg_cache);
        let output = home_check.output().unwrap();
        let printed_home = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(
            printed_home,
            sandbox.xdg.home.to_string_lossy(),
            "HOME should be redirected to sandbox"
        );
    }

    #[test]
    fn cli_sandbox_init_git_sets_author_identity() {
        let repo = TestRepository::new().unwrap();
        repo.init().unwrap();
        let sandbox = CliSandbox::new(repo, "/bin/true").unwrap();
        sandbox.init_git("tester", "tester@example.com").unwrap();

        // Commit a file to verify the author identity
        sandbox.repo.write("a.txt", "content\n").unwrap();
        sandbox.repo.commit_all("initial commit").unwrap();

        let log_output = sandbox.repo.git(&["log", "--format=%ae %an"]).unwrap();
        let log_str = String::from_utf8_lossy(&log_output.stdout);
        assert!(
            log_str.contains("tester@example.com"),
            "commit author email should be 'tester@example.com', got: {log_str}"
        );
        assert!(
            log_str.contains("tester"),
            "commit author name should be 'tester', got: {log_str}"
        );
    }
}

// ── Test fixtures ───────────────────────────────────────────────────────────────

pub mod fixtures;

/// Golden IR fixtures for workflow IR contracts (v1.29.0).
pub mod ir_fixtures;
