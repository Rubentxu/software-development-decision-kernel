//! Code Intelligence Port — SDDK-owned semantic seam for static
//! intelligence providers (CogniCode / future equivalent).
//!
//! Authority: `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`.
//! Cycle: `p-63676b11dc0ef88f/a6-cognicode-protocol-spike` (CC-S0).
//!         `p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness` (CC-S1).
//! ADR: `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`,
//!      `docs/architecture/adrs/ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md`.
//! Acceptance: `docs/architecture/specs/arch-acceptance-coverage-001.md`.
//!
//! CC-S0 spike added the trait, the four mandatory operations, and
//! the lifecycle/capability snapshot. CC-S1 extends the snapshot with
//! `available_strategies[]` and `semantic_classes[]`, and adds
//! `coverage_evaluation(contract, basis)` returning a `CoverageEvaluation`
//! per ADR-0139. The companion fake (`code_intelligence_port_fake`)
//! was the deterministic in-process provider used by CC-S0's
//! falsification battery; CC-S1 moves it under a `test-support`
//! feature gate so downstream consumers do not see it by default.

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
    /// Hash an arbitrary byte slice with real SHA-256 (64 hex chars).
    ///
    /// AIW-S1: replaces the CC-S0 FNV-64 provisional so durable
    /// evidence refs carry a cryptographic identity. The spike's
    /// 16-char digests are NOT upgradeable in place; consumers who
    /// need the historical spike identity can keep referencing
    /// recorded receipts (they are historical facts, not live refs).
    pub fn of(bytes: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let out = hasher.finalize();
        let mut s = String::with_capacity(64);
        for b in out {
            s.push_str(&format!("{b:02x}"));
        }
        DigestSha256(s)
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
///
/// CC-S1 extends the snapshot with `available_strategies[]` and
/// `semantic_classes[]` so that coverage evaluation can decide
/// whether a strategy can satisfy a claim without re-querying
/// the provider.
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
    /// Strategies the provider can serve (e.g. "lightweight",
    /// "full"). Empty if the provider does not declare strategies.
    /// CC-S1: per ADR-0139 §5, `lightweight` may satisfy a
    /// contract whose required capabilities are within its
    /// declared `semantic_classes[]`. Empty array means
    /// "unknown" and forces the semantics dimension to `Unknown`.
    pub available_strategies: Vec<String>,
    /// Semantic classes the provider declares to be representable
    /// (e.g. "fn", "struct", "trait" for symbol kinds;
    /// "find_usages", "analyze_impact" for relation classes).
    /// Empty if the provider does not declare them.
    pub semantic_classes: Vec<String>,
}

impl CapabilitySnapshot {
    /// Construct a snapshot from an advertised profile and
    /// analyzer set. Digest is derived from the inputs.
    pub fn from_advertised(profile: CapabilityProfile, analyzer_set: &[&str]) -> Self {
        Self::from_advertised_with_capabilities(profile, analyzer_set, &[], &[])
    }

