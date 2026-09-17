// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_declaration/validate.rs — A3-S10 fail-closed validation.
//
// Pure: no IO, no clock. Converts a parsed `DeclarationFile` into the AC1/AC2
// semantic objects, or fails with a specific error (AC-UAT-001).

use std::collections::BTreeSet;

use crate::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, EntityRef, Revision, SpecRef,
};
use crate::architecture_graph::{
    ArchitectureOverlayRelation, ArchitectureOverlayRelationKind, OverlayNodeRef, SoftwareUnit,
    SoftwareUnitRef, UnitKind,
};
use crate::knowledge::EventTime;

use super::types::{
    ContractDecl, DeclarationError, DeclarationFile, DeclaredArchitecture, RelationDecl, UnitDecl,
};

/// The nine relation kinds a declaration may state.
///
/// The semantic kinds (`decided_by`, `verified_by`, `contradicts_by`,
/// Convert declared observations into substrate observations (A4-0b).
///
/// Fail-closed on every field: an unknown relation kind, an endpoint naming no
/// declared unit, an unknown stance or origin, blank evidence. The declaration is
/// **input** — this validates shape and referential integrity, and decides nothing
/// about whether the observation is true.
///
/// The basis is the empty knowledge basis at the declaration's own origin: the
/// declaration supplies no knowledge basis, and pinning it here keeps observation
/// identity clock-stable (A4-0's invariant). `declared_at` does not enter identity,
/// so the pin is honest rather than a hidden zero.
fn convert_observations(
    decl: &DeclarationFile,
    unit_ids: &BTreeSet<String>,
) -> Result<Vec<crate::observation::SoftwareObservation>, DeclarationError> {
    use crate::evidence_ref::{EvidenceKind, EvidenceRef};
    use crate::knowledge::{EventTime, KnowledgeBasis};
    use crate::observation::{
        ObservationBasis, ObservationOrigin, ObservationStance, ObservationSubject,
        SoftwareEntityRef, SoftwareObservation, SoftwareRelation,
    };

    let basis_hash = KnowledgeBasis::empty(EventTime(0)).basis_hash().clone();
    // A4-3R2 — strict namespace. The CLI infers the typed endpoint
    // (Component / Entity / Unit) per observation: when a relation
    // endpoint string is referenced by a declared contract as
    // `SingleAuthority(component)` or `UniqueOwner(entity)`, the
    // matching observation endpoint is `Component` / `Entity`; in every
    // other case (and AC7 paradigm membership) it remains `Unit`.
    //
    // This avoids the pre-A4-3R2 cross-namespace equivalence that
    // matched `Unit("comp:dup")` against `SingleAuthority(ComponentRef("comp:dup"))`
    // via `as_str()`. Now both sides live in their own namespace and
    // `for_entity` finds the observation through typed equality.
    let mut component_endpoints: BTreeSet<String> = BTreeSet::new();
    let mut entity_endpoints: BTreeSet<String> = BTreeSet::new();
    for c in &decl.contracts {
        match c.kind.trim() {
            "single_authority" => {
                if let Some(ref component) = c.component {
                    component_endpoints.insert(component.clone());
                }
            }
            "unique_owner" => {
                if let Some(ref entity) = c.entity {
                    entity_endpoints.insert(entity.clone());
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::with_capacity(decl.observations.len());
    for o in &decl.observations {
        for endpoint in [&o.from, &o.to] {
            if !unit_ids.contains(endpoint) {
                return Err(DeclarationError::UnknownRelationEndpoint {
                    endpoint: endpoint.clone(),
                });
            }
        }
        let kind =
            core_relation_kind(&o.kind).ok_or_else(|| DeclarationError::UnknownRelationKind {
                from: o.from.clone(),
                to: o.to.clone(),
                kind: o.kind.clone(),
            })?;
        let stance = match o.stance.trim() {
            "affirms" => ObservationStance::Affirms,
            "denies" => ObservationStance::Denies,
            other => {
                return Err(DeclarationError::UnknownObservationField {
                    field: "stance".to_string(),
                    value: other.to_string(),
                });
            }
        };
        let origin = ObservationOrigin::ALL
            .into_iter()
            .find(|c| c.canonical_tag() == o.origin.trim())
            .ok_or_else(|| DeclarationError::UnknownObservationField {
                field: "origin".to_string(),
                value: o.origin.clone(),
            })?;
        let evidence_locator = o.evidence.trim();
        if evidence_locator.is_empty() {
            return Err(DeclarationError::UnknownObservationField {
                field: "evidence".to_string(),
                value: o.evidence.clone(),
            });
        }
        // A4-3R2 — typed endpoint constructor (Unit stays the
        // default; Component / Entity selected when the endpoint is
        // named by a SingleAuthority / UniqueOwner contract).
        let endpoint_ref = |raw: &str| -> SoftwareEntityRef {
            if component_endpoints.contains(raw) {
                SoftwareEntityRef::Component(
                    ComponentRef::new(raw.to_string()).expect("ascii component ref"),
                )
            } else if entity_endpoints.contains(raw) {
                SoftwareEntityRef::Entity(
                    EntityRef::new(raw.to_string()).expect("ascii entity ref"),
                )
            } else {
                SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new(
                    raw.to_string(),
                ))
            }
        };
        out.push(SoftwareObservation::declare(
            ObservationSubject::SoftwareRelation(SoftwareRelation::new(
                endpoint_ref(&o.from),
                kind,
                endpoint_ref(&o.to),
            )),
            stance,
            EvidenceRef::new(EvidenceKind::Adhoc, evidence_locator),
            origin,
            ObservationBasis::new(decl.revision.clone(), basis_hash.clone(), "declaration"),
            None,
            o.producer.clone(),
        ));
    }
    // Canonical order, so the declared set is deterministic regardless of file order.
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Resolve a core relation tag against the closed core vocabulary.
///
/// Derived from `CoreRelationKind::ALL` rather than hand-listed, so a new core kind
/// becomes declarable the moment it exists and the accepted set cannot go stale.
fn core_relation_kind(tag: &str) -> Option<crate::semantic_kind::CoreRelationKind> {
    crate::semantic_kind::CoreRelationKind::ALL
        .into_iter()
        .find(|k| k.domain_tag() == tag.trim())
}

/// `supersedes_by`, `architecture_claimed_by`) are produced by the system and
/// are deliberately NOT declarable.
pub const DECLARABLE_RELATION_KINDS: [&str; 9] = [
    "owns",
    "depends_on",
    "writes",
    "reads",
    "emits",
    "consumes",
    "projects",
    "derives_from",
    "implements",
];

fn relation_kind(kind: &str) -> Option<ArchitectureOverlayRelationKind> {
    match kind.trim() {
        "owns" => Some(ArchitectureOverlayRelationKind::Owns),
        "depends_on" => Some(ArchitectureOverlayRelationKind::DependsOn),
        "writes" => Some(ArchitectureOverlayRelationKind::Writes),
        "reads" => Some(ArchitectureOverlayRelationKind::Reads),
        "emits" => Some(ArchitectureOverlayRelationKind::Emits),
        "consumes" => Some(ArchitectureOverlayRelationKind::Consumes),
        "projects" => Some(ArchitectureOverlayRelationKind::Projects),
        "derives_from" => Some(ArchitectureOverlayRelationKind::DerivesFrom),
        "implements" => Some(ArchitectureOverlayRelationKind::Implements),
        _ => None,
    }
}

/// Convert one declared relation, checking its kind and both endpoints.
pub fn relation_from_decl(
    decl: &RelationDecl,
    unit_ids: &BTreeSet<String>,
) -> Result<ArchitectureOverlayRelation, DeclarationError> {
    let kind = relation_kind(&decl.kind).ok_or_else(|| DeclarationError::UnknownRelationKind {
        from: decl.from.clone(),
        to: decl.to.clone(),
        kind: decl.kind.clone(),
    })?;
    for endpoint in [&decl.from, &decl.to] {
        if !unit_ids.contains(endpoint) {
            return Err(DeclarationError::UnknownRelationEndpoint {
                endpoint: endpoint.clone(),
            });
        }
    }
    Ok(ArchitectureOverlayRelation {
        from: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new(decl.from.clone())),
        to: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new(decl.to.clone())),
        kind,
        evidence: Vec::new(),
    })
}

fn cid(decl: &ContractDecl) -> Result<ContractId, DeclarationError> {
    ContractId::new(decl.id.clone()).map_err(|e| DeclarationError::InvalidContract {
        id: decl.id.clone(),
        reason: e.to_string(),
    })
}

fn component(
    id: &str,
    value: &Option<String>,
    field: &'static str,
) -> Result<ComponentRef, DeclarationError> {
    let raw = value
        .as_ref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| DeclarationError::MissingField {
            id: id.to_string(),
            field,
        })?;
    ComponentRef::new(raw.clone()).map_err(|e| DeclarationError::InvalidContract {
        id: id.to_string(),
        reason: e.to_string(),
    })
}

fn entity(id: &str, value: &Option<String>) -> Result<EntityRef, DeclarationError> {
    let raw = value
        .as_ref()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| DeclarationError::MissingField {
            id: id.to_string(),
            field: "entity",
        })?;
    EntityRef::new(raw.clone()).map_err(|e| DeclarationError::InvalidContract {
        id: id.to_string(),
        reason: e.to_string(),
    })
}

