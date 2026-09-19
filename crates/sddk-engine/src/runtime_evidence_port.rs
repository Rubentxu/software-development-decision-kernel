//! Runtime evidence provider seam (A7).
//!
//! Cycle: `p-63676b11dc0ef88f/aiw-s5-chronos-runtime` (AIW-S5).
//!
//! Mirror of `code_intelligence_port` (arch-spec-021) for the runtime
//! side: a provider-neutral trait that yields typed
//! `SoftwareObservation`s from a live or captured execution, without
//! importing any provider (Chronos) type into the domain.
//!
//! Reuses the SAME digest/basis machinery as A6: `DigestSha256` and
//! `ObservationBasis::for_provider_result`, so the VerifyKernel can
//! treat runtime observations identically to static ones.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use crate::code_intelligence_port::{DigestSha256, ProviderKind};

use crate::observation::types::SoftwareObservation;

/// Capability profile a runtime provider can offer (subset relevant
/// to A7 claims; expansion is additive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilitySnapshot {
    pub provider_name: String,
    pub provider_version: String,
    pub protocol_major: u32,
    /// Tool names the provider exposes (sorted, deduplicated).
    pub tools: Vec<String>,
}

/// Request for one governed runtime capture: run (or attach to) a
/// target under observation and produce a typed summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCaptureRequest {
    /// Absolute or PATH-resolvable path of the target binary.
    pub program: String,
    /// Arguments for the target.
    pub args: Vec<String>,
    /// Working directory for the target (None = provider default).
    pub cwd: Option<String>,
    /// Kill/capture budget; the provider must honor it or fail.
    pub timeout_ms: u64,
}

/// Typed, provider-neutral result of one runtime capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCaptureResult {
    /// Stable id of the capture session (provider-assigned).
    pub session_id: String,
    /// Wall duration of the observed execution, in nanoseconds
    /// (None when the provider could not measure it).
    pub duration_ns: Option<u64>,
    /// Total captured events.
    pub total_events: u64,
    /// Events grouped by type, sorted by key.
    pub event_counts: BTreeMap<String, u64>,
    /// Thread count observed.
    pub thread_count: u64,
    /// Target exit status when the provider observed it
    /// (None = not observed; do NOT infer success).
    pub exit_status: Option<i32>,
    /// Content digest over the canonical result material.
    pub digest: DigestSha256,
    /// Observations derived from this capture (typed evidence).
    pub observations: ObservationSet,
}

/// Typed observation set (runtime flavor of the A6 one).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservationSet {
    pub observations: Vec<SoftwareObservation>,
}

impl ObservationSet {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, o: SoftwareObservation) {
        self.observations.push(o);
    }
    pub fn len(&self) -> usize {
        self.observations.len()
    }
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }
}

impl RuntimeCaptureResult {
    /// Convert the port-level observation set into the canonical
    /// observation set consumed by the VerifyKernel (same shape the
    /// static side produces).
    pub fn to_canonical_observation_set(&self) -> crate::observation::types::ObservationSet {
        use crate::evidence_ref::{EvidenceKind, EvidenceRef};
        use crate::observation::types::{ObservationBasis, ObservationOrigin, SoftwareObservation};
        let mut out = crate::observation::types::ObservationSet::new();
        for o in &self.observations.observations {
            out.insert(SoftwareObservation::declare(
                o.subject.clone(),
                o.stance,
                EvidenceRef::new(EvidenceKind::Adhoc, o.evidence.locator.clone()),
                ObservationOrigin::RuntimeProvider,
                ObservationBasis::for_provider_result("aiw-s5-capture", &self.digest.0),
                None,
                "chronos-mcp",
            ));
        }
        out
    }
}

/// Errors, mirroring `CodeIntelligencePortError` semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimePortError {
    /// Provider binary missing / pipe closed.
    Unavailable,
    /// Provider response malformed; fail closed, never guess.
    InvalidPayload(&'static str),
    /// Provider reported an execution error.
    ProviderError(String),
}

impl std::fmt::Display for RuntimePortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimePortError::Unavailable => write!(f, "runtime provider unavailable"),
            RuntimePortError::InvalidPayload(what) => {
                write!(f, "runtime provider payload invalid: {what}")
            }
            RuntimePortError::ProviderError(m) => write!(f, "runtime provider error: {m}"),
        }
    }
}

/// The seam. One method: capture. Capability negotiation is
/// provider-specific (the MCP adapter does the handshake).
pub trait RuntimeEvidencePort {
    fn capabilities(&self) -> RuntimeCapabilitySnapshot;
    fn capture(
        &self,
        req: &RuntimeCaptureRequest,
    ) -> Result<RuntimeCaptureResult, RuntimePortError>;
}

/// Canonical provider kind for runtime evidence.
/// NOTE: ProviderKind has no Chronos variant yet (A7 reserved); the
/// adapter maps runtime evidence to `ProviderKind::Null` at the port
/// level and carries `RuntimeProvider` in `ObservationOrigin` — the
/// origin is the typed runtime marker. Adding a `Chronos` variant is
/// an additive change for a later slice.
#[allow(dead_code)]
pub const RUNTIME_PROVIDER_KIND: ProviderKind = ProviderKind::Null;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_set_starts_empty() {
        let s = ObservationSet::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn error_display_is_stable() {
        assert_eq!(
            RuntimePortError::Unavailable.to_string(),
            "runtime provider unavailable"
        );
        assert_eq!(
            RuntimePortError::InvalidPayload("no session id").to_string(),
            "runtime provider payload invalid: no session id"
        );
    }
}
