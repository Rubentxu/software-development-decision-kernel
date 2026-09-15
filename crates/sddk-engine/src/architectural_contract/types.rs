// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/types.rs — typed newtypes + provenance refs.
//
// # State class
//
// All types in this file are state-class **variant enums and typed newtypes**
// (per ADR-0095 four-state-classes). They are leaf values that participate
// in hash derivation but are not themselves durable.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::error::ContractError;

// ─────────────────────────────────────────────────────────────────────────────
// ContractId
// ─────────────────────────────────────────────────────────────────────────────

/// Stable identity for an [`ArchitecturalContract`].
///
/// Identity is opaque to consumers but MUST round-trip through `Display`
/// and `FromStr`. Whitespace-only or empty input is rejected so call sites
/// cannot create meaningless ids.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractId(String);

impl ContractId {
    /// Construct a new `ContractId`. Returns an error for empty or
    /// whitespace-only input.
    pub fn new(raw: impl Into<String>) -> Result<Self, ContractError> {
        let s: String = raw.into();
        if s.trim().is_empty() {
            return Err(ContractError::InvalidId {
                reason: "empty or whitespace".into(),
            });
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for ContractId {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Revision
// ─────────────────────────────────────────────────────────────────────────────

/// Stable identity for a contract revision.
///
/// Free-form string (e.g. `"v1"`, `"2026-09-15.r3"`). Whitespace-only is
/// rejected; an empty string is rejected.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Revision(String);

impl Revision {
    /// Construct a new `Revision`. Empty / whitespace-only is rejected.
    pub fn new(raw: impl Into<String>) -> Result<Self, ContractError> {
        let s: String = raw.into();
        if s.trim().is_empty() {
            return Err(ContractError::InvalidRevision {
                reason: "empty or whitespace".into(),
            });
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Revision {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ComponentRef / EntityRef
// ─────────────────────────────────────────────────────────────────────────────

/// A free-form component / module reference used inside payload variants.
///
/// Typed as a newtype so we cannot mix it up with strings used elsewhere.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ComponentRef(String);

impl ComponentRef {
    /// Construct a new `ComponentRef`. Empty / whitespace-only is rejected.
    pub fn new(raw: impl Into<String>) -> Result<Self, ContractError> {
        let s: String = raw.into();
        if s.trim().is_empty() {
            return Err(ContractError::InvalidRef {
                kind: "ComponentRef".into(),
                reason: "empty or whitespace".into(),
            });
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ComponentRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for ComponentRef {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

/// A free-form entity reference used inside [`ContractPayload::UniqueOwner`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntityRef(String);

impl EntityRef {
    /// Construct a new `EntityRef`. Empty / whitespace-only is rejected.
    pub fn new(raw: impl Into<String>) -> Result<Self, ContractError> {
        let s: String = raw.into();
        if s.trim().is_empty() {
            return Err(ContractError::InvalidRef {
                kind: "EntityRef".into(),
                reason: "empty or whitespace".into(),
            });
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for EntityRef {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DecisionRef / SpecRef (typed provenance enums)
// ─────────────────────────────────────────────────────────────────────────────

/// Typed reference to the decision that established a contract.
///
/// At least one of the variants is required for [`ArchitecturalContract`]
/// to be constructible (REQ-A3S2-008, REQ-A3S2-010).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DecisionRef {
    /// Reference to a canonical Decision artifact in the engine's domain.
    Decision(String),
    /// Reference to an ADR document (e.g. ADR-0112).
    Adr(String),
    /// Reference to an external decision authority (human council,
    /// vendor approval). The `evidence_url` field is informational only
    /// and MUST NOT be used as runtime authority.
    ExternalDecision {
        /// Who decided.
        authority: String,
        /// Optional URL pointing to the decision record (NOT authority).
        evidence_url: Option<String>,
    },
}

impl DecisionRef {
    /// Canonical short tag for hashing.
    pub(super) fn canonical_tag(&self) -> std::borrow::Cow<'static, str> {
        use std::borrow::Cow;
        match self {
            DecisionRef::Decision(_) => Cow::Borrowed("decision"),
            DecisionRef::Adr(_) => Cow::Borrowed("adr"),
            DecisionRef::ExternalDecision { .. } => Cow::Borrowed("external_decision"),
        }
    }

    /// Borrow the inner reference string (for hashing).
    pub(crate) fn canonical_payload(&self) -> String {
        match self {
            DecisionRef::Decision(s) => s.clone(),
            DecisionRef::Adr(s) => s.clone(),
            DecisionRef::ExternalDecision {
                authority,
                evidence_url,
            } => match evidence_url {
                Some(u) => format!("{}|{}", authority, u),
                None => authority.clone(),
            },
        }
    }
}

impl DecisionRef {
    /// Canonical rendering of the reference (e.g. `ADR-0120`).
    ///
    /// Canonical here rather than in the CLI so `receipt`, the read surfaces and
    /// `why` render a decision identically (A3-S15).
    pub fn render(&self) -> String {
        match self {
            Self::Decision(s) => s.clone(),
            Self::Adr(s) => s.clone(),
            Self::ExternalDecision { authority, .. } => authority.clone(),
        }
    }
}

/// Typed reference to the spec / ADR that defined a contract's vocabulary.
///
/// At least one of the variants is required (REQ-A3S2-009, REQ-A3S2-010).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecRef {
    /// An arch-spec (e.g. arch-spec-032).
    ArchSpec(String),
    /// A standalone spec document (e.g. SPEC-SUPERSEDE-001).
    Spec(String),
    /// An ADR that doubles as the spec source.
    Adr(String),
}

impl SpecRef {
    /// Canonical rendering of the reference (e.g. `arch-spec-A3-S15`).
    pub fn render(&self) -> String {
        match self {
            Self::ArchSpec(v) | Self::Spec(v) | Self::Adr(v) => v.clone(),
        }
    }
}

impl SpecRef {
    /// Canonical short tag for hashing.
    pub(super) fn canonical_tag(&self) -> std::borrow::Cow<'static, str> {
        use std::borrow::Cow;
        match self {
            SpecRef::ArchSpec(_) => Cow::Borrowed("arch_spec"),
            SpecRef::Spec(_) => Cow::Borrowed("spec"),
            SpecRef::Adr(_) => Cow::Borrowed("adr"),
        }
    }

    /// Borrow the inner reference string (for hashing).
    pub(crate) fn canonical_payload(&self) -> &str {
        match self {
            SpecRef::ArchSpec(s) | SpecRef::Spec(s) | SpecRef::Adr(s) => s,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ContractKindRef (namespaced kind for extension seam)
// ─────────────────────────────────────────────────────────────────────────────

/// A namespaced kind identifier matching `lowercase_ascii.kind` shape.
///
/// Mirrors [`crate::semantic_kind::NamespacedKind`] but is local to this
/// module to avoid leaking the SemanticGraph enum into contract payloads.
/// Construction enforces the same lowercase ASCII rule.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractKindRef(String);

impl ContractKindRef {
    /// Construct a `ContractKindRef` from a raw input. Rejects empty input,
    /// inputs with more than two dot-separated segments, empty segments, or
    /// any non-lowercase-ASCII / non-digit / non-underscore characters.
    pub fn new(s: &str) -> Result<Self, ContractError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ContractError::InvalidKind {
                input: s.to_string(),
                reason: "empty".into(),
            });
        }
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() > 2 || parts.iter().any(|p| p.is_empty()) {
            return Err(ContractError::InvalidKind {
                input: s.to_string(),
                reason: "expected 'kind' or 'namespace.kind' (max 2 segments)".into(),
            });
        }
        for segment in &parts {
            for c in segment.chars() {
                if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
                    return Err(ContractError::InvalidKind {
                        input: s.to_string(),
                        reason: "expected lowercase ASCII, digits, '_'".into(),
                    });
                }
            }
        }
        // REQ-A3S2-016: reject `provider.<sdk>` namespace per ADR-0099.
        if parts.len() == 2 && parts[0] == "provider" {
            return Err(ContractError::ProviderNamespaceForbidden {
                kind: s.to_string(),
            });
        }
        Ok(Self(s.to_string()))
    }

    /// Borrow the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// (End of types.rs)
