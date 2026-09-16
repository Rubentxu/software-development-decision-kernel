//! Substrate types (REQ-A4S0-001..009).

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::architectural_contract::{ComponentRef, ContractId, EntityRef};
use crate::architecture_graph::SoftwareUnitRef;
use crate::evidence_ref::EvidenceRef;
use crate::knowledge::{BasisHash, KmtStatus, KnowledgeId};
use crate::semantic_kind::CoreRelationKind;

/// An endpoint of a software relation.
///
/// Reuses the refs that already exist rather than minting parallel ones: those
/// three already name the things a relation can join.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum SoftwareEntityRef {
    /// A software unit.
    Unit(SoftwareUnitRef),
    /// An architectural component.
    Component(ComponentRef),
    /// A domain entity.
    Entity(EntityRef),
}

impl SoftwareEntityRef {
    /// Canonical tag used in identity derivation. Namespaced so `unit:x` can never
    /// collide with `component:x`.
    pub fn canonical_tag(&self) -> String {
        match self {
            SoftwareEntityRef::Unit(u) => format!("unit:{}", u.as_str()),
            SoftwareEntityRef::Component(c) => format!("component:{}", c.as_str()),
            SoftwareEntityRef::Entity(e) => format!("entity:{}", e.as_str()),
        }
    }
}

/// A software relation: an ADT, **never** a rendered string (REQ-A4S0-001).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SoftwareRelation {
    /// Source endpoint.
    pub from: SoftwareEntityRef,
    /// Relation kind — reuses `CoreRelationKind`; no second taxonomy (REQ-A4S0-005).
    pub kind: CoreRelationKind,
    /// Target endpoint.
    pub to: SoftwareEntityRef,
}

impl SoftwareRelation {
    /// Construct a relation.
    pub fn new(from: SoftwareEntityRef, kind: CoreRelationKind, to: SoftwareEntityRef) -> Self {
        Self { from, kind, to }
    }

    /// The relation's deterministic identity.
    pub fn id(&self) -> RelationId {
        RelationId::derive(
            &self.from.canonical_tag(),
            self.kind.domain_tag(),
            &self.to.canonical_tag(),
        )
    }

    /// A human rendering. **Never** used for identity (REQ-A4S0-004).
    pub fn render(&self) -> String {
        format!(
            "{} --{}--> {}",
            self.from.canonical_tag(),
            self.kind.domain_tag(),
            self.to.canonical_tag()
        )
    }
}

/// Deterministic identity of a [`SoftwareRelation`].
///
/// `sha256(domain | from | kind | to)`. Timestamps, messages, severities and
/// rendered text are excluded (REQ-A4S0-002, REQ-A4S0-004).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct RelationId(pub String);

impl RelationId {
    /// Domain prefix.
    pub const DOMAIN: &'static str = "sddk.software_relation.id.v1|";

    /// Derive from canonical endpoint/kind tags.
    pub fn derive(from_tag: &str, kind_tag: &str, to_tag: &str) -> Self {
        let mut h = Sha256::new();
        h.update(Self::DOMAIN.as_bytes());
        h.update(from_tag.as_bytes());
        h.update(b"|");
        h.update(kind_tag.as_bytes());
        h.update(b"|");
        h.update(to_tag.as_bytes());
        Self(format!("{:064x}", h.finalize()))
    }

    /// Borrow the raw hex.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Who and how produced an observation (REQ-A4S0-006).
///
/// A different axis from "how strongly is this held": `AdvisoryProvenance` answers
/// the latter. Collapsing them would force a producer to declare a storage location
/// it does not have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationOrigin {
    /// A deterministic analyzer inside SDDK. The strongest origin.
    DeterministicLocal,
    /// A static code-intelligence provider (e.g. CogniCode).
    StaticProvider,
    /// A runtime/trace provider (e.g. Chronos).
    RuntimeProvider,
    /// A human declared it.
    HumanDeclared,
    /// Derived by a documented rule from other knowledge.
    Inferred,
}

impl ObservationOrigin {
    /// Every variant, in canonical order.
    pub const ALL: [ObservationOrigin; 5] = [
        ObservationOrigin::DeterministicLocal,
        ObservationOrigin::StaticProvider,
        ObservationOrigin::RuntimeProvider,
        ObservationOrigin::HumanDeclared,
        ObservationOrigin::Inferred,
    ];

