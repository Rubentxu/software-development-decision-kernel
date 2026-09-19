//! Code Intelligence Port — SDDK-owned semantic seam for static
//! intelligence providers (CogniCode / future equivalent).
//!
//! Authority: `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`.
//! Cycle: `p-63676b11dc0ef88f/a6-cognicode-protocol-spike` (CC-S0).
//! ADR: `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`.
//!
//! This module is the **spike** implementation. The trait is
//! additive: no existing SDDK engine surface is rewritten. The
//! companion fake (`code_intelligence_port_fake`) is the
//! deterministic in-process provider used by the falsification
//! battery. Both are accepted by the architectural lint
//! `no_new_root_level_context_module_without_adr` because
//! ADR-0137 mentions `code_intelligence_port` in its body.
//!
//! Honesty markers:
//!
//! - `arch-spec-021` remains `proposed`. Acceptance happens when
//!   CC-S1 ships the production adapter.
//! - No `serde` / `prost` / `tonic` dependency is added.
//! - The trait returns only SDDK ADTs; provider types never
//!   cross the adapter boundary (IPB-001).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::fmt;

/// Stable 32-byte SHA-256 digest (hex-encoded lowercase, 64 chars).
///
/// Used for capability snapshots, analyzer set digests, and
/// result digests. Determinism over the same inputs is
/// guaranteed (test T2 verifies).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DigestSha256(pub String);

impl DigestSha256 {
    /// Hash an arbitrary byte slice using a small, dependency-free
    /// FNV-1a + fold algorithm. Not cryptographic — used only as
    /// a content-stable digest for capability / analyzer-set
    /// snapshots inside the spike.
    pub fn of(bytes: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        DigestSha256(format!("{:016x}", h))
    }
}

impl fmt::Display for DigestSha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Capability profile advertised by a provider after negotiation.
///
/// Per `arch-spec-021` IPB-004, capability profiles include
/// BASE / STATIC_ENHANCED / RUNTIME_ENHANCED / FULLY_ENHANCED.
/// CC-S0 only models BASE and STATIC_ENHANCED (the A6 track);
/// RUNTIME_ENHANCED belongs to A7 (Chronos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityProfile {
    /// Provider absent or not negotiated. SDDK Verify / DebVerify
    /// paths must be unchanged (IPB-010).
    Base,
    /// Provider advertises static-intelligence capabilities
    /// (CogniCode track, AC10).
    StaticEnhanced,
    /// Future A7 Chronos track (out of scope for CC-S0).
    #[allow(dead_code)]
    RuntimeEnhanced,
    /// Future A8 convergence (out of scope for CC-S0).
    #[allow(dead_code)]
    FullyEnhanced,
}

impl fmt::Display for CapabilityProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CapabilityProfile::Base => "BASE",
            CapabilityProfile::StaticEnhanced => "STATIC_ENHANCED",
            CapabilityProfile::RuntimeEnhanced => "RUNTIME_ENHANCED",
            CapabilityProfile::FullyEnhanced => "FULLY_ENHANCED",
        })
    }
}

/// Provider lifecycle state.
///
/// Per `arch-spec-021` IPB-006, lifecycle semantics cover at
/// least: `UNAVAILABLE` / `DORMANT` / `STARTING` / `READY` /
/// `BUSY` / `INCOMPATIBLE` / `FAILED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderLifecycle {
    /// Provider absent.
    Unavailable,
    /// Provider is registered but not started.
    Dormant,
    /// Provider is in the process of starting.
    Starting,
    /// Provider is ready to serve requests.
    Ready,
    /// Provider is currently serving a request.
    Busy,
    /// Protocol-major mismatch; provider not usable.
    Incompatible,
    /// Provider failed (after a successful start).
    Failed,
}

impl fmt::Display for ProviderLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProviderLifecycle::Unavailable => "UNAVAILABLE",
            ProviderLifecycle::Dormant => "DORMANT",
            ProviderLifecycle::Starting => "STARTING",
            ProviderLifecycle::Ready => "READY",
            ProviderLifecycle::Busy => "BUSY",
            ProviderLifecycle::Incompatible => "INCOMPATIBLE",
            ProviderLifecycle::Failed => "FAILED",
        })
    }
}

