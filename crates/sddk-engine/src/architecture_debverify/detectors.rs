// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_debverify/detectors.rs — A3-S7 / AC5 global detectors.
//
// Five pure detectors over the whole contract set (and, for two of them, the
// semantic graph). No change basis is involved: this is the global challenge
// pass, not a change-scoped verification (AC-034-002).

use crate::architectural_contract::{
    ArchitecturalContract, ContractId, ContractKind, ContractPayload,
};
use crate::architecture_graph::ArchitectureGraphOverlay;
use crate::knowledge::EventTime;
use crate::semantic_graph::SemanticGraphProjection;
use std::collections::BTreeMap;

use super::types::{DebVerifyFinding, DebVerifyFindingKind};

/// Collect the graph node locators once (used by two detectors).
fn node_locators(overlay: &ArchitectureGraphOverlay) -> Vec<String> {
    overlay
        .projection()
        .nodes()
        .into_iter()
        .map(|n| n.locator)
        .collect()
}

/// Whether any node locator contains `subject`.
///
/// Documented heuristic (AC10 may replace it with typed edges): identifiers are
/// matched as substrings against node locators.
fn subject_in_graph(subject: &str, locators: &[String]) -> bool {
    locators.iter().any(|l| l.contains(subject))
}

fn sorted_ids(ids: &[ContractId]) -> Vec<ContractId> {
    let mut v: Vec<ContractId> = ids.to_vec();
    v.sort();
    v.dedup();
    v
}

fn finding(
    kind: DebVerifyFindingKind,
    subjects: Vec<String>,
    contract_ids: Vec<ContractId>,
    message: String,
) -> DebVerifyFinding {
    let mut subjects = subjects;
    subjects.sort();
    subjects.dedup();
    DebVerifyFinding {
        kind,
        severity: kind.severity(),
        subjects,
        contract_ids: sorted_ids(&contract_ids),
        message,
    }
}