    /// Stable short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            ObservationOrigin::DeterministicLocal => "deterministic_local",
            ObservationOrigin::StaticProvider => "static_provider",
            ObservationOrigin::RuntimeProvider => "runtime_provider",
            ObservationOrigin::HumanDeclared => "human_declared",
            ObservationOrigin::Inferred => "inferred",
        }
    }
}

/// Does the observation affirm the relation or deny it (REQ-A4S0-010)?
///
/// This is what makes contradiction representable without overwriting: an affirm
/// and a deny are two observations, not two versions of one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStance {
    /// The producer observed the relation as holding.
    Affirms,
    /// The producer observed the relation as not holding.
    Denies,
}

impl ObservationStance {
    /// Stable short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            ObservationStance::Affirms => "affirms",
            ObservationStance::Denies => "denies",
        }
    }
}

/// What the observation was taken against (REQ-A4S0-008).
///
/// `revision` and `knowledge_basis` are clock-stable (A3 measured that
/// `semantic_graph_digest` is not), which is why identity may include them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObservationBasis {
    /// Exact revision the observation was taken at.
    pub revision: String,
    /// Knowledge basis hash the observation was taken against.
    pub knowledge_basis: BasisHash,
    /// Digest of the producer's own input, so a re-observation can be compared.
    pub input_digest: String,
}

impl ObservationBasis {
    /// Construct a basis.
    pub fn new(
        revision: impl Into<String>,
        knowledge_basis: BasisHash,
        input_digest: impl Into<String>,
    ) -> Self {
        Self {
            revision: revision.into(),
            knowledge_basis,
            input_digest: input_digest.into(),
        }
    }

    /// Canonical identity contribution.
    fn canonical_tag(&self) -> String {
        format!(
            "{}\u{1f}{}\u{1f}{}",
            self.revision,
            self.knowledge_basis.to_hex(),
            self.input_digest
        )
    }
}

/// What the observation is about.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "subject")]
pub enum ObservationSubject {
    /// A software relation — the case `why architecture` needs.
    SoftwareRelation(SoftwareRelation),
    /// A software unit on its own.
    Unit(SoftwareUnitRef),
    /// An architectural contract.
    Contract(ContractId),
    /// A knowledge assertion.
    Knowledge(KnowledgeId),
}

impl ObservationSubject {
    /// Canonical identity contribution.
    pub fn canonical_tag(&self) -> String {
        match self {
            ObservationSubject::SoftwareRelation(r) => format!("relation:{}", r.id().as_str()),
            ObservationSubject::Unit(u) => format!("unit:{}", u.as_str()),
            ObservationSubject::Contract(c) => format!("contract:{}", c.as_str()),
            ObservationSubject::Knowledge(k) => format!("knowledge:{}", k.as_str()),
        }
    }
}

/// Deterministic identity of an observation.
///
/// `sha256(domain | subject | evidence | origin | stance | basis)`. No timestamp,
/// no message, no severity, no rendered text (REQ-A4S0-004).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ObservationId(pub String);

impl ObservationId {
    /// Domain prefix.
    pub const DOMAIN: &'static str = "sddk.software_observation.id.v1|";

    /// Derive the identity.
    pub fn derive(
        subject_tag: &str,
        evidence_tag: &str,
        origin: ObservationOrigin,
        stance: ObservationStance,
        basis_tag: &str,
    ) -> Self {
        let mut h = Sha256::new();
        h.update(Self::DOMAIN.as_bytes());
        h.update(subject_tag.as_bytes());
        h.update(b"|");
        h.update(evidence_tag.as_bytes());
        h.update(b"|");
        h.update(origin.canonical_tag().as_bytes());
        h.update(b"|");
        h.update(stance.canonical_tag().as_bytes());
        h.update(b"|");
        h.update(basis_tag.as_bytes());
        Self(format!("{:064x}", h.finalize()))
    }

