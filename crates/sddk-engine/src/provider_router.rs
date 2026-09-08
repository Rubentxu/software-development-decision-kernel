//! Provider router — bounded failover over a registered list of providers.
//!
//! Implements ADR-027 semantics (retry vs reroute vs disable) on top of the
//! closed-set [`ProviderFailure`] taxonomy adopted from SPEC-026.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::circuit_breaker::{
    CircuitBreakerStore, CircuitState, ProviderIdentity, ProviderKind, refresh_open_to_half_open,
};
use crate::retry::Clock;
use crate::run_view::ActionKind;
use crate::telemetry::{ProviderResult, TelemetrySink, UsageRecord};

// ── Failure taxonomy (SPEC-026) ──────────────────────────────────────────────

/// Closed-set taxonomy of provider/model transport failures. Adopted
/// verbatim from SPEC-026 §Error taxonomy. **Do not conflate** with
/// [`ExecutionFailure`] or [`TaskOutcome`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum ProviderFailure {
    RateLimited { retry_after_ms: Option<u64> },
    QuotaExhausted,
    AuthenticationFailed,
    AuthorizationFailed,
    ModelUnavailable,
    ServiceUnavailable { http_status: Option<u16> },
    Timeout { elapsed_ms: u64 },
    TransportFailure(String),
    UnknownProviderFailure,
}

impl ProviderFailure {
    pub fn kind(&self) -> ProviderFailureKind {
        match self {
            Self::RateLimited { .. } => ProviderFailureKind::RateLimited,
            Self::QuotaExhausted => ProviderFailureKind::QuotaExhausted,
            Self::AuthenticationFailed => ProviderFailureKind::AuthenticationFailed,
            Self::AuthorizationFailed => ProviderFailureKind::AuthorizationFailed,
            Self::ModelUnavailable => ProviderFailureKind::ModelUnavailable,
            Self::ServiceUnavailable { .. } => ProviderFailureKind::ServiceUnavailable,
            Self::Timeout { .. } => ProviderFailureKind::Timeout,
            Self::TransportFailure(_) => ProviderFailureKind::TransportFailure,
            Self::UnknownProviderFailure => ProviderFailureKind::UnknownProviderFailure,
        }
    }

    pub fn is_open_circuit(&self) -> bool {
        matches!(self, Self::QuotaExhausted | Self::ModelUnavailable)
    }

    pub fn is_disable(&self) -> bool {
        matches!(self, Self::AuthenticationFailed | Self::AuthorizationFailed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailureKind {
    RateLimited,
    QuotaExhausted,
    AuthenticationFailed,
    AuthorizationFailed,
    ModelUnavailable,
    ServiceUnavailable,
    Timeout,
    TransportFailure,
    UnknownProviderFailure,
}

/// Provider-side error returned by `Provider::execute`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderError {
    pub failure: ProviderFailure,
    pub retries_used: u32,
    pub last_attempt_at_ms: i64,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.failure)
    }
}

impl std::error::Error for ProviderError {}

/// Provider-side success payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderOutput {
    pub kind: ProviderOutputKind,
    pub wall_ms: u64,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost: Option<f64>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOutputKind {
    Ok,
}

// ── Provider trait ──────────────────────────────────────────────────────────

/// Provider capability surface. Mirrors `LeaseStore` (sync, trait-based).
/// Future cycles (AGENT-HOST-003+) introduce async-trait variants for
/// real network providers.
pub trait Provider: Send + Sync + std::fmt::Debug {
    fn identity(&self) -> &ProviderIdentity;

    /// Execute `cmd` once. MUST NOT panic on transient transport errors.
    /// MUST return an error tagged with a `ProviderFailure` variant.
    fn execute(&self, kind: ActionKind, cmd: &str) -> Result<ProviderOutput, ProviderError>;
}

// ── Router policy ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouterPolicy {
    /// Failures that MUST NOT auto-retry on the same provider.
    pub terminal: Vec<ProviderFailureKind>,
    /// Failures that MAY be retried on the same provider (with backoff).
    pub retryable: Vec<ProviderFailureKind>,
    /// Failures that MUST trigger immediate failover to another provider.
    pub failover: Vec<ProviderFailureKind>,
    /// Max providers attempted before giving up.
    pub max_hops: u8,
    /// Cooldown after which an Open circuit transitions to HalfOpen.
    pub circuit_cooldown_ms: i64,
}