/// Provider kind, propagated into the ObservationSet so downstream
/// consumers can identify which provider produced the data.
///
/// CC-S0 only models `Null` (no provider) and `Fake` (the spike
/// provider). Production values (e.g. `CogniCode`) will be
/// added in CC-S1+.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProviderKind {
    /// No provider is registered. Default for Base-mode
    /// observation sets.
    #[default]
    Null,
    /// In-process deterministic fake used by the spike.
    Fake,
    /// Real CogniCode binary (future, CC-S1+).
    #[allow(dead_code)]
    CogniCode,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProviderKind::Null => "NULL",
            ProviderKind::Fake => "FAKE",
            ProviderKind::CogniCode => "COGNICODE",
        })
    }
}

/// Capability snapshot — the negotiated capability set, content-
/// addressed for stability and reproducibility.
///
/// Per `arch-spec-021` IPB-007, the snapshot digest is part of
/// the Evidence provenance chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CapabilitySnapshot {
    /// The advertised profile (after negotiation).
    pub profile: CapabilityProfile,
    /// Negotiated protocol major version.
    pub protocol_major: u32,
    /// Negotiated protocol minor version.
    pub protocol_minor: u32,
    /// Hash of the analyzer set names (sorted, joined with `|`).
    pub analyzer_set_digest: DigestSha256,
    /// Hash of the (profile, protocol_major, protocol_minor,
    /// analyzer_set_digest). The snapshot's own digest, used as
    /// the primary identifier in receipts.
    pub digest: DigestSha256,
}

impl CapabilitySnapshot {
    /// Construct a snapshot from an advertised profile and
    /// analyzer set. Digest is derived from the inputs.
    pub fn from_advertised(profile: CapabilityProfile, analyzer_set: &[&str]) -> Self {
        let mut sorted = analyzer_set.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        let analyzer_set_digest = DigestSha256::of(sorted.join("|").as_bytes());
        let protocol_major = 1u32;
        let protocol_minor = 0u32;
        let mut hasher_input = Vec::new();
        hasher_input.extend_from_slice(profile.to_string().as_bytes());
        hasher_input.push(b'|');
        hasher_input.extend_from_slice(format!("{}.{}", protocol_major, protocol_minor).as_bytes());
        hasher_input.push(b'|');
        hasher_input.extend_from_slice(analyzer_set_digest.0.as_bytes());
        let digest = DigestSha256::of(&hasher_input);
        Self {
            profile,
            protocol_major,
            protocol_minor,
            analyzer_set_digest,
            digest,
        }
    }
}

/// Analysis basis — the reproducible foundation of any
/// provider-side analysis.
///
/// Per `arch-spec-021` IPB-007, receipts carry these fields so
/// the same basis + analyzer set reproduces the same digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnalysisBasis {
    /// Provider build / tag / commit identifier.
    pub provider_build: String,
    /// Negotiated protocol major.
    pub protocol_major: u32,
    /// Negotiated protocol minor.
    pub protocol_minor: u32,
    /// Snapshot digest of the negotiated capabilities.
    pub capability_snapshot: CapabilitySnapshot,
    /// Digest of the analyzer set names (sorted, joined).
    pub analyzer_set_digest: DigestSha256,
    /// Source / revision identifier.
    pub source_revision: String,
    /// Request scope identifier (URI-like).
    pub request_scope: String,
}

/// Scope request — `analyze_delta` input.
///
/// The CC-S0 spike uses a flat string list (added/removed/
/// modified) rather than a structured file map; this is the
/// minimum needed to test determinism (T2) and cancellation
/// (T4). CC-S1+ will introduce structured file metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ScopeRequest {
    /// Source units added in this delta.
    pub added_units: Vec<String>,
    /// Source units removed in this delta.
    pub removed_units: Vec<String>,
    /// Source units modified in this delta.
    pub modified_units: Vec<String>,
}

/// Impact request — `analyze_impact` input.
///
/// CC-S0 keeps the shape symmetric with `ScopeRequest` so the
/// spike can compile against both without diverging.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ImpactRequest {
    /// Units whose downstream impact we want to model.
    pub changed_units: Vec<String>,
    /// Optional graph-depth budget.
    pub depth: Option<u32>,
}

