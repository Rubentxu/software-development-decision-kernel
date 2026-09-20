//! Reactive Verify pipeline / AC9 changed-unit contract loop
//! (arch-spec-025 + arch-spec-037, J5).
//!
//! Pipeline: eventos host → coalesced `WorkspaceChangeSet` → KMT
//! (unidades afectadas) → contratos afectados → Verify delta-scoped
//! → ArchitectureConformanceDelta → ContextDelta útil.
//!
//! Los eventos ephemeral no se coalescean (RHB-003), los bursts se
//! coalescean con `turn_done` como frontera (RHB-004), y el verify
//! es delta-scoped, no whole-repo (RHB-005). La tensión de
//! alignment normal es contextual, nunca INTERRUPT (RHB-008,
//! AC-037-006).
//!
//! Upstream: `docs/architecture/specs/arch-spec-025-reactive-host-event-bridge.md`,
//! `docs/architecture/specs/arch-spec-037-reactive-conformance-loop.md`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Host-native event normalized at the adapter (RHB-001).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostEvent {
    pub kind: HostEventKind,
    pub namespace: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostEventKind {
    /// Routine read/search/list/housekeeping — EPHEMERAL (RHB-002).
    Read,
    /// Source mutation — MAY be MATERIAL.
    Edit,
    /// Relevant build/test result — MAY be MATERIAL.
    BuildResult,
    /// Turn completion — normal coalescing boundary (RHB-004).
    TurnDone,
}

/// Materiality classification (RHB-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Materiality {
    Ephemeral,
    Material,
}

impl HostEvent {
    #[must_use]
    pub fn materiality(&self) -> Materiality {
        match self.kind {
            HostEventKind::Read => Materiality::Ephemeral,
            HostEventKind::Edit | HostEventKind::BuildResult | HostEventKind::TurnDone => {
                Materiality::Material
            }
        }
    }
}

/// Bounded coalesced change set (RHB-004).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceChangeSet {
    /// Deduplicated namespaces touched by material events.
    pub namespaces: BTreeSet<String>,
    /// Turn that closed the change set (boundary).
    pub turn: u64,
}

/// Coalescer: accumulates material events until `turn_done` closes
/// the set. Ephemeral events never enter (RHB-003). Bounded: one
/// namespace counted once (BTreeSet).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeSetCoalescer {
    pending: BTreeSet<String>,
    current_turn: u64,
}

impl ChangeSetCoalescer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a normalized event. Returns the closed change set on
    /// `turn_done`, `None` otherwise.
    pub fn feed(&mut self, ev: HostEvent) -> Option<WorkspaceChangeSet> {
        if let HostEventKind::TurnDone = ev.kind {
            let cs = WorkspaceChangeSet {
                namespaces: std::mem::take(&mut self.pending),
                turn: self.current_turn,
            };
            self.current_turn += 1;
            return Some(cs);
        }
        if ev.materiality() == Materiality::Material {
            self.pending.insert(ev.namespace.clone());
        }
        None
    }
}

/// Knowledge–Machine Topology projection: which units each
/// namespace maps to (AC-037-004).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KmtIndex {
    /// namespace → affected units.
    pub units_by_namespace: BTreeMap<std::string::String, std::string::String>,
}

use std::collections::BTreeMap;

/// An architectural contract in force with its guarded units.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitecturalContractRef {
    pub contract_id: String,
    /// Units (KMT) whose changes affect this contract.
    pub guards_units: BTreeSet<String>,
}

/// Delta-scoped verification outcome for one contract.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractVerification {
    pub contract_id: String,
    pub outcome: ContractOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractOutcome {
    /// Contract satisfied on the delta slice.
    Conformant,
    /// Contract violated: surfaced as ATTENTION, not automatically
    /// INTERRUPT (RHB-008).
    Violated,
    /// Not enough evidence — provider deepening is OPTIONAL
    /// (RHB-006): absent providers stay a gap, never a failure of
    /// Base integration.
    EvidenceGap,
}

/// Attention level of the reactive output (RHB-008).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttentionLevel {
    Silent,
    Contextual,
    Attention,
    Interrupt,
}