    /// Construct a snapshot with explicit strategies and semantic
    /// classes (CC-S1). The digest includes the new fields so
    /// snapshots with different `available_strategies` /
    /// `semantic_classes` have different digests.
    pub fn from_advertised_with_capabilities(
        profile: CapabilityProfile,
        analyzer_set: &[&str],
        available_strategies: &[&str],
        semantic_classes: &[&str],
    ) -> Self {
        let mut sorted = analyzer_set.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        let analyzer_set_digest = DigestSha256::of(sorted.join("|").as_bytes());
        let protocol_major = 1u32;
        let protocol_minor = 0u32;
        let mut strategies: Vec<String> =
            available_strategies.iter().map(|s| s.to_string()).collect();
        strategies.sort();
        strategies.dedup();
        let mut classes: Vec<String> = semantic_classes.iter().map(|s| s.to_string()).collect();
        classes.sort();
        classes.dedup();
        let mut hasher_input = Vec::new();
        hasher_input.extend_from_slice(profile.to_string().as_bytes());
        hasher_input.push(b'|');
        hasher_input.extend_from_slice(format!("{}.{}", protocol_major, protocol_minor).as_bytes());
        hasher_input.push(b'|');
        hasher_input.extend_from_slice(analyzer_set_digest.0.as_bytes());
        hasher_input.push(b'|');
        hasher_input.extend_from_slice(strategies.join(",").as_bytes());
        hasher_input.push(b'|');
        hasher_input.extend_from_slice(classes.join(",").as_bytes());
        let digest = DigestSha256::of(&hasher_input);
        Self {
            profile,
            protocol_major,
            protocol_minor,
            analyzer_set_digest,
            digest,
            available_strategies: strategies,
            semantic_classes: classes,
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
    /// Provider process could not be spawned, died, or its pipe
    /// closed mid-request (AIW-S1).
    Unavailable,
    /// The provider did not answer within the request timeout
    /// (AIW-S1).
    #[allow(dead_code)] // wired when timeout handling lands in CC-S1+
    Timeout,
    /// The provider answered but the payload did not conform to
    /// the expected contract — fails closed, never coerced
    /// (AIW-S1).
    InvalidPayload,
    /// The provider reported an application error for the request
    /// (AIW-S1).
    ProviderError,
    /// The requested analysis mode has no provider mapping
    /// (AIW-S1; e.g. delta analysis needs structured file metadata
    /// the MCP contract does not expose yet).
    Unsupported,
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
            CodeIntelligencePortError::Unavailable => {
                f.write_str("provider unavailable (spawn failure or pipe closed)")
            }
            CodeIntelligencePortError::Timeout => f.write_str("provider timed out"),
            CodeIntelligencePortError::InvalidPayload => {
                f.write_str("provider payload failed strict validation")
            }
            CodeIntelligencePortError::ProviderError => f.write_str("provider returned an error"),
            CodeIntelligencePortError::Unsupported => {
                f.write_str("analysis mode unsupported by this provider adapter")
            }
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

    /// Evaluate coverage for a given contract against a
    /// reproducible basis. CC-S1 / ADR-0139 / `arch-acceptance-
    /// coverage-001`.
    ///
    /// Returns `EvidenceGap` if the provider is unavailable or
    /// incompatible. Otherwise returns a `CoverageEvaluation`
    /// whose `CoverageVerdict` is `Satisfied` only when the three
    /// independent dimensions (inventory, semantics, operational)
    /// are all `Demonstrated` against the contract's
    /// `required_capabilities[]`.
    fn coverage_evaluation(
        &self,
        contract: &CoverageContract,
        basis: &CoverageBasis,
    ) -> Result<CoverageEvaluation, EvidenceGap>;
}

// ── CC-S1: Coverage Contract types (ADR-0139, arch-acceptance-coverage-001) ──

/// Identifier of a capability or semantic class required by a
/// coverage contract.
///
/// Two kinds are recognised: `SymbolKind` (e.g. "fn", "struct",
/// "trait") and `RelationClass` (e.g. "find_usages",
/// "analyze_impact"). The name is opaque to the runtime; the
/// provider's `CapabilitySnapshot.semantic_classes[]` declares
/// which ones it can represent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequiredCapability {
    /// Kind of capability required.
    pub kind: RequiredCapabilityKind,
    /// Opaque identifier (matches `semantic_classes[]` entries).
    pub name: String,
}

/// The two kinds of capabilities a `CoverageContract` can require.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequiredCapabilityKind {
    /// A symbol kind the analysis must represent (e.g. "fn").
    SymbolKind,
    /// A relation class the analysis must support (e.g.
    /// "find_usages").
    RelationClass,
}

impl fmt::Display for RequiredCapabilityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RequiredCapabilityKind::SymbolKind => "SymbolKind",
            RequiredCapabilityKind::RelationClass => "RelationClass",
        })
    }
}

/// Reference to the scope the contract applies to.
///
/// `ScopeRef` carries the Git revision pin and the include/exclude
/// globs that produced the inventory. The combination
/// `(revision, include_globs, exclude_globs)` is what makes the
/// `CoverageBasis` reproducible.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeRef {
    /// Repository identifier (e.g. "sddk-framework").
    pub repo: String,
    /// Git revision pinned for this evaluation.
    pub revision: String,
    /// Globs of files included in the scope.
    pub include_globs: Vec<String>,
    /// Globs of files excluded from the scope.
    pub exclude_globs: Vec<String>,
}

/// Independent inventory of expected files for a scope.
///
/// Produced by a deterministic command (e.g. `just
/// inventory-static-enhanced`) at the pinned revision. CC-S1's
/// `tests/fixtures/static_enhanced/inventory_v1.json` is the
/// first artefact. The inventory is the **only** source of truth
/// for "what files should be analysed".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Inventory {
    /// Pinned Git revision.
    pub revision: String,
    /// Rule version (SemVer-style string, e.g. "1.0.0"). Bumped
    /// on rule changes; each bump is a contract change.
    pub rule_version: String,
    /// Expected files in the scope, sorted.
    pub expected_files: Vec<String>,
}

