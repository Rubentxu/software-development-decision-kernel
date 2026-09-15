// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/hashing.rs — canonical basis-hash derivation.
//
// # State class
//
// Pure functions of the inputs. No IO, no clock reads. State-class
// **EPHEMERAL** (transient computation).

use sha2::{Digest, Sha256};

use crate::knowledge::{BasisHash, EventTime};

use super::payload::{ContractKind, ContractPayload};
use super::types::{ContractId, DecisionRef, Revision, SpecRef};

/// Compute the canonical basis hash for an [`ArchitecturalContract`].
///
/// Pure function of (id, kind, payload, decided_by, specified_by, revision,
/// declared_at). Domain prefix is `sddk.architectural_contract.v1\n` so
/// collisions with A3-S1 `KnowledgeAssertion::basis_hash` are impossible
/// (REQ-A3S2-002 crosswalk #3).
pub(super) fn derive_contract_basis_hash(
    id: &ContractId,
    kind: &ContractKind,
    payload: &ContractPayload,
    decided_by: &DecisionRef,
    specified_by: &SpecRef,
    revision: &Revision,
    declared_at: EventTime,
) -> BasisHash {
    let mut hasher = Sha256::new();
    hasher.update(b"sddk.architectural_contract.v1\n");
    // id
    hasher.update((id.as_str().len() as u64).to_le_bytes());
    hasher.update(id.as_str().as_bytes());
    // kind tag
    hash_cow(&mut hasher, kind.canonical_tag());
    // payload
    hash_payload(&mut hasher, payload);
    // decided_by
    hash_cow(&mut hasher, decided_by.canonical_tag());
    hash_str(&mut hasher, &decided_by.canonical_payload());
    // specified_by
    hash_cow(&mut hasher, specified_by.canonical_tag());
    hash_str(&mut hasher, specified_by.canonical_payload());
    // revision
    hash_str(&mut hasher, revision.as_str());
    // declared_at
    hasher.update(declared_at.0.to_le_bytes());
    BasisHash::from_digest(hasher)
}

pub(super) fn hash_payload(hasher: &mut Sha256, payload: &ContractPayload) {
    hash_cow(hasher, payload.canonical_tag());
    match payload {
        ContractPayload::SingleAuthority(c) => {
            hash_str(hasher, c.as_str());
        }
        ContractPayload::UniqueOwner(e) => {
            hash_str(hasher, e.as_str());
        }
        ContractPayload::ForbiddenDependency { from, to, reason } => {
            hash_str(hasher, from.as_str());
            hash_str(hasher, to.as_str());
            hash_str(hasher, reason);
        }
        ContractPayload::ProjectionOnly { source_kind } => {
            hash_str(hasher, source_kind);
        }
        ContractPayload::BoundedCompatibility {
            deprecated_after,
            replaced_by,
        } => {
            hasher.update(deprecated_after.0.to_le_bytes());
            match replaced_by {
                Some(cid) => hash_str(hasher, cid.as_str()),
                None => hasher.update(b"<none>"),
            }
        }
        ContractPayload::ProviderBoundary { boundary, surface } => {
            hash_cow(hasher, boundary.canonical_tag());
            hash_str(hasher, surface);
        }
        ContractPayload::Extension { kind, fields } => {
            // BTreeMap iteration is sorted by key, so insertion order does
            // not influence the hash (REQ-A3S2-007).
            hash_str(hasher, kind.as_str());
            hasher.update((fields.len() as u64).to_le_bytes());
            for (k, v) in fields {
                hasher.update((k.len() as u64).to_le_bytes());
                hasher.update(k.as_bytes());
                let form = v.canonical_form();
                hasher.update((form.len() as u64).to_le_bytes());
                hasher.update(form.as_bytes());
            }
        }
    }
}

pub(super) fn hash_str(hasher: &mut Sha256, s: &str) {
    hasher.update((s.len() as u64).to_le_bytes());
    hasher.update(s.as_bytes());
}

pub(super) fn hash_cow(hasher: &mut Sha256, s: std::borrow::Cow<'static, str>) {
    hash_str(hasher, s.as_ref());
}
