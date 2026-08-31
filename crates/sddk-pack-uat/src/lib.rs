//! sddk-pack-uat — UAT Pack
//!
//! This crate is the SDDK UAT pack: it provides the user acceptance testing
//! guided runner, form DSL, scenario management, and evidence collection.
//!
//! ## Pack boundary
//!
//! All UAT-specific types live here. The universal evidence model
//! ([`sddk_domain::EvidenceBundle`]) remains in `sddk-domain` — this pack
//! depends on `sddk-domain` and re-exports UAT types for ergonomic access.
//!
//! ## Backward compatibility
//!
//! During the Phase 4 extraction, types are re-exported from `sddk_domain::uat`
//! to avoid breaking existing imports. Once all consumers are migrated, the
//! canonical location for UAT types will be this crate.
//!
//! ## Capabilities
//!
//! - `uat.plan.create` — create a UAT plan from a spec or discovery run
//! - `uat.plan.execute` — execute a UAT plan in guided or automated mode
//! - `uat.plan.approve` — record human approval decision for a scenario
//! - `uat.evidence.collect` — collect and store evidence artifacts
//! - `uat.receipt.emit` — emit a signed execution receipt

// Re-export the universal evidence model from sddk-domain for use by UAT types.
pub use sddk_domain::EvidenceArtifact;
pub use sddk_domain::EvidenceBundle;
pub use sddk_domain::EvidenceEnvironment;
pub use sddk_domain::EvidenceExecution;
pub use sddk_domain::EvidenceKind;
pub use sddk_domain::EvidenceKindItem;

// Re-export UAT types from sddk-domain during the extraction transition period.
// These re-exports maintain backward compatibility while the pack boundary is established.
// After Phase 4, canonical locations will be in this crate.
pub use sddk_domain::uat;

// Re-export evidence-related types that UAT uses.
pub use sddk_domain::EvidenceAutomationStatus as UatAutomationStatus;
pub use sddk_domain::EvidenceBlastRadius as UatBlastRadius;
pub use sddk_domain::EvidenceExpectedCheck as UatExpectedCheck;
pub use sddk_domain::EvidenceOrigin as UatOrigin;
pub use sddk_domain::EvidenceRiskClassification as UatRiskClassification;

// Pack manifest and conformance.
pub use sddk_domain::pack::{
    PackConsequence, PackDiagnostic, PackError, PackManifest, PackRisk, load_pack_manifest,
    parse_pack_manifest, validate_pack_manifest,
};

pub mod conformance;
