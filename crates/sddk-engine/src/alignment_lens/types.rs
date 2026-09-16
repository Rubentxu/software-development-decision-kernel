// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/types.rs — A4-4b: core types for the generic lens kernel.
//
// These are the "things a lens and the kernel exchange". The kernel
// shape is:
//
// ```text
// LensInput  ──→ [AlignmentLens] ──→ LensContribution
// ```
//
// Every type in this file is **EPHEMERAL** (ADR-0095). It is descriptive
// geometry, not durable identity. The kernel that consumes them is pure
// and deterministic.

use serde::Serialize;
use std::collections::BTreeSet;

use crate::architecture_graph::SoftwareUnitRef;
use crate::intent_universal_concern::types::IntentId;
use crate::intent_universal_concern::{ApplicableConcern, UniversalConcern};
use crate::knowledge::BasisHash;
use crate::observation::ObservationSet;

// ─── Lens identity ──────────────────────────────────────────────────────────

/// Stable identity of one [`AlignmentLens`](super::AlignmentLens).
///
/// Constructed by content (the lens's stable, declared identifier); never
/// derived from wall clock, registration order, address, or rendered text.
///
/// `LensId` itself is `&'static str` because the kernel never generates
/// one. Lenses declare their own; the kernel stores them. `BTreeSet<LensId>`
/// iteration is therefore in lexicographic order — deterministic.
///
/// Note: `Deserialize` is intentionally NOT derived because
/// `&'static str` cannot be safely deserialized from arbitrary input.
/// The kernel never needs JSON-shape deserialization; it constructs
/// `LensId` from the lens author's declared identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct LensId(pub &'static str);

impl LensId {
    /// Construct a `LensId` from a `&'static str` literal. Caller is
    /// responsible for keeping the string stable for the lens's lifetime.
    pub const fn new(s: &'static str) -> Self {
        Self(s)
    }

    /// Borrow the underlying identifier.
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for LensId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// Semantic version of one [`AlignmentLens`](super::AlignmentLens) implementation.
///
/// Bumping the version is the lens author's way of saying "the algorithm
/// changed in a way that may produce different `LensContribution`s on the
/// same input". The kernel re-exports this version verbatim into each
/// contribution's identity so two contributions from differently-versioned
/// lenses cannot collide.
///
/// `Deserialize` is intentionally NOT derived (the kernel never needs
/// JSON deserialization; `LensVersion` is constructed via
/// [`LensVersion::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct LensVersion {
    /// Major bump: the algorithm or its inputs/outputs changed shape.
    pub major: u32,
    /// Minor bump: behaviour changed but the contribution shape is
    /// stable (e.g. a fresh heuristic inside the lens).
    pub minor: u32,
}

impl LensVersion {
    /// Construct a version.
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Canonical tag (e.g. `"1.0"`). Used for identity.
    pub fn canonical_tag(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

impl std::fmt::Display for LensVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.canonical_tag())
    }
}

// ─── Insufficient gap: typed replacement for the gap: String ─────────────────

/// Typed reason a [`LensContribution`](super::LensContribution)'s
/// `EvidenceResolution` is `Insufficient`.
///
/// `observation::EvidenceResolution::Insufficient` carries a
/// `gap: String` for wire compat. The lens kernel does **not** let that
/// string reach identity: identity is derived from this enum (via its
/// canonical tag) and the `gap: String` is only set when serializing.
///
/// Why this matters: the A4-4b pin *no-message-text-in-identity*
/// (spec §3 X15, pin 10). Without this enum the kernel would either
/// (a) invent message text by string concatenation (stringly), or
/// (b) leave `gap` empty (losing provenance). This enum gives both:
/// typed provenance in identity; faithful serialization on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InsufficientGap {
    /// No observation covers the relation at all.
    NoObservation,
    /// Some observation covers the relation but its stance is empty
    /// (no affirm, no deny) — the relation is unobservable in practice.
    ObservationsWithoutStance,
    /// Provenance is missing (no `basis` or `evidence` attached to the
    /// covering observation). Distinct from "no observation" because
    /// the observation exists but cannot be reasoned about.
    MissingProvenance,
    /// The lens author explicitly declared "insufficient but I have a
    /// typed reason that is none of the above". Used by future built-in
    /// lenses (A4-4M).
    LensDeclaredGap,
}

impl InsufficientGap {
    /// Canonical short tag. Used for identity.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::NoObservation => "no_observation",
            Self::ObservationsWithoutStance => "observations_without_stance",
            Self::MissingProvenance => "missing_provenance",
            Self::LensDeclaredGap => "lens_declared_gap",
        }
    }

    /// Wire form for `observation::EvidenceResolution::Insufficient.gap`.
    /// This is a serialized message; it MUST NOT enter identity.
    pub fn wire_message(self) -> String {
        // Stable, deterministic, derived from the enum so the message
        // does not carry clock, locale, or random bytes. The message is
        // for human-readable serialization only.
        format!("alignment_lens_insufficient:{}", self.canonical_tag())
    }
}

// ─── LensDescriptor ─────────────────────────────────────────────────────────

