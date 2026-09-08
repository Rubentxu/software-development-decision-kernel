//! Provider circuit breaker — closed-set state machine adopted from SPEC-026.
//!
//! Key = `(provider, credential_route, optional model)` (SPEC-026 §Circuit
//! breaker key). The breaker decides whether a route is eligible for the next
//! attempt and reacts to successes/failures recorded by the
//! [`crate::provider_router::ProviderRouter`].

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Closed-set circuit state. `Disabled` is reserved for routes that have
/// been administratively removed (auth revoked, provider deprecated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
    Disabled,
}

/// Identity of a provider route. This is the **circuit breaker key**.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderIdentity {
    pub id: String,
    pub kind: ProviderKind,
    pub credentials_route: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Llm,
    Tool,
    Mock,
    Deterministic,
}

/// Persisted circuit entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitEntry {
    pub identity: ProviderIdentity,
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub opened_at_ms: i64,
    pub cooldown_ms: i64,
}

/// Backing store for circuit state. Implementations MUST be
/// `Send + Sync + Debug` (matching `LeaseStore`).
pub trait CircuitBreakerStore: Send + Sync + std::fmt::Debug {
    fn state(&self, identity: &ProviderIdentity) -> CircuitState;
    fn record_success(&self, identity: &ProviderIdentity, at_ms: i64);
    fn record_failure(&self, identity: &ProviderIdentity, at_ms: i64) -> CircuitState;
    fn trip(&self, identity: &ProviderIdentity, at_ms: i64, cooldown_ms: i64);
    fn disable(&self, identity: &ProviderIdentity, at_ms: i64);
    fn close(&self, identity: &ProviderIdentity);
}

/// In-memory implementation. Mirrors `InMemoryLeaseStore`.
#[derive(Debug)]
pub struct InMemoryCircuitBreakerStore {
    inner: Mutex<HashMap<ProviderIdentity, CircuitEntry>>,
}

impl Default for InMemoryCircuitBreakerStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryCircuitBreakerStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn seed(&self, identity: ProviderIdentity, state: CircuitState, cooldown_ms: i64) {
        let mut guard = self.inner.lock().expect("poisoned");
        guard.insert(
            identity.clone(),
            CircuitEntry {
                identity,
                state,
                consecutive_failures: 0,
                opened_at_ms: 0,
                cooldown_ms,
            },
        );
    }
}

impl CircuitBreakerStore for InMemoryCircuitBreakerStore {
    fn state(&self, identity: &ProviderIdentity) -> CircuitState {
        let guard = self.inner.lock().expect("poisoned");
        guard
            .get(identity)
            .map(|e| e.state)
            .unwrap_or(CircuitState::Closed)
    }

    fn record_success(&self, identity: &ProviderIdentity, _at_ms: i64) {
        let mut guard = self.inner.lock().expect("poisoned");
        if let Some(entry) = guard.get_mut(identity) {
            entry.consecutive_failures = 0;
            if entry.state == CircuitState::HalfOpen {
                entry.state = CircuitState::Closed;
            }
        }
    }

    fn record_failure(&self, identity: &ProviderIdentity, at_ms: i64) -> CircuitState {
        let mut guard = self.inner.lock().expect("poisoned");
        let entry = guard.entry(identity.clone()).or_insert(CircuitEntry {
            identity: identity.clone(),
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at_ms: 0,
            cooldown_ms: 60_000,
        });
        entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        // HalfOpen probe failure trips back to Open with refreshed timestamp.
        if entry.state == CircuitState::HalfOpen {
            entry.state = CircuitState::Open;
            entry.opened_at_ms = at_ms;
            return CircuitState::Open;
        }
        CircuitState::Closed
    }

    fn trip(&self, identity: &ProviderIdentity, at_ms: i64, cooldown_ms: i64) {
        let mut guard = self.inner.lock().expect("poisoned");
        let entry = guard.entry(identity.clone()).or_insert(CircuitEntry {
            identity: identity.clone(),
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at_ms: 0,
            cooldown_ms,
        });
        entry.state = CircuitState::Open;
        entry.opened_at_ms = at_ms;
        entry.cooldown_ms = cooldown_ms;
    }

    fn disable(&self, identity: &ProviderIdentity, at_ms: i64) {
        let mut guard = self.inner.lock().expect("poisoned");
        let entry = guard.entry(identity.clone()).or_insert(CircuitEntry {
            identity: identity.clone(),
            state: CircuitState::Closed,
            consecutive_failures: 0,
            opened_at_ms: at_ms,
            cooldown_ms: 0,
        });
        entry.state = CircuitState::Disabled;
    }

    fn close(&self, identity: &ProviderIdentity) {
        let mut guard = self.inner.lock().expect("poisoned");
        if let Some(entry) = guard.get_mut(identity) {
            entry.state = CircuitState::Closed;
            entry.consecutive_failures = 0;
            entry.opened_at_ms = 0;
        }
    }
}

/// Inspect an `Open` circuit and, if cooldown elapsed, transition it to
/// `HalfOpen`. Returns the (possibly updated) state.
///
/// Used by the router before evaluating eligibility. Pure function of
/// `(state, opened_at_ms, cooldown_ms, now_ms)`.
pub fn refresh_open_to_half_open(
    state: CircuitState,
    opened_at_ms: i64,
    cooldown_ms: i64,
    now_ms: i64,
) -> CircuitState {
    if state == CircuitState::Open && now_ms.saturating_sub(opened_at_ms) >= cooldown_ms {
        CircuitState::HalfOpen
    } else {
        state
    }
}
