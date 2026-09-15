// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/payload.rs — `ContractKind`, `BoundaryKind`,
// `ContractPayload`, `ContractExtensionValue`.
//
// # State class
//
// These types are state-class **variant enums and typed newtypes** that
// compose the payload half of an [`ArchitecturalContract`](super::ArchitecturalContract).
// They participate in canonical hashing via [`super::hashing::hash_payload`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::knowledge::EventTime;

use super::types::{ComponentRef, ContractId, ContractKindRef, EntityRef};

// ─────────────────────────────────────────────────────────────────────────────
// ContractKind (closed enum)
// ─────────────────────────────────────────────────────────────────────────────

/// The closed taxonomy of architectural contract kinds.
///
/// Adding a new variant is a deliberate breaking change (REQ-A3S2-005).
/// Extensions go through [`ContractKind::Extension`] using a
/// [`ContractKindRef`] (custom seam, REQ-A3S2-006).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractKind {
    /// One component owns a single capability.
    SingleAuthority,
    /// One entity uniquely owns a resource.
    UniqueOwner,
    /// A specific dependency edge is forbidden.
    ForbiddenDependency,
    /// The entity is rebuilt (projection), not stored.
    ProjectionOnly,
    /// The compatibility window has an upper bound.
    BoundedCompatibility,
    /// No inward provider SDK types allowed (ADR-0099).
    ProviderBoundary,
    /// Custom extension kind, identified by a [`ContractKindRef`].
    Extension(ContractKindRef),
}