/// The reactive conformance delta emitted per change set
/// (AC-037-005: generic, not host-specific).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureConformanceDelta {
    pub change_set_turn: u64,
    pub verifications: Vec<ContractVerification>,
    pub attention: AttentionLevel,
}

/// Reactive Verify pipeline (delta-scoped, RHB-005). Given the
/// change set, KMT and contracts in force, verify only the affected
/// slice.
#[must_use]
pub fn run_reactive_verify(
    cs: &WorkspaceChangeSet,
    kmt: &KmtIndex,
    contracts: &[ArchitecturalContractRef],
    // Resolution: (contract_id, unit) -> outcome. Absent entries
    // are EvidenceGap (RHB-006).
    resolve: impl Fn(&str, &str) -> Option<ContractOutcome>,
) -> ArchitectureConformanceDelta {
    // Affected units via KMT (AC-037-004).
    let affected: BTreeSet<&str> = cs
        .namespaces
        .iter()
        .filter_map(|ns| kmt.units_by_namespace.get(ns.as_str()))
        .map(String::as_str)
        .collect();

    let mut verifications = Vec::new();
    let mut any_violated = false;
    let mut any_evaluated = false;
    for c in contracts {
        if !c.guards_units.iter().any(|u| affected.contains(u.as_str())) {
            continue; // contract untouched by this delta — not verified, not emitted
        }
        let mut outcome = ContractOutcome::EvidenceGap;
        for u in &c.guards_units {
            if affected.contains(u.as_str())
                && let Some(o) = resolve(&c.contract_id, u)
            {
                any_evaluated = true;
                if o == ContractOutcome::Violated {
                    outcome = ContractOutcome::Violated;
                    any_violated = true;
                    break;
                }
                outcome = o;
            }
        }
        let _ = any_evaluated;
        verifications.push(ContractVerification {
            contract_id: c.contract_id.clone(),
            outcome,
        });
    }

    // Attention policy (RHB-008, AC-037-006): normal tension is
    // contextual/advisory; violation → ATTENTION; only explicit
    // governance policy would raise to INTERRUPT (not modeled here
    // — the loop cannot produce INTERRUPT on its own).
    let attention = if verifications.is_empty() {
        AttentionLevel::Silent
    } else if any_violated {
        AttentionLevel::Attention
    } else {
        AttentionLevel::Contextual
    };

    ArchitectureConformanceDelta {
        change_set_turn: cs.turn,
        verifications,
        attention,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kmt() -> KmtIndex {
        KmtIndex {
            units_by_namespace: BTreeMap::from([
                ("src/engine/ledger.rs".into(), "unit:ledger".into()),
                ("src/engine/graph.rs".into(), "unit:graph".into()),
                ("src/cli/main.rs".into(), "unit:cli".into()),
            ]),
        }
    }

    fn contracts() -> Vec<ArchitecturalContractRef> {
        vec![
            ArchitecturalContractRef {
                contract_id: "c-ledger-append-only".into(),
                guards_units: BTreeSet::from(["unit:ledger".into()]),
            },
            ArchitecturalContractRef {
                contract_id: "c-cli-no-engine-internals".into(),
                guards_units: BTreeSet::from(["unit:cli".into()]),
            },
        ]
    }

    /// RHB-003/004: reads stay ephemeral and never enter the change
    /// set; edits coalesce; turn_done closes the bounded set.
    #[test]
    fn rhb003_004_coalescing_and_ephemeral() {
        let mut c = ChangeSetCoalescer::new();
        assert!(
            c.feed(HostEvent {
                kind: HostEventKind::Read,
                namespace: "src/engine/ledger.rs".into()
            })
            .is_none()
        );
        assert!(
            c.feed(HostEvent {
                kind: HostEventKind::Edit,
                namespace: "src/engine/ledger.rs".into()
            })
            .is_none()
        );
        assert!(
            c.feed(HostEvent {
                kind: HostEventKind::Edit,
                namespace: "src/engine/ledger.rs".into()
            })
            .is_none()
        ); // burst coalesces
        let cs = c
            .feed(HostEvent {
                kind: HostEventKind::TurnDone,
                namespace: "host".into(),
            })
            .unwrap();
        assert_eq!(
            cs.namespaces,
            BTreeSet::from(["src/engine/ledger.rs".to_string()])
        );
        assert_eq!(cs.turn, 0);
        // Empty turn (only reads) yields an empty, still-bounded set.
        c.feed(HostEvent {
            kind: HostEventKind::Read,
            namespace: "src/cli/main.rs".into(),
        });
        let cs2 = c
            .feed(HostEvent {
                kind: HostEventKind::TurnDone,
                namespace: "host".into(),
            })
            .unwrap();
        assert!(cs2.namespaces.is_empty());
        assert_eq!(cs2.turn, 1);
    }

    /// RHB-005 / AC9: only contracts guarding affected units are
    /// verified — no whole-repo unconditional scan.
    #[test]
    fn ac9_changed_unit_contract_loop() {
        let mut c = ChangeSetCoalescer::new();
        c.feed(HostEvent {
            kind: HostEventKind::Edit,
            namespace: "src/engine/ledger.rs".into(),
        });
        let cs = c
            .feed(HostEvent {
                kind: HostEventKind::TurnDone,
                namespace: "host".into(),
            })
            .unwrap();
        let delta = run_reactive_verify(&cs, &kmt(), &contracts(), |cid, u| {
            assert_eq!(cid, "c-ledger-append-only");
            assert_eq!(u, "unit:ledger");
            Some(ContractOutcome::Conformant)
        });
        // CLI contract untouched by this delta: not emitted.
        assert_eq!(delta.verifications.len(), 1);
        assert_eq!(delta.verifications[0].contract_id, "c-ledger-append-only");
        assert_eq!(delta.verifications[0].outcome, ContractOutcome::Conformant);
        assert_eq!(delta.attention, AttentionLevel::Contextual);
    }

    /// RHB-008: violation surfaces as ATTENTION, never automatic
    /// INTERRUPT (AC-037-006).
    #[test]
    fn rhb008_violation_attention_not_interrupt() {
        let cs = WorkspaceChangeSet {
            namespaces: BTreeSet::from(["src/engine/ledger.rs".into()]),
            turn: 3,
        };
        let delta = run_reactive_verify(&cs, &kmt(), &contracts(), |_, _| {
            Some(ContractOutcome::Violated)
        });
        assert_eq!(delta.attention, AttentionLevel::Attention);
        assert!(
            delta
                .verifications
                .iter()
                .any(|v| v.outcome == ContractOutcome::Violated)
        );
        // Structural: Interrupt is never produced by this pipeline.
        assert_ne!(delta.attention, AttentionLevel::Interrupt);
    }

    /// RHB-006: absent provider resolution stays EvidenceGap, Base
    /// integration does not break.
    #[test]
    fn rhb006_provider_absent_is_gap_not_failure() {
        let cs = WorkspaceChangeSet {
            namespaces: BTreeSet::from(["src/engine/ledger.rs".into()]),
            turn: 0,
        };
        let delta = run_reactive_verify(&cs, &kmt(), &contracts(), |_, _| None);
        assert_eq!(delta.verifications[0].outcome, ContractOutcome::EvidenceGap);
        assert_eq!(delta.attention, AttentionLevel::Contextual);
    }

    /// AC-037-002 + RHB-003: no change set ever materializes from
    /// pure reads — no canonical fact is created.
    #[test]
    fn ac037002_reads_never_become_facts() {
        let mut c = ChangeSetCoalescer::new();
        for _ in 0..5 {
            c.feed(HostEvent {
                kind: HostEventKind::Read,
                namespace: "src/engine/graph.rs".into(),
            });
        }
        let cs = c
            .feed(HostEvent {
                kind: HostEventKind::TurnDone,
                namespace: "host".into(),
            })
            .unwrap();
        let delta = run_reactive_verify(&cs, &kmt(), &contracts(), |_, _| {
            Some(ContractOutcome::Conformant)
        });
        assert!(delta.verifications.is_empty());
        assert_eq!(delta.attention, AttentionLevel::Silent);
    }
}
