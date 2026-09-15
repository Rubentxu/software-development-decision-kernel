// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_mutation/types.rs — A3-S6 / AC6 public types.
//
// Closed vocabularies and data shapes for critical mutation probes.
// See `mod.rs` for the state-class contract.

use crate::architectural_contract::ContractId;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// MutationId / GuardId (typed newtypes)
// ─────────────────────────────────────────────────────────────────────────────

/// Stable identity for one mutation spec.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MutationId(pub String);

impl MutationId {
    /// Construct a `MutationId`.
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// Borrow the raw string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MutationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable identity for a guard.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GuardId(pub String);

impl GuardId {
    /// Construct a `GuardId`.
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// Borrow the raw string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GuardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MutationKind (REQ-AC6-011)
// ─────────────────────────────────────────────────────────────────────────────

/// Closed taxonomy of the initial critical mutations (AC-038-003).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MutationKind {
    /// A domain module acquires a provider SDK dependency
    /// (`inject domain → tonic dependency`).
    ProviderTypeLeak,
    /// An alignment/instruction module acquires a governance/authority
    /// dependency (`inject Alignment → AuthorityEngine dependency`).
    AlignmentToGovernance,
    /// A workbook acquires a canonical write path
    /// (`inject Workbook → canonical write path`).
    WorkbookCanonicalWrite,
    /// A second canonical writer appears
    /// (`inject second event writer`).
    SecondCanonicalWriter,
}

impl MutationKind {
    /// All variants, in canonical order.
    pub const ALL: [Self; 4] = [
        Self::ProviderTypeLeak,
        Self::AlignmentToGovernance,
        Self::WorkbookCanonicalWrite,
        Self::SecondCanonicalWriter,
    ];

    /// Canonical short tag for receipts and digests.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::ProviderTypeLeak => "provider_type_leak",
            Self::AlignmentToGovernance => "alignment_to_governance",
            Self::WorkbookCanonicalWrite => "workbook_canonical_write",
            Self::SecondCanonicalWriter => "second_canonical_writer",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GuardScope / GuardCheck (REQ-AC6-009, 010)
// ─────────────────────────────────────────────────────────────────────────────

/// The path prefixes a guard inspects. An empty scope means "the whole
/// sandbox".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardScope(pub Vec<String>);

impl GuardScope {
    /// The whole sandbox.
    pub fn all() -> Self {
        Self(Vec::new())
    }

    /// A scope limited to the given path prefixes.
    pub fn prefixes(prefixes: &[&str]) -> Self {
        Self(prefixes.iter().map(|s| (*s).to_string()).collect())
    }

    /// Whether `path` is inside this scope.
    pub fn matches(&self, path: &str) -> bool {
        self.0.is_empty() || self.0.iter().any(|p| path.starts_with(p.as_str()))
    }
}

/// The closed set of guard checks (REQ-AC6-009).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardCheck {
    /// Fail on any line containing one of `forbidden` (substring match).
    ForbiddenLines {
        /// Substrings that must not appear in scope.
        forbidden: Vec<String>,
    },
    /// Fail when the number of lines containing `pattern` exceeds
    /// `max_allowed`.
    MaxOccurrences {
        /// Substring to count.
        pattern: String,
        /// Maximum tolerated occurrences.
        max_allowed: usize,
    },
}

/// A guard: an id, a scope and a check.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationGuard {
    /// Guard identity.
    pub id: GuardId,
    /// Path prefixes inspected.
    pub scope: GuardScope,
    /// The check to run.
    pub check: GuardCheck,
}

// ─────────────────────────────────────────────────────────────────────────────
// MutationInjection (REQ-AC6-004, 005)
// ─────────────────────────────────────────────────────────────────────────────