/// Coverage contract — versioned policy owned by SDDK that
/// determines whether a static enhanced capability may be
/// announced for a scope.
///
/// Per ADR-0139 §2 and `arch-acceptance-coverage-001 §2 AR-1`,
/// the contract is approved **before** evaluation; its version
/// is part of the `CoverageBasis`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoverageContract {
    /// Identifier (e.g. "static-enhanced-workspace-v1").
    pub contract_id: String,
    /// Contract version (e.g. "1.0.0").
    pub contract_version: String,
    /// Consumer identifier (e.g. "verify-kernel::static_evidence").
    pub consumer: String,
    /// Scope the contract applies to.
    pub scope: ScopeRef,
    /// Capabilities the provider must declare. Empty means
    /// "no requirement" (only inventory + operational matter).
    pub required_capabilities: Vec<RequiredCapability>,
}

/// Reproducible basis for a coverage evaluation.
///
/// Per ADR-0139 §3, the basis pins the revision, the inventory,
/// the provider strategy and version, and the contract revision
/// that was applied. Two evaluations with byte-equal bases and
/// identical observations produce byte-equal verdicts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoverageBasis {
    /// Pinned Git revision.
    pub revision: String,
    /// Independent inventory.
    pub inventory: Inventory,
    /// Strategy the provider executed ("lightweight", "full", …).
    pub provider_strategy: String,
    /// Provider version (e.g. "0.97.1").
    pub provider_version: String,
    /// Contract revision applied.
    pub contract_revision: String,
}

/// Value of one coverage dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DimensionValue {
    /// The dimension is fully demonstrated.
    Demonstrated,
    /// Some evidence exists; gaps are enumerated.
    Partial,
    /// Evidence is in part but missing critical pieces.
    Incomplete,
    /// No information is available.
    Unknown,
}

impl fmt::Display for DimensionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DimensionValue::Demonstrated => "Demonstrated",
            DimensionValue::Partial => "Partial",
            DimensionValue::Incomplete => "Incomplete",
            DimensionValue::Unknown => "Unknown",
        })
    }
}

/// A specific gap in coverage, classified by kind.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoverageGap {
    /// A file the contract requires that the provider did not
    /// analyse.
    MissingFile(String),
    /// A semantic class the contract requires that the provider
    /// does not declare.
    MissingSemanticClass(String),
    /// An operational issue (errors, cancellation, truncation).
    OperationalError(String),
    /// A dimension is unknown because the provider doesn't report
    /// it. Carries the dimension name.
    UnknownDimension(String),
}

impl fmt::Display for CoverageGap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoverageGap::MissingFile(p) => write!(f, "missing_file: {p}"),
            CoverageGap::MissingSemanticClass(c) => {
                write!(f, "missing_semantic_class: {c}")
            }
            CoverageGap::OperationalError(e) => write!(f, "operational_error: {e}"),
            CoverageGap::UnknownDimension(d) => write!(f, "unknown_dimension: {d}"),
        }
    }
}

/// The verdict over a `CoverageContract` applied to a
/// `CoverageBasis` with concrete provider observations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoverageEvaluation {
    /// Inventory dimension (M5, AR-3).
    pub inventory: DimensionValue,
    /// Semantics dimension (M5, AR-3).
    pub semantics: DimensionValue,
    /// Operational dimension (M5, AR-3).
    pub operational: DimensionValue,
    /// Specific gaps registered when dimensions are not
    /// `Demonstrated`. Empty iff all three are `Demonstrated`.
    pub gaps: Vec<CoverageGap>,
    /// Final verdict (AR-3).
    pub verdict: CoverageVerdict,
}

impl CoverageEvaluation {
    /// Construct a fully-Demonstrated evaluation (all dimensions
    /// pass). `gaps` is empty.
    pub fn satisfied() -> Self {
        Self {
            inventory: DimensionValue::Demonstrated,
            semantics: DimensionValue::Demonstrated,
            operational: DimensionValue::Demonstrated,
            gaps: Vec::new(),
            verdict: CoverageVerdict::Satisfied,
        }
    }

