//! Agent Host substrate — owns identity, lease, fencing, and retry.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentHost.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-079-AGENT-HOST-SUBSTRATE.md

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::retry::{Clock, RetryPolicy};
use crate::run_view::{ActionKind, DecisionRecord, DecisionVerdict};

/// Kind of agent (P8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Human,
    Auto,
    External,
}

/// Stable identity for an agent host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentIdentity {
    id: String,
    display_name: String,
    kind: AgentKind,
}

impl AgentIdentity {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>, kind: AgentKind) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            kind,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
    pub fn kind(&self) -> AgentKind {
        self.kind
    }
}

/// Errors from lease operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LeaseError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("lease conflict: owner `{owner}` holds token {fencing_token}")]
    Conflict { owner: String, fencing_token: i64 },
    #[error("lease expired")]
    Expired,
}

/// Errors from `execute_with_retry` and `execute_decision`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExecuteError<E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static> {
    #[error("retries exhausted for {kind:?}: {attempts} attempts; last error: {last_error}")]
    RetriesExhausted {
        kind: ActionKind,
        attempts: u32,
        last_error: E,
    },
    #[error("execution aborted")]
    Aborted,
}

/// Errors from `execute_decision`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ExecuteDecisionError {
    #[error("decision not actionable: verdict is not Allow (got {verdict:?})")]
    NotActionable { verdict: DecisionVerdict },
    #[error("lease error: {0}")]
    Lease(#[from] LeaseError),
    #[error("execution error: {0}")]
    Execute(String),
}

/// Abstraction over the lease store. Real implementations forward to
/// `sddk_storage::Ledger::acquire_cycle_lease / release_cycle_lease`.
pub trait LeaseStore: Send + Sync + std::fmt::Debug {
    fn acquire(
        &self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<LeaseRecord, LeaseError>;
    fn release(&self, cycle_id: &str, owner: &str, fencing_token: i64) -> Result<bool, LeaseError>;
}

/// In-memory lease store for tests + scaffolding.
#[derive(Debug, Default)]
pub struct InMemoryLeaseStore {
    inner: Mutex<HashMap<String, LeaseRecord>>,
}

impl InMemoryLeaseStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LeaseStore for InMemoryLeaseStore {
    fn acquire(
        &self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<LeaseRecord, LeaseError> {
        let mut guard = self.inner.lock().expect("poisoned");
        let entry = guard
            .entry(cycle_id.to_string())
            .or_insert_with(|| LeaseRecord {
                cycle_id: cycle_id.to_string(),
                owner: String::new(),
                fencing_token: 0,
                acquired_at_ms: 0,
                expires_at_ms: 0,
            });
        if entry.expires_at_ms > now_ms && entry.owner != owner {
            return Err(LeaseError::Conflict {
                owner: entry.owner.clone(),
                fencing_token: entry.fencing_token,
            });
        }
        entry.fencing_token += 1;
        entry.owner = owner.to_string();
        entry.acquired_at_ms = now_ms;
        entry.expires_at_ms = expires_at_ms;
        Ok(entry.clone())
    }

    fn release(&self, cycle_id: &str, owner: &str, fencing_token: i64) -> Result<bool, LeaseError> {
        let mut guard = self.inner.lock().expect("poisoned");
        if let Some(entry) = guard.get_mut(cycle_id)
            && entry.owner == owner
            && entry.fencing_token == fencing_token
        {
            entry.owner.clear();
            entry.expires_at_ms = 0;
            return Ok(true);
        }
        Ok(false)
    }
}

/// Lease state returned from the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRecord {
    pub cycle_id: String,
    pub owner: String,
    pub fencing_token: i64,
    pub acquired_at_ms: i64,
    pub expires_at_ms: i64,
}

/// RAII lease handle. `Drop` releases the lease atomically.
#[derive(Debug)]
pub struct LeaseHandle {
    record: LeaseRecord,
    store: Arc<dyn LeaseStore>,
    released: bool,
}

impl LeaseHandle {
    pub(crate) fn new(record: LeaseRecord, store: Arc<dyn LeaseStore>) -> Self {
        Self {
            record,
            store,
            released: false,
        }
    }

    pub fn cycle_id(&self) -> &str {
        &self.record.cycle_id
    }
    pub fn owner(&self) -> &str {
        &self.record.owner
    }
    pub fn fencing_token(&self) -> i64 {
        self.record.fencing_token
    }
    pub fn expires_at_ms(&self) -> i64 {
        self.record.expires_at_ms
    }
    pub fn acquired_at_ms(&self) -> i64 {
        self.record.acquired_at_ms
    }

    /// Explicit release (returns the handle as `()`).
    pub fn release(mut self) -> Result<(), LeaseError> {
        self.do_release()?;
        Ok(())
    }

    fn do_release(&mut self) -> Result<(), LeaseError> {
        if self.released {
            return Ok(());
        }
        self.store.release(
            &self.record.cycle_id,
            &self.record.owner,
            self.record.fencing_token,
        )?;
        self.released = true;
        Ok(())
    }
}

impl Drop for LeaseHandle {
    fn drop(&mut self) {
        let _ = self.do_release();
    }
}

/// Per-decision execution receipt. `Serialize + Deserialize` so it can
/// be persisted to the ledger as durable provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DecisionReceipt {
    pub decision: DecisionRecord,
    pub fencing_token: i64,
    pub executed_at_ms: i64,
    pub retries_used: u32,
    pub agent_id: String,
    /// Provider chain executed for this decision. `Some` when the
    /// decision was routed through a `ProviderRouter` (AGENT-HOST-002);
    /// `None` for lease-only mutations or projection reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_route: Option<Vec<crate::provider_router::RouteAttempt>>,
}

