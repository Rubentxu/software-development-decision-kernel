// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_declaration/types.rs — A3-S10 declaration shapes.
//
// The declarative architecture file is INPUT, never authority: `validate`
// converts it into AC1 `ArchitecturalContract`s and AC2 `SoftwareUnit`s, which
// are the semantic objects everything else consumes.

use serde::Deserialize;

use crate::architectural_contract::ArchitecturalContract;
use crate::architecture_graph::SoftwareUnit;

/// One declared software unit (AC2 substrate).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct UnitDecl {
    /// Unit identity (the AC2 locator used for node identity).
    pub id: String,
    /// Unit kind; defaults to `module`.
    #[serde(default)]
    pub kind: Option<String>,
    /// Human locator (path); defaults to `id`.
    #[serde(default)]
    pub locator: Option<String>,
}

/// One declared architectural contract.
///
/// A tagged-by-`kind` union: only the fields relevant to `kind` are required,
/// and `validate` fails closed when they are missing (AC-UAT-001).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ContractDecl {
    /// Contract identity.
    pub id: String,
    /// One of: `single_authority`, `unique_owner`, `forbidden_dependency`,
    /// `projection_only`, `bounded_compatibility`.
    pub kind: String,

    // ── kind-specific payload fields ───────────────────────────────────────
    /// `single_authority`: the owning component.
    #[serde(default)]
    pub component: Option<String>,
    /// `unique_owner`: the owning entity.
    #[serde(default)]
    pub entity: Option<String>,
    /// `forbidden_dependency`: source component.
    #[serde(default)]
    pub from: Option<String>,
    /// `forbidden_dependency`: target component.
    #[serde(default)]
    pub to: Option<String>,
    /// `forbidden_dependency`: informational rationale.
    #[serde(default)]
    pub reason: Option<String>,
    /// `projection_only`: the rebuilt source-kind tag.
    #[serde(default)]
    pub source_kind: Option<String>,
    /// `bounded_compatibility`: epoch-ms at which the window closes.
    #[serde(default)]
    pub deprecated_after_ms: Option<i64>,
    /// `bounded_compatibility`: optional replacement contract id.
    #[serde(default)]
    pub replaced_by: Option<String>,

    // ── common metadata ────────────────────────────────────────────────────
    /// Decision reference.
    pub decided_by: String,
    /// Specification reference.
    pub specified_by: String,
    /// Contract revision.
    pub revision: String,
    /// Declared-at epoch-ms.
    #[serde(default)]
    pub declared_at_ms: Option<i64>,
}

/// The parsed declaration file (engine-side shape; the CLI parses YAML into it).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct DeclarationFile {
    /// Exact named revision the declaration describes (AC-041-001).
    pub revision: String,
    /// Human-readable knowledge basis; defaults to `declaration:<label>`.
    #[serde(default)]
    pub knowledge_basis: Option<String>,
    /// Declared software units.
    #[serde(default)]
    pub units: Vec<UnitDecl>,
    /// Declared contracts.
    #[serde(default)]
    pub contracts: Vec<ContractDecl>,
    /// Governed waivers, carried verbatim.
    #[serde(default)]
    pub waivers: Vec<String>,
}

/// The validated declaration: AC1 contracts + AC2 units, ready to project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredArchitecture {
    /// Exact named revision.
    pub revision: String,
    /// Knowledge basis.
    pub knowledge_basis: String,
    /// AC2 units (declaration order).
    pub units: Vec<SoftwareUnit>,
    /// AC1 contracts (sorted by id).
    pub contracts: Vec<ArchitecturalContract>,
    /// Waivers (sorted, deduplicated).
    pub waivers: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors (fail-closed, REQ-A3S10-003)
// ─────────────────────────────────────────────────────────────────────────────

/// A declaration could not be validated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclarationError {
    /// `revision` is missing or blank.
    EmptyRevision,
    /// A contract/unit id is missing or blank.
    BlankId {
        /// Which collection (`unit` | `contract`).
        collection: &'static str,
    },
    /// Two contracts (or two units) share an id.
    DuplicateId {
        /// Which collection (`unit` | `contract`).
        collection: &'static str,
        /// The duplicated id.
        id: String,
    },
    /// A kind-specific required field is absent.
    MissingField {
        /// The offending contract/unit id.
        id: String,
        /// The missing field name.
        field: &'static str,
    },
    /// The contract `kind` is not one of the five declarable kinds.
    UnknownKind {
        /// The offending contract id.
        id: String,
        /// The unrecognised kind string.
        kind: String,
    },
    /// The unit `kind` is not a recognised AC2 unit kind.
    UnknownUnitKind {
        /// The offending unit id.
        id: String,
        /// The unrecognised kind string.
        kind: String,
    },
    /// AC1 rejected the declared payload (kind/payload mismatch or invalid id).
    InvalidContract {
        /// The offending contract id.
        id: String,
        /// The underlying message.
        reason: String,
    },
}

impl std::fmt::Display for DeclarationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyRevision => write!(f, "declaration: `revision` is required and non-empty"),
            Self::BlankId { collection } => {
                write!(f, "declaration: a {collection} id is blank")
            }
            Self::DuplicateId { collection, id } => {
                write!(f, "declaration: duplicate {collection} id `{id}`")
            }
            Self::MissingField { id, field } => {
                write!(f, "declaration: contract `{id}` is missing field `{field}`")
            }
            Self::UnknownKind { id, kind } => {
                write!(f, "declaration: contract `{id}` has unknown kind `{kind}`")
            }
            Self::UnknownUnitKind { id, kind } => {
                write!(f, "declaration: unit `{id}` has unknown kind `{kind}`")
            }
            Self::InvalidContract { id, reason } => {
                write!(f, "declaration: contract `{id}` is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for DeclarationError {}