impl Default for RouterPolicy {
    fn default() -> Self {
        Self {
            // Per SPEC-026 §Sample policy:
            terminal: vec![
                ProviderFailureKind::QuotaExhausted,
                ProviderFailureKind::AuthenticationFailed,
                ProviderFailureKind::AuthorizationFailed,
                ProviderFailureKind::ModelUnavailable,
            ],
            retryable: vec![
                ProviderFailureKind::RateLimited,
                ProviderFailureKind::ServiceUnavailable,
                ProviderFailureKind::Timeout,
                ProviderFailureKind::TransportFailure,
                ProviderFailureKind::UnknownProviderFailure,
            ],
            failover: vec![
                ProviderFailureKind::QuotaExhausted,
                ProviderFailureKind::AuthenticationFailed,
                ProviderFailureKind::AuthorizationFailed,
                ProviderFailureKind::ModelUnavailable,
            ],
            max_hops: 3,
            circuit_cooldown_ms: 60_000,
        }
    }
}

// ── Route output ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteAttempt {
    pub identity: ProviderIdentity,
    pub failure: ProviderFailure,
    pub at_ms: i64,
    pub retries_used: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteError {
    NoEligibleProvider {
        kind: ActionKind,
    },
    AllProvidersFailed {
        attempts: Vec<RouteAttempt>,
    },
    Disabled {
        identity: ProviderIdentity,
        reason: String,
    },
    LeaseExpired {
        cycle_id: String,
        expires_at_ms: i64,
    },
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEligibleProvider { kind } => {
                write!(f, "no eligible provider for {:?}", kind)
            }
            Self::AllProvidersFailed { attempts } => {
                write!(f, "all {} providers failed", attempts.len())
            }
            Self::Disabled { identity, reason } => {
                write!(f, "route {:?} disabled: {}", identity, reason)
            }
            Self::LeaseExpired {
                cycle_id,
                expires_at_ms,
            } => write!(
                f,
                "lease for cycle {} expired at {}",
                cycle_id, expires_at_ms
            ),
        }
    }
}

impl std::error::Error for RouteError {}

// ── ProviderRouter ─────────────────────────────────────────────────────────

pub struct ProviderRouter {
    providers: Vec<Arc<dyn Provider>>,
    breaker: Arc<dyn CircuitBreakerStore>,
    telemetry: Arc<dyn TelemetrySink>,
    clock: Arc<dyn Clock>,
    policy: RouterPolicy,
}

impl std::fmt::Debug for ProviderRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRouter")
            .field("providers", &self.providers.len())
            .field("breaker", &self.breaker)
            .field("telemetry", &"...")
            .field("policy", &self.policy)
            .finish()
    }
}

impl ProviderRouter {
    pub fn new(
        providers: Vec<Arc<dyn Provider>>,
        breaker: Arc<dyn CircuitBreakerStore>,
        telemetry: Arc<dyn TelemetrySink>,
        clock: Arc<dyn Clock>,
        policy: RouterPolicy,
    ) -> Self {
        Self {
            providers,
            breaker,
            telemetry,
            clock,
            policy,
        }
    }

    pub fn policy(&self) -> &RouterPolicy {
        &self.policy
    }

    pub fn providers(&self) -> &[Arc<dyn Provider>] {
        &self.providers
    }

    /// Route a command across the registered providers. Iterates in
    /// registration order; skips Disabled routes entirely. See
    /// `REQ-ProviderFailover` Scenario 1..=9 for behavior contract.
    pub fn route(&self, kind: ActionKind, cmd: &str) -> Result<ProviderOutput, RouteError> {
        let now_ms = self.clock.now_ms() as i64;
        if self.providers.is_empty() {
            return Err(RouteError::NoEligibleProvider { kind });
        }

        let mut attempts: Vec<RouteAttempt> = Vec::new();
        let mut hops_taken: u8 = 0;

        for provider in &self.providers {
            if hops_taken >= self.policy.max_hops {
                break;
            }
            let identity = provider.identity().clone();
            let state = self.breaker.state(&identity);
            let observed_state =
                refresh_open_to_half_open(state, 0, self.policy.circuit_cooldown_ms, now_ms);
            if observed_state == CircuitState::Disabled {
                return Err(RouteError::Disabled {
                    identity,
                    reason: "circuit_disabled".to_string(),
                });
            }
            if observed_state == CircuitState::Open {
                continue;
            }

            hops_taken = hops_taken.saturating_add(1);
            let result =
                self.try_provider(&**provider, kind, cmd, &identity, now_ms, &mut attempts);
            if let Some(output) = result {
                return Ok(output);
            }
        }

        if attempts.is_empty() {
            return Err(RouteError::NoEligibleProvider { kind });
        }
        Err(RouteError::AllProvidersFailed { attempts })
    }