/// How a mutation is written into the sandbox (closed, REQ-AC6-004).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationInjection {
    /// Append `line` after the existing content of the target path.
    AppendLine(String),
    /// Prepend `line` before the existing content of the target path.
    PrependLine(String),
    /// Insert `line` immediately before the first line containing `needle`.
    /// If no line contains `needle`, nothing is applied.
    InsertBeforeFirstLineContaining {
        /// Marker to locate.
        needle: String,
        /// Line to insert.
        line: String,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// MutationSpec (REQ-AC6-012, 018)
// ─────────────────────────────────────────────────────────────────────────────

/// One mutation probe specification: what to inject, where, and which guard is
/// expected to detect it (AC-038-002).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationSpec {
    /// Spec identity.
    pub id: MutationId,
    /// Which critical invariant this mutation exercises.
    pub kind: MutationKind,
    /// Repo-relative path to mutate (created if absent).
    pub target_path: String,
    /// The injection to apply.
    pub injection: MutationInjection,
    /// The guard expected to detect the injected violation.
    pub expected_guard: GuardId,
    /// The architectural contract this mutation exercises, when it maps to one.
    /// Detected probes contribute this id to AC4's `contradiction_witnesses`.
    pub expected_contract: Option<ContractId>,
}

// ─────────────────────────────────────────────────────────────────────────────
// GuardHit / MutationProbe (REQ-AC6-006..008)
// ─────────────────────────────────────────────────────────────────────────────

/// One guard violation inside the sandbox.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardHit {
    /// Path of the offending file.
    pub path: String,
    /// 1-based line number.
    pub line: u32,
    /// The trimmed offending line.
    pub matched: String,
}

/// The result of one mutation probe (AC-038-002 shape).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationProbe {
    /// Which spec produced this probe.
    pub spec_id: MutationId,
    /// The mutation kind.
    pub kind: MutationKind,
    /// The path that was mutated.
    pub target_path: String,
    /// Whether the injection actually applied.
    pub mutation_applied: bool,
    /// The guard that was expected to detect the violation.
    pub expected_guard: GuardId,
    /// Whether the expected guard detected the injected violation.
    pub detected: bool,
    /// The guard's evidence (file:line + matched text).
    pub evidence: Vec<GuardHit>,
    /// The contract this probe exercises, when the spec declared one.
    /// Detected probes contribute this id to AC4's witnesses.
    pub expected_contract: Option<ContractId>,
}

// ─────────────────────────────────────────────────────────────────────────────
// MutationSuiteReceipt (REQ-AC6-015..017)
// ─────────────────────────────────────────────────────────────────────────────

/// The outcome of running a set of mutation probes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationSuiteReceipt {
    /// Probes, sorted by `spec_id`.
    pub probes: Vec<MutationProbe>,
    /// Whether every probe was detected.
    pub all_detected: bool,
    /// sha256 over the canonical receipt payload.
    pub digest: [u8; 32],
}

impl MutationSuiteReceipt {
    /// Spec ids that were not detected, sorted.
    pub fn undetected(&self) -> Vec<MutationId> {
        self.probes
            .iter()
            .filter(|p| !p.detected)
            .map(|p| p.spec_id.clone())
            .collect()
    }

    /// Contract ids contributed by detected probes (sorted, deduplicated).
    ///
    /// The order and shape AC4's `ConformanceInputs.contradiction_witnesses`
    /// consumes.
    pub fn witnesses(&self) -> Vec<ContractId> {
        let mut out: Vec<ContractId> = Vec::new();
        for p in &self.probes {
            if !p.detected {
                continue;
            }
            if let Some(c) = &p.expected_contract {
                out.push(c.clone());
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// Number of probes that were detected.
    pub fn detected_count(&self) -> usize {
        self.probes.iter().filter(|p| p.detected).count()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors surfaced while running mutation probes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutationError {
    /// A spec referenced a guard id that is not in the supplied guard set.
    UnknownGuard {
        /// The mutation spec that referenced it.
        spec: MutationId,
        /// The missing guard id.
        guard: GuardId,
    },
}

impl std::fmt::Display for MutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownGuard { spec, guard } => write!(
                f,
                "architecture_mutation: mutation `{spec}` expects unknown guard `{guard}`"
            ),
        }
    }
}

impl std::error::Error for MutationError {}

impl MutationProbe {
    /// Borrow the contract this probe exercises, when the spec declared one.
    pub fn expected_contract(&self) -> Option<&ContractId> {
        self.expected_contract.as_ref()
    }
}