    /// Borrow the raw hex.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One observation, with its provenance (REQ-A4S0-007, 008, 009).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SoftwareObservation {
    /// Deterministic identity.
    pub id: ObservationId,
    /// What was observed.
    pub subject: ObservationSubject,
    /// Whether the relation was observed to hold.
    pub stance: ObservationStance,
    /// The evidence supporting this observation — the **universal** reference.
    pub evidence: EvidenceRef,
    /// Who and how.
    pub origin: ObservationOrigin,
    /// What it was taken against.
    pub basis: ObservationBasis,
    /// Freshness, **only when an expected basis was supplied**.
    ///
    /// `None` means *not evaluated*, never "fresh".
    pub freshness: Option<KmtStatus>,
    /// The producer's identifier (a tool name, an analyzer id, a human id).
    pub producer: String,
}

impl SoftwareObservation {
    /// Declare an observation, computing its identity.
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        subject: ObservationSubject,
        stance: ObservationStance,
        evidence: EvidenceRef,
        origin: ObservationOrigin,
        basis: ObservationBasis,
        freshness: Option<KmtStatus>,
        producer: impl Into<String>,
    ) -> Self {
        let evidence_tag = format!("{}:{}", evidence.kind.domain_tag(), evidence.locator);
        let id = ObservationId::derive(
            &subject.canonical_tag(),
            &evidence_tag,
            origin,
            stance,
            &basis.canonical_tag(),
        );
        Self {
            id,
            subject,
            stance,
            evidence,
            origin,
            basis,
            freshness,
            producer: producer.into(),
        }
    }
}

/// An insert-only collection of observations (REQ-A4S0-010, 011).
///
/// There is **no** `replace` and no `latest`. Inserting never removes or supersedes
/// anything.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ObservationSet {
    observations: Vec<SoftwareObservation>,
}

impl ObservationSet {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an observation. Duplicates (by identity) collapse; nothing else does.
    pub fn insert(&mut self, observation: SoftwareObservation) {
        if !self.observations.iter().any(|o| o.id == observation.id) {
            self.observations.push(observation);
            self.normalize();
        }
    }

    /// Sort by id so the set has one canonical order.
    pub fn normalize(&mut self) {
        self.observations.sort_by(|a, b| a.id.cmp(&b.id));
    }

    /// All observations, in canonical order.
    pub fn observations(&self) -> &[SoftwareObservation] {
        &self.observations
    }

    /// How many observations are held.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// True iff nothing is held.
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Every observation whose subject is the given relation.
    pub fn for_relation(&self, relation: &RelationId) -> Vec<&SoftwareObservation> {
        self.observations
            .iter()
            .filter(|o| match &o.subject {
                ObservationSubject::SoftwareRelation(r) => r.id() == *relation,
                _ => false,
            })
            .collect()
    }

    /// Every observation whose subject matches the given target ref
    /// (A4-4bR: subject-general lookup).
    ///
    /// The target kind is part of the match: a Unit observation never
    /// matches a Relation target and vice versa. The lookup is
    /// deterministic; observations are returned in canonical order
    /// (the set already sorts on insert).
    pub fn for_subject(
        &self,
        target: &crate::observation::ObservationTargetRef,
    ) -> Vec<&SoftwareObservation> {
        use crate::observation::ObservationTargetRef as T;
        self.observations
            .iter()
            .filter(|o| match (target, &o.subject) {
                (T::Relation(r), ObservationSubject::SoftwareRelation(obs_r)) => obs_r.id() == *r,
                (T::Unit(u), ObservationSubject::Unit(obs_u)) => obs_u == u,
                (T::Contract(c), ObservationSubject::Contract(obs_c)) => obs_c == c,
                (T::Knowledge(k), ObservationSubject::Knowledge(obs_k)) => obs_k == k,
                _ => false,
            })
            .collect()
    }

    /// Every observation of a relation that has the given entity as an endpoint.
    ///
    /// This is what lets `why architecture` connect an observation to the contract
    /// subject it constrains.
    pub fn for_entity(&self, entity: &SoftwareEntityRef) -> Vec<&SoftwareObservation> {
        let tag = entity.canonical_tag();
        self.observations
            .iter()
            .filter(|o| match &o.subject {
                ObservationSubject::SoftwareRelation(r) => {
                    r.from.canonical_tag() == tag || r.to.canonical_tag() == tag
                }
                ObservationSubject::Unit(u) => {
                    u.as_str() == entity.canonical_tag().trim_start_matches("unit:")
                }
                _ => false,
            })
            .collect()
    }
}
