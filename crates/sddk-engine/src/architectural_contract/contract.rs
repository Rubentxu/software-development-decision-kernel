// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/contract.rs — `ArchitecturalContract` OBJECT.
//
// # State class
//
// **OBJECT** (durable, revisioned, basis-addressed).
// Construction is intentionally private to external callers: the only
// canonical constructor is [`ArchitecturalContract::declare`], which
// computes the [`BasisHash`] from the declared content. This guarantees
// the hash always reflects the bytes that were declared (REQ-A3S2-001,
// REQ-A3S2-002).

use serde::{Deserialize, Serialize};

use crate::knowledge::{BasisHash, EventTime};

use super::error::ContractError;
use super::hashing::derive_contract_basis_hash;
use super::payload::{BoundaryKind, ContractKind, ContractPayload};
use super::types::{ComponentRef, ContractId, DecisionRef, EntityRef, Revision, SpecRef};

/// A durable, revisioned, basis-addressed architectural contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitecturalContract {
    id: ContractId,
    kind: ContractKind,
    payload: ContractPayload,
    basis_hash: BasisHash,
    decided_by: DecisionRef,
    specified_by: SpecRef,
    revision: Revision,
    declared_at: EventTime,
}

impl ArchitecturalContract {
    /// Declare a new contract. `basis_hash` is computed here from the
    /// declared content; it cannot be set by the caller.
    ///
    /// The kind/payload coupling is enforced by requiring callers to first
    /// build a typed payload variant via the helpers below (e.g.
    /// [`ArchitecturalContract::declare_single_authority`]). The raw
    /// [`ArchitecturalContract::declare`] constructor exists for extension
    /// payloads only.
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        id: ContractId,
        kind: ContractKind,
        payload: ContractPayload,
        decided_by: DecisionRef,
        specified_by: SpecRef,
        revision: Revision,
        declared_at: EventTime,
    ) -> Result<Self, ContractError> {
        // Enforce kind/payload tag alignment (REQ-A3S2-005). The two
        // exception paths are: (a) the `Extension` kind, whose payload
        // variant may legitimately be `ContractPayload::Extension`, and
        // (b) the case where the kind is `Extension(k)` and the payload
        // kind matches.
        match (&kind, &payload) {
            (ContractKind::SingleAuthority, ContractPayload::SingleAuthority(_)) => {}
            (ContractKind::UniqueOwner, ContractPayload::UniqueOwner(_)) => {}
            (ContractKind::ForbiddenDependency, ContractPayload::ForbiddenDependency { .. }) => {}
            (ContractKind::ProjectionOnly, ContractPayload::ProjectionOnly { .. }) => {}
            (ContractKind::BoundedCompatibility, ContractPayload::BoundedCompatibility { .. }) => {}
            (ContractKind::ProviderBoundary, ContractPayload::ProviderBoundary { .. }) => {}
            (ContractKind::Extension(k), ContractPayload::Extension { kind: pk, .. }) => {
                if k != pk {
                    return Err(ContractError::KindPayloadMismatch {
                        kind: k.as_str().to_string(),
                        payload_kind: pk.as_str().to_string(),
                    });
                }
            }
            (ContractKind::Extension(k), _) => {
                return Err(ContractError::KindPayloadMismatch {
                    kind: k.as_str().to_string(),
                    payload_kind: payload.canonical_tag().to_string(),
                });
            }
            (k, _) => {
                return Err(ContractError::KindPayloadMismatch {
                    kind: k.canonical_tag().to_string(),
                    payload_kind: payload.canonical_tag().to_string(),
                });
            }
        }

        let basis_hash = derive_contract_basis_hash(
            &id,
            &kind,
            &payload,
            &decided_by,
            &specified_by,
            &revision,
            declared_at,
        );
        Ok(Self {
            id,
            kind,
            payload,
            basis_hash,
            decided_by,
            specified_by,
            revision,
            declared_at,
        })
    }

    /// Helper: declare a [`ContractKind::SingleAuthority`] contract.
    pub fn declare_single_authority(
        id: ContractId,
        owner: ComponentRef,
        decided_by: DecisionRef,
        specified_by: SpecRef,
        revision: Revision,
        declared_at: EventTime,
    ) -> Result<Self, ContractError> {
        Self::declare(
            id,
            ContractKind::SingleAuthority,
            ContractPayload::SingleAuthority(owner),
            decided_by,
            specified_by,
            revision,
            declared_at,
        )
    }

    /// Helper: declare a [`ContractKind::UniqueOwner`] contract.
    pub fn declare_unique_owner(
        id: ContractId,
        owner: EntityRef,
        decided_by: DecisionRef,
        specified_by: SpecRef,
        revision: Revision,
        declared_at: EventTime,
    ) -> Result<Self, ContractError> {
        Self::declare(
            id,
            ContractKind::UniqueOwner,
            ContractPayload::UniqueOwner(owner),
            decided_by,
            specified_by,
            revision,
            declared_at,
        )
    }

    /// Helper: declare a [`ContractKind::ForbiddenDependency`] contract.
    #[allow(clippy::too_many_arguments)]
    pub fn declare_forbidden_dependency(
        id: ContractId,
        from: ComponentRef,
        to: ComponentRef,
        reason: String,
        decided_by: DecisionRef,
        specified_by: SpecRef,
        revision: Revision,
        declared_at: EventTime,
    ) -> Result<Self, ContractError> {
        Self::declare(
            id,
            ContractKind::ForbiddenDependency,
            ContractPayload::ForbiddenDependency { from, to, reason },
            decided_by,
            specified_by,
            revision,
            declared_at,
        )
    }

    /// Helper: declare a [`ContractKind::ProjectionOnly`] contract.
    #[allow(clippy::too_many_arguments)]
    pub fn declare_projection_only(
        id: ContractId,
        source_kind: String,
        decided_by: DecisionRef,
        specified_by: SpecRef,
        revision: Revision,
        declared_at: EventTime,
    ) -> Result<Self, ContractError> {
        Self::declare(
            id,
            ContractKind::ProjectionOnly,
            ContractPayload::ProjectionOnly { source_kind },
            decided_by,
            specified_by,
            revision,
            declared_at,
        )
    }

    /// Helper: declare a [`ContractKind::BoundedCompatibility`] contract.
    #[allow(clippy::too_many_arguments)]
    pub fn declare_bounded_compatibility(
        id: ContractId,
        deprecated_after: EventTime,
        replaced_by: Option<ContractId>,
        decided_by: DecisionRef,
        specified_by: SpecRef,
        revision: Revision,
        declared_at: EventTime,
    ) -> Result<Self, ContractError> {
        Self::declare(
            id,
            ContractKind::BoundedCompatibility,
            ContractPayload::BoundedCompatibility {
                deprecated_after,
                replaced_by,
            },
            decided_by,
            specified_by,
            revision,
            declared_at,
        )
    }

    /// Helper: declare a [`ContractKind::ProviderBoundary`] contract.
    #[allow(clippy::too_many_arguments)]
    pub fn declare_provider_boundary(
        id: ContractId,
        boundary: BoundaryKind,
        surface: String,
        decided_by: DecisionRef,
        specified_by: SpecRef,
        revision: Revision,
        declared_at: EventTime,
    ) -> Result<Self, ContractError> {
        Self::declare(
            id,
            ContractKind::ProviderBoundary,
            ContractPayload::ProviderBoundary { boundary, surface },
            decided_by,
            specified_by,
            revision,
            declared_at,
        )
    }

    // Accessors ─────────────────────────────────────────────────────────────

    /// The contract identity.
    pub fn id(&self) -> &ContractId {
        &self.id
    }

    /// The contract kind tag.
    pub fn kind(&self) -> &ContractKind {
        &self.kind
    }

    /// The contract payload.
    pub fn payload(&self) -> &ContractPayload {
        &self.payload
    }

    /// The basis hash (derived).
    pub fn basis_hash(&self) -> &BasisHash {
        &self.basis_hash
    }

    /// The decision reference.
    pub fn decided_by(&self) -> &DecisionRef {
        &self.decided_by
    }

    /// The spec / ADR reference.
    pub fn specified_by(&self) -> &SpecRef {
        &self.specified_by
    }

    /// The revision label.
    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    /// When the contract was declared.
    pub fn declared_at(&self) -> EventTime {
        self.declared_at
    }
}