fn required_string(
    id: &str,
    value: Option<&String>,
    field: &'static str,
) -> Result<String, DeclarationError> {
    value
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .ok_or_else(|| DeclarationError::MissingField {
            id: id.to_string(),
            field,
        })
}

fn decision(id: &str, raw: &str) -> Result<DecisionRef, DeclarationError> {
    if raw.trim().is_empty() {
        return Err(DeclarationError::MissingField {
            id: id.to_string(),
            field: "decided_by",
        });
    }
    Ok(DecisionRef::Decision(raw.to_string()))
}

fn spec(id: &str, raw: &str) -> Result<SpecRef, DeclarationError> {
    if raw.trim().is_empty() {
        return Err(DeclarationError::MissingField {
            id: id.to_string(),
            field: "specified_by",
        });
    }
    Ok(SpecRef::Spec(raw.to_string()))
}

fn revision(id: &str, raw: &str) -> Result<Revision, DeclarationError> {
    Revision::new(raw.to_string()).map_err(|e| DeclarationError::InvalidContract {
        id: id.to_string(),
        reason: e.to_string(),
    })
}

/// Convert one declaration into an AC1 contract.
pub fn contract_from_decl(decl: &ContractDecl) -> Result<ArchitecturalContract, DeclarationError> {
    let id = cid(decl)?;
    let declared_at = EventTime(decl.declared_at_ms.unwrap_or(0));
    let dec = decision(&decl.id, &decl.decided_by)?;
    let spc = spec(&decl.id, &decl.specified_by)?;
    let rev = revision(&decl.id, &decl.revision)?;

    let built = match decl.kind.as_str() {
        "single_authority" => {
            let owner = component(&decl.id, &decl.component, "component")?;
            ArchitecturalContract::declare_single_authority(id, owner, dec, spc, rev, declared_at)
        }
        "unique_owner" => {
            let owner = entity(&decl.id, &decl.entity)?;
            ArchitecturalContract::declare_unique_owner(id, owner, dec, spc, rev, declared_at)
        }
        "forbidden_dependency" => {
            let from = component(&decl.id, &decl.from, "from")?;
            let to = component(&decl.id, &decl.to, "to")?;
            let reason = decl.reason.clone().unwrap_or_default();
            ArchitecturalContract::declare_forbidden_dependency(
                id,
                from,
                to,
                reason,
                dec,
                spc,
                rev,
                declared_at,
            )
        }
        "projection_only" => {
            let source_kind = required_string(&decl.id, decl.source_kind.as_ref(), "source_kind")?;
            ArchitecturalContract::declare_projection_only(
                id,
                source_kind,
                dec,
                spc,
                rev,
                declared_at,
            )
        }
        "bounded_compatibility" => {
            let deprecated_after =
                decl.deprecated_after_ms
                    .ok_or_else(|| DeclarationError::MissingField {
                        id: decl.id.clone(),
                        field: "deprecated_after_ms",
                    })?;
            let replaced_by = match decl.replaced_by.as_ref() {
                Some(raw) if !raw.trim().is_empty() => {
                    Some(ContractId::new(raw.clone()).map_err(|e| {
                        DeclarationError::InvalidContract {
                            id: decl.id.clone(),
                            reason: e.to_string(),
                        }
                    })?)
                }
                _ => None,
            };
            ArchitecturalContract::declare_bounded_compatibility(
                id,
                EventTime(deprecated_after),
                replaced_by,
                dec,
                spc,
                rev,
                declared_at,
            )
        }
        other => {
            return Err(DeclarationError::UnknownKind {
                id: decl.id.clone(),
                kind: other.to_string(),
            });
        }
    };

    built.map_err(|e| DeclarationError::InvalidContract {
        id: decl.id.clone(),
        reason: e.to_string(),
    })
}