    /// Try one provider: at most one in-place retry if retryable,
    /// otherwise advance immediately. Returns `Some(output)` on success.
    fn try_provider(
        &self,
        provider: &dyn Provider,
        kind: ActionKind,
        cmd: &str,
        identity: &ProviderIdentity,
        started_at_ms: i64,
        attempts: &mut Vec<RouteAttempt>,
    ) -> Option<ProviderOutput> {
        let mut retries_used: u32 = 0;
        let now_ms = self.clock.now_ms() as i64;

        let first = provider.execute(kind, cmd);
        match first {
            Ok(out) => {
                let ProviderOutput {
                    kind: _,
                    wall_ms,
                    input_tokens,
                    output_tokens,
                    cost,
                    currency,
                } = out;
                self.breaker.record_success(identity, now_ms);
                self.telemetry.record(UsageRecord {
                    identity: identity.clone(),
                    action_kind: kind,
                    attempts: retries_used.saturating_add(1),
                    failovers: attempts.len() as u32,
                    wall_ms,
                    input_tokens,
                    output_tokens,
                    cost,
                    currency: currency.clone(),
                    result: ProviderResult::Ok,
                });
                Some(ProviderOutput {
                    kind: ProviderOutputKind::Ok,
                    wall_ms,
                    input_tokens,
                    output_tokens,
                    cost,
                    currency,
                })
            }
            Err(err) => {
                let failure = err.failure.clone();
                let last_attempt_at_ms = err.last_attempt_at_ms;
                attempts.push(RouteAttempt {
                    identity: identity.clone(),
                    failure: failure.clone(),
                    at_ms: last_attempt_at_ms,
                    retries_used: err.retries_used,
                });
                retries_used = err.retries_used;

                if failure.is_disable() {
                    self.breaker.disable(identity, last_attempt_at_ms);
                    self.telemetry.record(UsageRecord {
                        identity: identity.clone(),
                        action_kind: kind,
                        attempts: retries_used.saturating_add(1),
                        failovers: attempts.len().saturating_sub(1) as u32,
                        wall_ms: 0,
                        input_tokens: None,
                        output_tokens: None,
                        cost: None,
                        currency: None,
                        result: ProviderResult::TerminalFailure,
                    });
                    return None;
                }

                if failure.is_open_circuit() {
                    self.breaker.trip(
                        identity,
                        last_attempt_at_ms,
                        self.policy.circuit_cooldown_ms,
                    );
                } else {
                    let _ = self.breaker.record_failure(identity, last_attempt_at_ms);
                }

                let kind_fk = failure.kind();
                let is_retryable = self.policy.retryable.contains(&kind_fk);
                if is_retryable {
                    let retry = provider.execute(kind, cmd);
                    match retry {
                        Ok(out) => {
                            let ProviderOutput {
                                kind: _,
                                wall_ms,
                                input_tokens,
                                output_tokens,
                                cost,
                                currency,
                            } = out;
                            self.breaker.record_success(identity, last_attempt_at_ms);
                            self.telemetry.record(UsageRecord {
                                identity: identity.clone(),
                                action_kind: kind,
                                attempts: 2,
                                failovers: attempts.len().saturating_sub(1) as u32,
                                wall_ms,
                                input_tokens,
                                output_tokens,
                                cost,
                                currency: currency.clone(),
                                result: ProviderResult::Ok,
                            });
                            return Some(ProviderOutput {
                                kind: ProviderOutputKind::Ok,
                                wall_ms,
                                input_tokens,
                                output_tokens,
                                cost,
                                currency,
                            });
                        }
                        Err(err2) => {
                            attempts.push(RouteAttempt {
                                identity: identity.clone(),
                                failure: err2.failure.clone(),
                                at_ms: err2.last_attempt_at_ms,
                                retries_used: err2.retries_used,
                            });
                            if err2.failure.is_open_circuit() {
                                self.breaker.trip(
                                    identity,
                                    err2.last_attempt_at_ms,
                                    self.policy.circuit_cooldown_ms,
                                );
                            } else {
                                let _ = self
                                    .breaker
                                    .record_failure(identity, err2.last_attempt_at_ms);
                            }
                            self.telemetry.record(UsageRecord {
                                identity: identity.clone(),
                                action_kind: kind,
                                attempts: 2,
                                failovers: attempts.len().saturating_sub(1) as u32,
                                wall_ms: 0,
                                input_tokens: None,
                                output_tokens: None,
                                cost: None,
                                currency: None,
                                result: ProviderResult::RetryableFailure,
                            });
                            return None;
                        }
                    }
                }

                // Terminal or non-retryable: record telemetry, advance.
                self.telemetry.record(UsageRecord {
                    identity: identity.clone(),
                    action_kind: kind,
                    attempts: retries_used.saturating_add(1),
                    failovers: attempts.len().saturating_sub(1) as u32,
                    wall_ms: 0,
                    input_tokens: None,
                    output_tokens: None,
                    cost: None,
                    currency: None,
                    result: ProviderResult::TerminalFailure,
                });
                let _ = started_at_ms;
                None
            }
        }
    }
}

/// Test/mock convenience: build an identity quickly.
pub fn mock_identity(id: &str, kind: ProviderKind) -> ProviderIdentity {
    ProviderIdentity {
        id: id.to_string(),
        kind,
        credentials_route: format!("mock://{id}"),
        model: None,
    }
}
