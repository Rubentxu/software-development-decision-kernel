// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_graph/rebuild.rs — T-03 (A3-S3 / AC2)
//
// `rebuild(contracts, claims, units)` clears the overlay's projection and
// re-projects deterministically. Two rebuilds over the same inputs MUST
// yield identical canonical bytes (REQ-AC2-006 / AC-033-006).
//
// The rebuild path does NOT introduce randomness; it iterates the inputs
// in sorted order and emits a fixed sequence of (node, relation) calls
// against the canonical InMemorySemanticGraph.

use super::overlay::ArchitectureGraphOverlay;
use super::types::{SoftwareUnit, SoftwareUnitRef};
use crate::architectural_contract::{ArchitecturalContract, ArchitectureClaim};

/// Inputs to a rebuild. All collections are owned (or borrowed); rebuild
/// iterates them in sorted order for determinism.
pub struct RebuildInputs<'a> {
    pub contracts: &'a [ArchitecturalContract],
    pub claims: &'a [(ArchitectureClaim, SoftwareUnitRef)],
    pub units: &'a [SoftwareUnit],
}

/// Rebuild the overlay from canonical inputs. REQ-AC2-006/007/008.
pub fn rebuild(overlay: &mut ArchitectureGraphOverlay, inputs: &RebuildInputs<'_>) {
    overlay.clear();

    // 1) Sort units by locator and add.
    let mut sorted_units: Vec<&SoftwareUnit> = inputs.units.iter().collect();
    sorted_units.sort_by(|a, b| a.locator.cmp(&b.locator));
    for unit in sorted_units {
        overlay.add_unit(unit);
    }

    // 2) Sort contracts by id and emit DecidedBy metadata.
    let mut sorted_contracts: Vec<&ArchitecturalContract> = inputs.contracts.iter().collect();
    sorted_contracts.sort_by(|a, b| a.id().as_str().cmp(b.id().as_str()));
    for contract in sorted_contracts {
        let decision_ref = contract.decided_by();
        let spec_ref = contract.specified_by();
        overlay.add_contract_metadata(contract, decision_ref, spec_ref, &[]);
    }

    // 3) Sort claims by (contract_id, outcome_tag, evaluated_at) and emit.
    let mut sorted_claims: Vec<&(ArchitectureClaim, SoftwareUnitRef)> =
        inputs.claims.iter().collect();
    sorted_claims.sort_by(|a, b| {
        let ac = a.0.contract_id().as_str().cmp(b.0.contract_id().as_str());
        if ac != std::cmp::Ordering::Equal {
            return ac;
        }
        let ao =
            a.0.outcome()
                .canonical_tag()
                .cmp(b.0.outcome().canonical_tag());
        if ao != std::cmp::Ordering::Equal {
            return ao;
        }
        a.0.evaluated_at().0.cmp(&b.0.evaluated_at().0)
    });
    for (claim, unit_ref) in sorted_claims {
        let claim_id = overlay.add_claim(claim);
        overlay.attach_claim_to_unit(claim, &claim_id, unit_ref);
    }
}