fn unit_kind(id: &str, raw: Option<&String>) -> Result<UnitKind, DeclarationError> {
    match raw.map(|s| s.trim()).unwrap_or("module") {
        "module" => Ok(UnitKind::Module),
        "crate" => Ok(UnitKind::Crate),
        "function" => Ok(UnitKind::Function),
        "endpoint" => Ok(UnitKind::Endpoint),
        "service" => Ok(UnitKind::Service),
        "schema" => Ok(UnitKind::Schema),
        "table" => Ok(UnitKind::Table),
        other => Err(DeclarationError::UnknownUnitKind {
            id: id.to_string(),
            kind: other.to_string(),
        }),
    }
}

/// Convert one declaration into an AC2 software unit.
pub fn unit_from_decl(decl: &UnitDecl) -> Result<SoftwareUnit, DeclarationError> {
    if decl.id.trim().is_empty() {
        return Err(DeclarationError::BlankId { collection: "unit" });
    }
    let kind = unit_kind(&decl.id, decl.kind.as_ref())?;
    let locator = decl
        .locator
        .clone()
        .filter(|l| !l.trim().is_empty())
        .unwrap_or_else(|| decl.id.clone());
    Ok(SoftwareUnit::new(
        SoftwareUnitRef::new(decl.id.clone()),
        kind,
        locator,
    ))
}