/// The owner identifier of an authority/ownership contract, if any.
fn owner_subject(contract: &ArchitecturalContract) -> Option<(&'static str, String)> {
    match contract.payload() {
        ContractPayload::SingleAuthority(component) => {
            Some(("component", component.as_str().to_string()))
        }
        ContractPayload::UniqueOwner(entity) => Some(("entity", entity.as_str().to_string())),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. ShadowAuthority (REQ-AC5-008)
// ─────────────────────────────────────────────────────────────────────────────

/// Two or more contracts claiming the same authority (AC-UAT-009).
pub fn shadow_authority(contracts: &[ArchitecturalContract]) -> Vec<DebVerifyFinding> {
    // component ref -> contract ids
    let mut by_component: BTreeMap<String, Vec<ContractId>> = BTreeMap::new();
    let mut by_entity: BTreeMap<String, Vec<ContractId>> = BTreeMap::new();
    for c in contracts {
        match c.payload() {
            ContractPayload::SingleAuthority(component) => {
                by_component
                    .entry(component.as_str().to_string())
                    .or_default()
                    .push(c.id().clone());
            }
            ContractPayload::UniqueOwner(entity) => {
                by_entity
                    .entry(entity.as_str().to_string())
                    .or_default()
                    .push(c.id().clone());
            }
            _ => {}
        }
    }

    let mut out = Vec::new();
    for (subject, ids) in by_component {
        if ids.len() > 1 {
            let n = ids.len();
            out.push(finding(
                DebVerifyFindingKind::ShadowAuthority,
                vec![subject.clone()],
                ids,
                format!("{n} contracts claim SingleAuthority over component `{subject}`"),
            ));
        }
    }
    for (subject, ids) in by_entity {
        if ids.len() > 1 {
            let n = ids.len();
            out.push(finding(
                DebVerifyFindingKind::ShadowAuthority,
                vec![subject.clone()],
                ids,
                format!("{n} contracts claim UniqueOwner over entity `{subject}`"),
            ));
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. MissingOwner (REQ-AC5-009)
// ─────────────────────────────────────────────────────────────────────────────

/// Authority/ownership contracts naming an owner absent from the graph.
pub fn missing_owner(
    contracts: &[ArchitecturalContract],
    overlay: &ArchitectureGraphOverlay,
) -> Vec<DebVerifyFinding> {
    let locators = node_locators(overlay);
    let mut out = Vec::new();
    for c in contracts {
        if let Some((noun, subject)) = owner_subject(c)
            && !subject_in_graph(&subject, &locators)
        {
            out.push(finding(
                DebVerifyFindingKind::MissingOwner,
                vec![subject.clone()],
                vec![c.id().clone()],
                format!(
                    "contract `{}` names {noun} `{subject}` which is absent from the graph",
                    c.id()
                ),
            ));
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. AuthorityBypass (REQ-AC5-010)
// ─────────────────────────────────────────────────────────────────────────────

/// Forbidden dependency contracts whose edge exists in the graph.
pub fn authority_bypass(
    contracts: &[ArchitecturalContract],
    overlay: &ArchitectureGraphOverlay,
) -> Vec<DebVerifyFinding> {
    let projection = overlay.projection();
    let nodes = projection.nodes();
    let locator_of: BTreeMap<String, String> = nodes
        .iter()
        .map(|n| (n.id.as_str().to_string(), n.locator.clone()))
        .collect();

    let mut out = Vec::new();
    for c in contracts {
        if let ContractPayload::ForbiddenDependency { from, to, .. } = c.payload() {
            let f = from.as_str();
            let t = to.as_str();
            let mut joined = false;
            for rel in projection.relations() {
                let (Some(fl), Some(tl)) = (
                    locator_of.get(rel.from.as_str()),
                    locator_of.get(rel.to.as_str()),
                ) else {
                    continue;
                };
                let forward = fl.contains(f) && tl.contains(t);
                let reverse = fl.contains(t) && tl.contains(f);
                if forward || reverse {
                    joined = true;
                    break;
                }
            }
            if joined {
                out.push(finding(
                    DebVerifyFindingKind::AuthorityBypass,
                    vec![format!("{f} -> {t}")],
                    vec![c.id().clone()],
                    format!("forbidden dependency `{f} -> {t}` is present in the graph"),
                ));
            }
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. StaleCompatibility (REQ-AC5-011)
// ─────────────────────────────────────────────────────────────────────────────

/// Compatibility windows that elapsed without a replacement.
pub fn stale_compatibility(
    contracts: &[ArchitecturalContract],
    now: EventTime,
) -> Vec<DebVerifyFinding> {
    let mut out = Vec::new();
    for c in contracts {
        if let ContractPayload::BoundedCompatibility {
            deprecated_after,
            replaced_by,
        } = c.payload()
            && now.0 >= deprecated_after.0
            && replaced_by.is_none()
        {
            out.push(finding(
                DebVerifyFindingKind::StaleCompatibility,
                vec![c.id().as_str().to_string()],
                vec![c.id().clone()],
                format!(
                    "compatibility window for `{}` elapsed at {} with no replacement",
                    c.id(),
                    deprecated_after.0
                ),
            ));
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Contradiction (REQ-AC5-012)
// ─────────────────────────────────────────────────────────────────────────────

/// The same subject declared authority-owned and projection-only.
pub fn contradiction(contracts: &[ArchitecturalContract]) -> Vec<DebVerifyFinding> {
    // subject -> (authority contract ids, projection contract ids)
    let mut authority: BTreeMap<String, Vec<ContractId>> = BTreeMap::new();
    let mut projected: BTreeMap<String, Vec<ContractId>> = BTreeMap::new();

    for c in contracts {
        match c.payload() {
            ContractPayload::SingleAuthority(component) => {
                authority
                    .entry(component.as_str().to_string())
                    .or_default()
                    .push(c.id().clone());
            }
            ContractPayload::UniqueOwner(entity) => {
                authority
                    .entry(entity.as_str().to_string())
                    .or_default()
                    .push(c.id().clone());
            }
            ContractPayload::ProjectionOnly { source_kind } => {
                projected
                    .entry(source_kind.clone())
                    .or_default()
                    .push(c.id().clone());
            }
            _ => {}
        }
    }

    let mut out = Vec::new();
    for (subject, auth_ids) in &authority {
        if let Some(proj_ids) = projected.get(subject) {
            let mut ids = auth_ids.clone();
            ids.extend(proj_ids.iter().cloned());
            out.push(finding(
                DebVerifyFindingKind::Contradiction,
                vec![subject.clone()],
                ids,
                format!("subject `{subject}` is declared both authority-owned and projection-only"),
            ));
        }
    }
    out
}

/// Whether a contract kind participates in the named-dimension detectors.
/// (Kept for documentation; not used in control flow.)
pub fn kind_is_auditable(kind: &ContractKind) -> bool {
    matches!(
        kind,
        ContractKind::SingleAuthority
            | ContractKind::UniqueOwner
            | ContractKind::ForbiddenDependency
            | ContractKind::ProjectionOnly
            | ContractKind::BoundedCompatibility
    )
}
