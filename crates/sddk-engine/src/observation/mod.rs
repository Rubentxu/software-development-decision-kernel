// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// observation/mod.rs — A4-0: the Evidence/Observation/Provenance substrate.
//
// Cycle: `p-63676b11dc0ef88f/a4-0-verification-provenance-foundation`
// Spec:  `docs/architecture/specs/arch-spec-042-evidence-observation-provenance.md`
// ADR:   `docs/architecture/adrs/ADR-0122-EVIDENCE-OBSERVES-SOFTWARE.md`
//
// A producer observes software and records the observation with its provenance.
// SDDK decides afterwards how to interpret it.
//
// ```text
// Evidence ≠ truth      Provider ≠ authority      Observation ≠ decision
// ```
//
// This module **evaluates nothing**: it runs no probe, consults no domain, and
// imports neither Alignment nor Authority. It stores observations, identifies them
// deterministically, and preserves contradictions. Any producer — a deterministic
// local analyzer, CogniCode's static observations, Chronos's runtime traces — can
// emit observations without becoming authority.
//
// Contradictions are first-class and are never collapsed by "latest wins".

pub mod project;
pub mod resolution;
pub mod types;

#[cfg(test)]
mod tests;

pub use project::{
    OBSERVATION_NODE_KIND, OBSERVES_RELATION_KIND, RELATION_NODE_KIND, project_into,
};
pub use resolution::{EvidenceResolution, resolve_relation};
pub use types::{
    ObservationBasis, ObservationId, ObservationOrigin, ObservationSet, ObservationStance,
    ObservationSubject, RelationId, SoftwareEntityRef, SoftwareObservation, SoftwareRelation,
};