/// Validate a declaration file into AC1 contracts + AC2 units.
///
/// Fail-closed: any blank id, duplicate id, missing kind-specific field or
/// unrecognised kind is an error (AC-UAT-001).
pub fn validate(
    decl: &DeclarationFile,
    source_label: &str,
) -> Result<DeclaredArchitecture, DeclarationError> {
    if decl.revision.trim().is_empty() {
        return Err(DeclarationError::EmptyRevision);
    }

    // ── units ──────────────────────────────────────────────────────────────
    let mut unit_ids: BTreeSet<String> = BTreeSet::new();
    let mut units: Vec<SoftwareUnit> = Vec::with_capacity(decl.units.len());
    for u in &decl.units {
        if u.id.trim().is_empty() {
            return Err(DeclarationError::BlankId { collection: "unit" });
        }
        if !unit_ids.insert(u.id.clone()) {
            return Err(DeclarationError::DuplicateId {
                collection: "unit",
                id: u.id.clone(),
            });
        }
        units.push(unit_from_decl(u)?);
    }

    // ── relations (both endpoints must be declared units) ──────────────────
    let mut relations: Vec<ArchitectureOverlayRelation> = Vec::with_capacity(decl.relations.len());
    for r in &decl.relations {
        relations.push(relation_from_decl(r, &unit_ids)?);
    }

    // ── contracts ──────────────────────────────────────────────────────────
    let mut contract_ids: BTreeSet<String> = BTreeSet::new();
    let mut contracts: Vec<ArchitecturalContract> = Vec::with_capacity(decl.contracts.len());
    for c in &decl.contracts {
        if c.id.trim().is_empty() {
            return Err(DeclarationError::BlankId {
                collection: "contract",
            });
        }
        if !contract_ids.insert(c.id.clone()) {
            return Err(DeclarationError::DuplicateId {
                collection: "contract",
                id: c.id.clone(),
            });
        }
        contracts.push(contract_from_decl(c)?);
    }
    contracts.sort_by(|a, b| a.id().cmp(b.id()));

    // ── waivers ────────────────────────────────────────────────────────────
    let mut waivers: Vec<String> = decl
        .waivers
        .iter()
        .map(|w| w.trim().to_string())
        .filter(|w| !w.is_empty())
        .collect();
    waivers.sort();
    waivers.dedup();

    let knowledge_basis = decl
        .knowledge_basis
        .clone()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| format!("declaration:{source_label}"));

    // ── observations (A4-0b) ───────────────────────────────────────────────
    let observations = convert_observations(decl, &unit_ids)?;

    Ok(DeclaredArchitecture {
        revision: decl.revision.trim().to_string(),
        knowledge_basis,
        units,
        relations,
        contracts,
        observations,
        waivers,
    })
}