impl ContractKind {
    /// Canonical short tag for hashing. Allocates a `String` only for the
    /// `Extension` variant (whose kind is a runtime namespaced identifier);
    /// all closed variants return `Cow::Borrowed` zero-allocation strings.
    pub(super) fn canonical_tag(&self) -> std::borrow::Cow<'static, str> {
        use std::borrow::Cow;
        match self {
            ContractKind::SingleAuthority => Cow::Borrowed("single_authority"),
            ContractKind::UniqueOwner => Cow::Borrowed("unique_owner"),
            ContractKind::ForbiddenDependency => Cow::Borrowed("forbidden_dependency"),
            ContractKind::ProjectionOnly => Cow::Borrowed("projection_only"),
            ContractKind::BoundedCompatibility => Cow::Borrowed("bounded_compatibility"),
            ContractKind::ProviderBoundary => Cow::Borrowed("provider_boundary"),
            ContractKind::Extension(k) => Cow::Owned(k.as_str().to_string()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BoundaryKind (closed enum)
// ─────────────────────────────────────────────────────────────────────────────

/// Closed taxonomy of provider-boundary surfaces. Per ADR-0099 these bound
/// where provider SDK types may NOT appear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BoundaryKind {
    /// Inbound calls (from provider into our types).
    Inbound,
    /// Outbound calls (we call into provider SDK types).
    Outbound,
    /// Bidirectional (neither allowed).
    Bidirectional,
}

impl BoundaryKind {
    /// Canonical short tag for hashing.
    pub(super) fn canonical_tag(self) -> std::borrow::Cow<'static, str> {
        use std::borrow::Cow;
        match self {
            BoundaryKind::Inbound => Cow::Borrowed("inbound"),
            BoundaryKind::Outbound => Cow::Borrowed("outbound"),
            BoundaryKind::Bidirectional => Cow::Borrowed("bidirectional"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ContractPayload (sum type coupled to ContractKind)
// ─────────────────────────────────────────────────────────────────────────────

/// The closed sum type of payload shapes, one variant per contract kind.
///
/// Construction is decoupled from [`ContractKind`] by the typed
/// `ArchitecturalContract::declare_*` family of constructors so the
/// kind/payload coupling is enforced at compile time (REQ-A3S2-005).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractPayload {
    /// One [`ComponentRef`] owns the capability.
    SingleAuthority(ComponentRef),
    /// One [`EntityRef`] owns the resource.
    UniqueOwner(EntityRef),
    /// A `from → to` dependency edge is forbidden.
    ForbiddenDependency {
        /// Source side of the forbidden edge.
        from: ComponentRef,
        /// Target side of the forbidden edge.
        to: ComponentRef,
        /// Human-readable rationale (informational, NOT authority).
        reason: String,
    },
    /// The entity is rebuilt, not stored. The `source_kind` is the
    /// `NamespacedKind` or `CoreNodeKind` tag identifying what is rebuilt.
    ProjectionOnly {
        /// Source kind tag (lowercase snake_case).
        source_kind: String,
    },
    /// Compatibility window ends at `deprecated_after`. The `replaced_by`
    /// contract (if any) is the migration target.
    BoundedCompatibility {
        /// Time at which the contract becomes stale.
        deprecated_after: EventTime,
        /// Optional replacement contract (none ⇒ `Stale` once `now` crosses).
        replaced_by: Option<ContractId>,
    },
    /// No inward provider SDK types allowed on the given surface.
    ProviderBoundary {
        /// Which side(s) are forbidden.
        boundary: BoundaryKind,
        /// Human-readable surface description (e.g. `"engine -> provider"`).
        surface: String,
    },
    /// Custom extension payload. Uses [`BTreeMap`] so hash derivation is
    /// insertion-order independent (REQ-A3S2-007).
    Extension {
        /// The kind tag identifying this extension.
        kind: ContractKindRef,
        /// Field map (sorted by `BTreeMap` for hashing).
        fields: BTreeMap<String, ContractExtensionValue>,
    },
}

/// Allowed value types inside an extension payload.
///
/// `BTreeMap` and `Vec` keep insertion-order independent hashing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractExtensionValue {
    /// UTF-8 string.
    String(String),
    /// 64-bit signed integer.
    Integer(i64),
    /// Boolean flag.
    Boolean(bool),
    /// Nested ordered map.
    Object(BTreeMap<String, ContractExtensionValue>),
    /// Ordered list.
    Array(Vec<ContractExtensionValue>),
}

impl ContractExtensionValue {
    /// Canonical short tag for hashing.
    #[allow(dead_code)] // used by hashing.rs (kept for future direct callers)
    pub(super) fn canonical_tag(&self) -> &'static str {
        match self {
            ContractExtensionValue::String(_) => "string",
            ContractExtensionValue::Integer(_) => "integer",
            ContractExtensionValue::Boolean(_) => "boolean",
            ContractExtensionValue::Object(_) => "object",
            ContractExtensionValue::Array(_) => "array",
        }
    }

    /// Borrow the canonical string form for hashing (recursive over nested
    /// containers, deterministic because of `BTreeMap` ordering).
    pub(super) fn canonical_form(&self) -> String {
        match self {
            ContractExtensionValue::String(s) => format!("s:{}", s),
            ContractExtensionValue::Integer(i) => format!("i:{}", i),
            ContractExtensionValue::Boolean(b) => format!("b:{}", b),
            ContractExtensionValue::Object(m) => {
                let mut parts = Vec::with_capacity(m.len());
                for (k, v) in m {
                    parts.push(format!("{}={}", k, v.canonical_form()));
                }
                format!("o:{{{}}}", parts.join(","))
            }
            ContractExtensionValue::Array(arr) => {
                let parts: Vec<String> = arr.iter().map(|v| v.canonical_form()).collect();
                format!("a:[{}]", parts.join(","))
            }
        }
    }
}

impl ContractPayload {
    /// Canonical short tag for hashing. Returns `Cow<'static, str>` —
    /// the closed variants borrow, only `Extension` allocates.
    pub(super) fn canonical_tag(&self) -> std::borrow::Cow<'static, str> {
        use std::borrow::Cow;
        match self {
            ContractPayload::SingleAuthority(_) => Cow::Borrowed("single_authority"),
            ContractPayload::UniqueOwner(_) => Cow::Borrowed("unique_owner"),
            ContractPayload::ForbiddenDependency { .. } => Cow::Borrowed("forbidden_dependency"),
            ContractPayload::ProjectionOnly { .. } => Cow::Borrowed("projection_only"),
            ContractPayload::BoundedCompatibility { .. } => Cow::Borrowed("bounded_compatibility"),
            ContractPayload::ProviderBoundary { .. } => Cow::Borrowed("provider_boundary"),
            ContractPayload::Extension { kind, .. } => Cow::Owned(kind.as_str().to_string()),
        }
    }
}
