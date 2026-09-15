// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_debverify/types.rs — A3-S7 / AC5 public types.
//
// Closed vocabularies and the audit receipt for the global DebVerify pass.
// See `mod.rs` for the state-class and exit-criterion contract.

use crate::architectural_contract::ContractId;
use crate::knowledge::EventTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// DebVerifyFindingKind (REQ-AC5-005)
// ─────────────────────────────────────────────────────────────────────────────

/// The five global challenge classes AC5 performs (roadmap nouns).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DebVerifyFindingKind {
    /// Two or more contracts claim the same authority.
    ShadowAuthority,
    /// An authority/ownership contract names an owner that is absent from the graph.
    MissingOwner,
    /// A forbidden dependency exists in the graph despite a contract forbidding it.
    AuthorityBypass,
    /// A compatibility window has elapsed without a replacement.
    StaleCompatibility,
    /// The same subject is declared both authority-owned and projection-only.
    Contradiction,
}

impl DebVerifyFindingKind {
    /// All variants, in canonical (severity-then-name) order.
    pub const ALL: [Self; 5] = [
        Self::ShadowAuthority,
        Self::AuthorityBypass,
        Self::MissingOwner,
        Self::Contradiction,
        Self::StaleCompatibility,
    ];

    /// Canonical short tag for receipts and digests.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::ShadowAuthority => "shadow_authority",
            Self::MissingOwner => "missing_owner",
            Self::AuthorityBypass => "authority_bypass",
            Self::StaleCompatibility => "stale_compatibility",
            Self::Contradiction => "contradiction",
        }
    }

    /// Deterministic severity (REQ-AC5-006).
    pub fn severity(self) -> FindingSeverity {
        match self {
            Self::ShadowAuthority | Self::AuthorityBypass => FindingSeverity::Critical,
            Self::MissingOwner | Self::Contradiction => FindingSeverity::High,
            Self::StaleCompatibility => FindingSeverity::Medium,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FindingSeverity (REQ-AC5-006)
// ─────────────────────────────────────────────────────────────────────────────

/// Closed severity taxonomy. **Not a score**: no numeric weight exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FindingSeverity {
    /// The architecture is actively incoherent (duplicate authority, live bypass).
    Critical,
    /// A structural gap that must be repaired.
    High,
    /// Debt-shaped but not incoherent.
    Medium,
}

impl FindingSeverity {
    /// All variants.
    pub const ALL: [Self; 3] = [Self::Critical, Self::High, Self::Medium];

    /// Canonical short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DebVerifyFinding (REQ-AC5-007)
// ─────────────────────────────────────────────────────────────────────────────

/// One finding of the global challenge pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebVerifyFinding {
    /// The challenge class.
    pub kind: DebVerifyFindingKind,
    /// Deterministic severity, derived from `kind`.
    pub severity: FindingSeverity,
    /// The subject identifiers involved (sorted, deduplicated).
    pub subjects: Vec<String>,
    /// The contracts involved (sorted, deduplicated).
    pub contract_ids: Vec<ContractId>,
    /// Human-readable explanation (informational only, never authority).
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// DebVerifyAudit (REQ-AC5-013..018)
// ─────────────────────────────────────────────────────────────────────────────

/// The result of a global DebVerify pass.
///
/// A **PROJECTION** (ADR-0095): reconstructible from the graph, the contract
/// set and `now`. Deliberately distinct from AC4's change-scoped
/// `ArchitectureConformanceDelta` — there is no conversion between them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebVerifyAudit {
    /// Findings, sorted by `(kind, subjects, contract_ids)`.
    pub findings: Vec<DebVerifyFinding>,
    /// How many contracts were inspected.
    pub audited_contracts: usize,
    /// The overlay digest at audit time.
    pub graph_digest: Vec<u8>,
    /// sha256 over the canonical audit payload.
    pub digest: [u8; 32],
    /// The evaluation timestamp passed by the caller.
    pub evaluated_at: EventTime,
}

impl DebVerifyAudit {
    /// Findings of one kind, in order.
    pub fn by_kind(&self, kind: DebVerifyFindingKind) -> Vec<&DebVerifyFinding> {
        self.findings.iter().filter(|f| f.kind == kind).collect()
    }

    /// Findings per kind (only kinds with at least one finding appear).
    pub fn counts(&self) -> BTreeMap<DebVerifyFindingKind, usize> {
        let mut out: BTreeMap<DebVerifyFindingKind, usize> = BTreeMap::new();
        for f in &self.findings {
            *out.entry(f.kind).or_insert(0) += 1;
        }
        out
    }

    /// Whether the audit found nothing.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// The distinct contracts implicated by any finding, sorted.
    pub fn implicated_contracts(&self) -> Vec<ContractId> {
        let mut out: Vec<ContractId> = Vec::new();
        for f in &self.findings {
            out.extend(f.contract_ids.iter().cloned());
        }
        out.sort();
        out.dedup();
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors surfaced while running the DebVerify audit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DebVerifyError {
    /// A subject identifier could not be read from a contract payload.
    ///
    /// Kept for API stability: current detectors never fail (identifiers are
    /// always readable), so this is reserved.
    UnreadableSubject {
        /// The contract whose subject could not be read.
        contract: ContractId,
    },
}

impl std::fmt::Display for DebVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnreadableSubject { contract } => write!(
                f,
                "architecture_debverify: could not read a subject from contract `{contract}`"
            ),
        }
    }
}

impl std::error::Error for DebVerifyError {}