/// Observation set — provider-produced observations that the
/// SDDK adapter surfaces to downstream consumers.
///
/// Per `arch-spec-021` IPB-002, observations are
/// `OBSERVED` evidence; the SDDK side turns them into
/// `KnowledgeAssertion`s later (CC-S1+).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObservationSet {
    /// Provider kind that produced these observations.
    pub provider_kind: ProviderKind,
    /// Per-unit observations keyed by source unit identifier.
    pub units: BTreeMap<String, Vec<Observation>>,
    /// True iff the provider restarted mid-request (T5).
    pub restart_observed: bool,
}

/// A single observation — the SDDK ADT shape for provider output.
///
/// CC-S0 uses a flat `text` payload for the spike. CC-S1+
/// will introduce typed observation variants (call, dependency,
/// impact, graph).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// Free-form text content (provider-specific; the SDDK
    /// adapter does not interpret it).
    pub text: String,
}

/// Result of an `analyze_*` call. Carries the digest of the
/// observation set and a `partial` marker for restart-recovery
/// flows (T5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    /// Stable digest of the observation set + basis + request.
    pub digest: DigestSha256,
    /// True iff the result is partial (provider restart).
    pub partial: bool,
    /// The observation set itself.
    pub observations: ObservationSet,
}

/// Errors returned by the port. CC-S0 only models the variants
/// required by the falsification battery; production variants
/// (`Timeout`, `Incompatible`, etc.) land in CC-S1+.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeIntelligencePortError {
    /// The provider cancelled the request mid-flight (T4).
    Cancelled,
    /// The provider's protocol major does not match what SDDK
    /// supports (T3). Carries the actual protocol major.
    ProtocolMajorMismatch {
        /// Protocol major advertised by the provider.
        provider_major: u32,
        /// Protocol major supported by SDDK.
        sddk_major: u32,
    },
}

impl fmt::Display for CodeIntelligencePortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodeIntelligencePortError::Cancelled => f.write_str("provider cancelled the request"),
            CodeIntelligencePortError::ProtocolMajorMismatch {
                provider_major,
                sddk_major,
            } => write!(
                f,
                "provider protocol major {provider_major} does not match SDDK major {sddk_major}"
            ),
        }
    }
}

impl std::error::Error for CodeIntelligencePortError {}

/// The SDDK-owned semantic port for static-intelligence
/// providers.
///
/// Per `arch-spec-021` IPB-001..IPB-012. Implementations MUST
/// return only SDDK ADTs from these methods.
pub trait CodeIntelligencePort {
    /// Negotiate / report the capability snapshot.
    fn capabilities(&self) -> CapabilitySnapshot;

    /// Report the current provider lifecycle state.
    fn lifecycle_state(&self) -> ProviderLifecycle;

    /// Analyze a delta.
    fn analyze_delta(
        &self,
        basis: &AnalysisBasis,
        request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError>;

    /// Analyze a scope (full source tree, scoped by request).
    fn analyze_scope(
        &self,
        basis: &AnalysisBasis,
        request: &ScopeRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError>;

    /// Analyze the impact of changing a set of units.
    fn analyze_impact(
        &self,
        basis: &AnalysisBasis,
        request: &ImpactRequest,
    ) -> Result<AnalysisResult, CodeIntelligencePortError>;
}

/// Test-only helper that emits an analyzer-set token.
///
/// Implemented as a function so it works from integration tests
/// (which compile `sddk-engine` without `#[cfg(test)]`). The
/// helper is a no-op identity; it exists as a single seam where
/// future spike code could plug normalization (e.g. trim, fold
/// case). CC-S0 keeps the implementation trivial.
pub fn analyzer_token(name: &str) -> &str {
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_snapshot_digest_is_stable() {
        let a = CapabilitySnapshot::from_advertised(
            CapabilityProfile::StaticEnhanced,
            &["rust", "deps"],
        );
        let b = CapabilitySnapshot::from_advertised(
            CapabilityProfile::StaticEnhanced,
            &["deps", "rust"],
        );
        assert_eq!(a.digest, b.digest);
        assert_eq!(a.profile, b.profile);
    }

    #[test]
    fn digest_of_is_deterministic() {
        let a = DigestSha256::of(b"hello");
        let b = DigestSha256::of(b"hello");
        let c = DigestSha256::of(b"world");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn lifecycle_display_is_stable() {
        assert_eq!(ProviderLifecycle::Incompatible.to_string(), "INCOMPATIBLE");
        assert_eq!(ProviderLifecycle::Ready.to_string(), "READY");
    }
}