    /// Construct an Incomplete evaluation from the three
    /// dimensions and a non-empty `gaps` list. Verdict is
    /// derived.
    pub fn incomplete(
        inventory: DimensionValue,
        semantics: DimensionValue,
        operational: DimensionValue,
        gaps: Vec<CoverageGap>,
    ) -> Self {
        let verdict = if inventory == DimensionValue::Demonstrated
            && semantics == DimensionValue::Demonstrated
            && operational == DimensionValue::Demonstrated
            && gaps.is_empty()
        {
            CoverageVerdict::Satisfied
        } else {
            CoverageVerdict::Incomplete { gaps: gaps.clone() }
        };
        Self {
            inventory,
            semantics,
            operational,
            gaps,
            verdict,
        }
    }
}

/// The binary outcome: all-green or not-and-why.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoverageVerdict {
    /// All dimensions `Demonstrated`, no gaps.
    Satisfied,
    /// At least one dimension is `< Demonstrated`, gaps
    /// enumerated.
    Incomplete {
        /// The same `gaps` as in `CoverageEvaluation`.
        gaps: Vec<CoverageGap>,
    },
}

/// The non-success state when the provider cannot serve the
/// request (lifecycle != Ready, or contract missing). AR-4.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EvidenceGap {
    /// Provider is not Ready.
    ProviderUnavailable {
        /// Provider lifecycle observed.
        lifecycle: ProviderLifecycle,
    },
    /// Provider is incompatible (protocol major mismatch).
    ProviderIncompatible {
        /// Provider protocol major.
        provider_major: u32,
        /// SDDK protocol major.
        sddk_major: u32,
    },
}

impl fmt::Display for EvidenceGap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceGap::ProviderUnavailable { lifecycle } => {
                write!(f, "provider_unavailable: {lifecycle}")
            }
            EvidenceGap::ProviderIncompatible {
                provider_major,
                sddk_major,
            } => write!(
                f,
                "provider_incompatible: provider_major={provider_major} sddk_major={sddk_major}"
            ),
        }
    }
}

impl std::error::Error for EvidenceGap {}

/// Default coverage evaluation that does not require a provider.
///
/// Implements AR-2 (determinism) by checking capabilities
/// declared by the snapshot against the contract's required
/// capabilities. The provider-specific adapter is responsible
/// for the inventory and operational dimensions; this default
/// produces `Unknown` for those two and `Incomplete` for
/// semantics unless the snapshot declares every required
/// capability.
///
/// Used by tests and by adapters that haven't yet specialised
/// the inventory/operational dimensions.
pub fn default_coverage_evaluation(
    contract: &CoverageContract,
    basis: &CoverageBasis,
    snapshot: &CapabilitySnapshot,
) -> CoverageEvaluation {
    let mut gaps: Vec<CoverageGap> = Vec::new();

    // Semantics dimension.
    let semantics = if contract.required_capabilities.is_empty() {
        // No capabilities required -> Demonstrated (trivially).
        DimensionValue::Demonstrated
    } else if snapshot.semantic_classes.is_empty() {
        // Snapshot does not declare any semantic classes -> Unknown.
        gaps.push(CoverageGap::UnknownDimension("semantics".to_string()));
        DimensionValue::Unknown
    } else {
        let mut missing: Vec<String> = Vec::new();
        for req in &contract.required_capabilities {
            if !snapshot.semantic_classes.iter().any(|c| c == &req.name) {
                missing.push(req.name.clone());
            }
        }
        if missing.is_empty() {
            DimensionValue::Demonstrated
        } else {
            for m in &missing {
                gaps.push(CoverageGap::MissingSemanticClass(m.clone()));
            }
            DimensionValue::Incomplete
        }
    };

    if basis.provider_strategy == "lightweight" {
        // AR-5: a `lightweight` strategy that the provider does
        // not advertise in `available_strategies[]` cannot be
        // used to satisfy a contract. The semantics check above
        // already records missing classes by name; here we
        // additionally note that the strategy itself was not
        // declared.
        if !snapshot
            .available_strategies
            .iter()
            .any(|s| s == "lightweight")
        {
            for req in &contract.required_capabilities {
                if req.kind == RequiredCapabilityKind::RelationClass {
                    gaps.push(CoverageGap::MissingSemanticClass(req.name.clone()));
                }
            }
        }
    }

    // Inventory dimension: SDDK does not invent it (M6).
    gaps.push(CoverageGap::UnknownDimension("inventory".to_string()));
    let inventory = DimensionValue::Unknown;

    // Operational dimension: same.
    gaps.push(CoverageGap::UnknownDimension("operational".to_string()));
    let operational = DimensionValue::Unknown;

    CoverageEvaluation::incomplete(inventory, semantics, operational, gaps)
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