/// Agent Host — owns identity, lease, fencing, retry.
pub struct AgentHost {
    identity: AgentIdentity,
    store: Arc<dyn LeaseStore>,
    clock: Arc<dyn Clock>,
    retry_policies: HashMap<ActionKind, RetryPolicy>,
}

impl AgentHost {
    /// Build a host with default retry policies:
    /// - `Retry`: `RetryPolicy::Exponential { base_ms: 1000, max_backoff_ms: 10000, max_attempts: 3 }`
    /// - everything else: `RetryPolicy::None` (execute once, no retry)
    pub fn new(identity: AgentIdentity, store: Arc<dyn LeaseStore>, clock: Arc<dyn Clock>) -> Self {
        let mut retry_policies = HashMap::new();
        for kind in ALL_ACTION_KINDS {
            retry_policies.insert(
                *kind,
                if *kind == ActionKind::Retry {
                    RetryPolicy::Exponential {
                        base_ms: 1000,
                        max_backoff_ms: 10_000,
                        max_attempts: 3,
                    }
                } else {
                    RetryPolicy::None
                },
            );
        }
        Self {
            identity,
            store,
            clock,
            retry_policies,
        }
    }

    pub fn identity(&self) -> &AgentIdentity {
        &self.identity
    }

    pub fn retry_policy(&self, kind: ActionKind) -> RetryPolicy {
        self.retry_policies
            .get(&kind)
            .cloned()
            .unwrap_or(RetryPolicy::None)
    }

    pub fn acquire_lease(&self, cycle_id: &str, ttl_ms: i64) -> Result<LeaseHandle, LeaseError> {
        let now = self.clock.now_ms() as i64;
        let expires = now.saturating_add(ttl_ms);
        let record = self
            .store
            .acquire(cycle_id, self.identity.id(), now, expires)?;
        Ok(LeaseHandle::new(record, Arc::clone(&self.store)))
    }

    pub fn execute_with_retry<F, T, E>(&self, kind: ActionKind, op: F) -> Result<T, ExecuteError<E>>
    where
        F: Fn() -> Result<T, E>,
        E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        let policy = self.retry_policy(kind);
        let max_attempts = policy.max_attempts();
        let mut last_err: Option<E> = None;
        for _attempt in 1..=max_attempts {
            match op() {
                Ok(v) => return Ok(v),
                Err(e) => {
                    last_err = Some(e);
                    // Future cycles can extend the Clock trait with `sleep`.
                    // For now, backoff is observable via clock advancement
                    // after the operation (deterministic with MockClock).
                }
            }
        }
        Err(ExecuteError::RetriesExhausted {
            kind,
            attempts: max_attempts,
            last_error: last_err.expect("at least one attempt ran"),
        })
    }

    pub fn execute_decision<F, T, E>(
        &self,
        decision: &DecisionRecord,
        cycle_id: &str,
        op: F,
    ) -> Result<DecisionReceipt, ExecuteDecisionError>
    where
        F: Fn() -> Result<T, E>,
        E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
        T: 'static,
    {
        if decision.verdict() != &DecisionVerdict::Allow {
            return Err(ExecuteDecisionError::NotActionable {
                verdict: decision.verdict().clone(),
            });
        }
        let kind = decision.kind();
        let handle = self
            .acquire_lease(cycle_id, DEFAULT_LEASE_TTL_MS)
            .map_err(ExecuteDecisionError::Lease)?;
        let fencing_token = handle.fencing_token();
        let policy = self.retry_policy(kind);
        let max_attempts = policy.max_attempts();
        let mut last_err: Option<E> = None;
        let mut retries_used: u32 = 0;
        let result = (|| -> Result<T, ExecuteError<E>> {
            for attempt in 1..=max_attempts {
                match op() {
                    Ok(v) => return Ok(v),
                    Err(e) => {
                        last_err = Some(e);
                        retries_used = attempt.saturating_sub(1);
                        if attempt >= max_attempts {
                            break;
                        }
                    }
                }
            }
            Err(ExecuteError::RetriesExhausted {
                kind,
                attempts: max_attempts,
                last_error: last_err.expect("at least one attempt ran"),
            })
        })();
        // Release the lease regardless of outcome.
        let _ = handle.release();
        let executed_at_ms = self.clock.now_ms() as i64;
        match result {
            Ok(_) => Ok(DecisionReceipt {
                decision: decision.clone(),
                fencing_token,
                executed_at_ms,
                retries_used,
                agent_id: self.identity.id().to_string(),
                provider_route: None,
            }),
            Err(ExecuteError::RetriesExhausted { .. }) => Err(ExecuteDecisionError::Execute(
                format!("retries exhausted after {retries_used} retries"),
            )),
            Err(ExecuteError::Aborted) => Err(ExecuteDecisionError::Execute("aborted".to_string())),
        }
    }
}

const DEFAULT_LEASE_TTL_MS: i64 = 60_000;

const ALL_ACTION_KINDS: &[ActionKind] = &[
    ActionKind::Start,
    ActionKind::Resume,
    ActionKind::Abort,
    ActionKind::Approve,
    ActionKind::Escalate,
    ActionKind::Retry,
    ActionKind::Reconcile,
];