/// Static description of one [`AlignmentLens`](super::AlignmentLens) implementation.
///
/// `LensDescriptor` declares what a lens **says about itself**. It is the
/// surface used by the registry's `for_concern(...)` lookup. A lens MAY
/// support more than one `UniversalConcern`; a registry lookup returns
/// every lens whose `supported_concerns` contains the asked-for concern.
///
/// **NO** priority, weight, score, confidence, or capability rank. The
/// registry's dedup is `(LensId, LensVersion)` only. Two lenses sharing
/// a `LensId` but disagreeing on `supported_concerns` is a registered
/// `LensError::InconsistentSupportedConcerns` (see `error.rs`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LensDescriptor {
    /// Stable identifier of the lens.
    pub id: LensId,
    /// Semantic version.
    pub version: LensVersion,
    /// The closed set of `UniversalConcern` values this lens declares
    /// to evaluate. Lookup keys off this set only.
    pub supported_concerns: BTreeSet<UniversalConcern>,
}

impl LensDescriptor {
    /// Construct a descriptor.
    pub fn new(
        id: LensId,
        version: LensVersion,
        supported_concerns: BTreeSet<UniversalConcern>,
    ) -> Self {
        Self {
            id,
            version,
            supported_concerns,
        }
    }

    /// True iff this descriptor declares support for `c`.
    pub fn supports(&self, c: UniversalConcern) -> bool {
        self.supported_concerns.contains(&c)
    }
}

// ─── LensInput ──────────────────────────────────────────────────────────────

/// Typed input handed to an [`AlignmentLens`](super::AlignmentLens).
///
/// The kernel constructs `LensInput` from canonical types only. No
/// strings are parsed to discover intent/contracts/evidence. Each field
/// maps 1:1 to a stable, content-addressed type in the substrate.
///
/// Identity (for `LensContributionId`) is derived from `(intent_basis,
/// concern, observation_set_canonical_tag)` — see `id.rs`. The kernel
/// guarantees the same input bytes (semantically, not byte-for-byte)
/// produce identical contribution identities.
///
/// **Applicability is a precondition.** The lens kernel never sees a
/// `LensInput` whose `applicable` is `NotApplicable(...)`. Callers
/// must use [`LensInput::try_new`], which returns
/// [`LensError::NotApplicableInput`](super::LensError::NotApplicableInput)
/// in that case. Mixing layers is the shape A4-4aR corrected.
///
/// # A4-4bR: lenses consume real targets, never synthetic relations.
///
/// Lenses query the `ObservationSet` for **real** observation subjects
/// — `ObservationSubject::SoftwareRelation`, `::Unit`,
/// `::Contract`, `::Knowledge` — and emit contributions on those real
/// targets. The kernel MUST NOT, and lens authors MUST NOT, derive a
/// synthetic `RelationId` from `(intent_id, concern, unit_ref)` as a
/// convenience to satisfy the relation-only API of
/// `EvidenceResolution`. Subject-general evidence resolution lives in
/// A4-4bR; pre-A4-4bR relation-only API MUST be used only when the
/// lens genuinely observes a `SoftwareRelation`.
///
/// `LensInput` carries the `ObservationSet` as-is; lenses iterate
/// `observations` (via
/// [`SoftwareObservation::subject`](crate::observation::SoftwareObservation::subject))
/// and pick their targets directly. The fixture `Freshness` lens in
/// `tests/alignment_lens_fixture.rs` is the falsification: it works
/// against `ObservationSubject::Unit(foo)` without fabricating any
/// `SoftwareRelation`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LensInput {
    /// The applicability answer being evaluated. Lens authors read
    /// `concern()` from this to know what to evaluate.
    pub applicable: ApplicableConcern,

    /// The unit the applicability answer is for.
    pub unit_ref: SoftwareUnitRef,

    /// Declared project intent identity. Carried for traceability, not
    /// for re-parsing: lenses read `intent_id` for identity only.
    pub intent_id: IntentId,

    /// Knowledge basis the lens is being asked to evaluate against.
    /// Lenses MUST NOT read wall clock; `basis` is content-stable.
    pub basis: BasisHash,

    /// The canonical observation set. Lenses iterate this directly to
    /// pick real observation subjects (relations, units, contracts,
    /// knowledge). They MUST NOT derive synthetic `RelationId`s from
    /// `(intent_id, concern, unit_ref)` — see the module-level note
    /// above.
    pub observations: ObservationSet,
}

impl LensInput {
    /// Construct an input, refusing `NotApplicable` answers (A4-4aR pin).
    pub fn try_new(
        applicable: ApplicableConcern,
        unit_ref: SoftwareUnitRef,
        intent_id: IntentId,
        basis: BasisHash,
        observations: ObservationSet,
    ) -> Result<Self, super::error::LensError> {
        if !applicable.is_applicable() {
            return Err(super::error::LensError::NotApplicableInput {
                concern: applicable.concern(),
                reason: match &applicable {
                    ApplicableConcern::NotApplicable(_, r) => *r,
                    _ => unreachable!("guarded above"),
                },
            });
        }
        Ok(Self {
            applicable,
            unit_ref,
            intent_id,
            basis,
            observations,
        })
    }

    /// The `UniversalConcern` this input is about.
    pub fn concern(&self) -> UniversalConcern {
        self.applicable.concern()
    }
}
