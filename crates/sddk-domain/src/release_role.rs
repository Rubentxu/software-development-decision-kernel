//! What a release target is responsible for, and where its responsibility
//! stops.
//!
//! # The assumption this module exists to remove
//!
//! The release flow was written as one shape: build, tag, publish a stable
//! release. Every step belongs to the same actor, in the same place, at the same
//! time. That shape is not a law, it is a habit — and it is wrong for a large
//! and perfectly ordinary arrangement: **one repository produces candidate
//! material, and something else certifies and promotes it.** The producer is not
//! a smaller version of the publisher. It is a different responsibility, and
//! asking it to do the publisher's job means either that it must, or that the
//! distinction is invisible.
//!
//! So a [`ReleaseRole`] is a **declaration of responsibility**, and the thing it
//! most needs to be able to say is where it stops.
//!
//! # Two questions, and the reason they are not one
//!
//! A role and a channel look like they compete. They do not, and mixing them is
//! the defect this module is built to make impossible:
//!
//! - **Is this step legal in the channel lattice?** [`can_promote`] answers that,
//!   and it answers it for everyone. It is the existing authority and this
//!   module does not restate it.
//! - **Is this actor allowed to go that far?** [`ReleaseRole::may_reach`] answers
//!   that, by comparing the requested channel against the role's ceiling.
//!
//! Both must hold. Neither replaces the other, and the two disagree in both
//! directions, which is what makes them worth separating:
//!
//! - a `FullPublisher` still cannot jump `Dev → Stable` in one step, because the
//!   lattice says no and the role does not get a vote;
//! - a `CandidateProducer` cannot reach `Stable` **even after every gate passes**,
//!   because passing the gates is what a *promoter* needs and this is not one.
//!
//! A design that collapsed them would report one of those as allowed.
//!
//! [`can_promote`]: crate::channel::can_promote

use crate::channel::{ReleaseChannel, can_promote, promotion_target};
use crate::release_ref::{CandidateSequence, ReleaseRef, SourceRevision};
use crate::version_authority::{ProductVersion, ReleaseTarget};
use serde::{Deserialize, Serialize};
use std::fmt;

/// How many promotions separate `from` from `to`, or `None` when `to` is not
/// reachable from `from` by promoting at all.
///
/// ## Why this walks the lattice instead of comparing two channels
///
/// **The derived `Ord` on `ReleaseChannel` runs the opposite way to the
/// promotion order.** The enum is declared `Stable, Candidate, Edge, Dev` —
/// which is the order the docs read in, from the end of the chain to its
/// start — so `Ord` puts `Stable` *first*, and `Stable <= Candidate` is
/// **true**. MEDIDO: writing a ceiling check as `channel <= ceiling` let a
/// `CandidateProducer` reach `Stable`, which is the one thing this module
/// exists to prevent.
///
/// A derived ordering that disagrees with the domain's own lattice is not a
/// detail to be careful about, it is a trap with a type system around it, so
/// this walks [`promotion_target`] instead. It costs a handful of comparisons
/// and it cannot be wrong in that direction.
pub fn promotion_distance(from: ReleaseChannel, to: ReleaseChannel) -> Option<u32> {
    let mut current = from;
    for steps in 0..=4 {
        if current == to {
            return Some(steps);
        }
        current = promotion_target(current)?;
    }
    None
}

/// Who is responsible for a release, and therefore where it stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseRole {
    /// Produces candidate material, verifies its **producer** gates, emits a
    /// handoff, and stops there.
    ///
    /// It does not tag a stable release, it does not publish one, and — the
    /// part that is easy to get wrong — it does not **wait**. A producer that
    /// blocks until an external party certifies has turned its handoff into a
    /// dependency it cannot control, and the work it can do independently stops
    /// with it. Termination is success, not incompleteness.
    CandidateProducer,
    /// Verifies the gates for a channel and says whether a release may be
    /// promoted into it.
    ///
    /// A certifier **does not promote**. Saying yes and doing it are two
    /// responsibilities, and a certifier that also promotes is a certifier that
    /// can certify its own work.
    Certifier,
    /// Moves a release along the channel lattice, once the gates allow it.
    ///
    /// Whether a *step* is legal is [`can_promote`]'s answer, not this role's.
    Promoter,
    /// Produces, certifies, promotes and publishes: the whole flow in one place.
    FullPublisher,
}

impl ReleaseRole {
    /// Every role, as it is written.
    pub const KNOWN: &'static [&'static str] = &[
        "candidate_producer",
        "certifier",
        "promoter",
        "full_publisher",
    ];

    /// This role's own name.
    ///
    /// One spelling, shared with the serialisation and with the parse error, for
    /// the reason the naming in [`crate::release_ref`] has one: a value written
    /// two ways is two values to keep in step.
    pub fn name(self) -> &'static str {
        match self {
            Self::CandidateProducer => "candidate_producer",
            Self::Certifier => "certifier",
            Self::Promoter => "promoter",
            Self::FullPublisher => "full_publisher",
        }
    }

    /// The declared role, read from its own name.
    pub fn parse(name: &str) -> Result<Self, UnknownReleaseRole> {
        match name {
            "candidate_producer" => Ok(Self::CandidateProducer),
            "certifier" => Ok(Self::Certifier),
            "promoter" => Ok(Self::Promoter),
            "full_publisher" => Ok(Self::FullPublisher),
            other => Err(UnknownReleaseRole {
                given: other.to_owned(),
                known: Self::KNOWN,
            }),
        }
    }

    /// The furthest channel this role is responsible for.
    ///
    /// A **ceiling**, not a destination: it says what this actor may publish,
    /// and says nothing about whether a given step to get there is legal. That
    /// is [`can_promote`]'s question and it is asked separately, on purpose.
    ///
    /// A certifier's ceiling is `Stable` because it may *certify into* `Stable`.
    /// It still may not promote there, and [`Self::may_promote`] is what keeps
    /// those two apart — a certifier that could promote would be certifying its
    /// own work.
    pub fn ceiling(self) -> ReleaseChannel {
        match self {
            // The whole point: a producer stops at candidate, and everything
            // past it belongs to somebody else.
            Self::CandidateProducer => ReleaseChannel::Candidate,
            Self::Certifier | Self::Promoter | Self::FullPublisher => ReleaseChannel::Stable,
        }
    }

    /// May this role carry a release as far as `channel`?
    ///
    /// **Not** "may this promotion happen" — that is `can_promote`, and calling
    /// it from here would put a second answer to the same question in the tree.
    /// This asks the different one, and the two are checked together.
    ///
    /// ## Why this walks the lattice instead of comparing two channels
    ///
    /// La primera versión comparaba con `<=`, asumiendo que el orden derivado
    /// del enum iba en el mismo sentido que la promoción. **No va**, y el
    /// falsador lo encontró medido: `ReleaseChannel` se declara `Stable,
    /// Candidate, Edge, Dev` —el orden documental, que va de mayor a menor—,
    /// luego su `Ord` derivado es el **revés** del retículo, y
    /// `Stable <= Candidate` es cierto. Un productor pasaba a poder llegar a
    /// `Stable`, que es exactamente lo que este módulo existe para impedir.
    ///
    /// Así que la pregunta se responde con [`promotion_distance`], que camina
    /// el retículo que ya existía. Y el **techo** se lee al revés de como se
    /// lee una promoción: un techo no es «hasta dónde llego», es «hasta dónde
    /// me dejan llegar», luego `channel` tiene que ser un **antepasado** del
    /// techo, no su sucesor.
    pub fn may_reach(self, channel: ReleaseChannel) -> bool {
        promotion_distance(channel, self.ceiling()).is_some()
    }

    /// May this role take the step `from → to`?
    ///
    /// Both questions, and both must hold: the lattice must allow that step,
    /// **and** the role must be allowed to go that far.
    ///
    /// **The lattice is asked first**, and the order is not cosmetic. A lattice
    /// rule is a property of the channels and does not depend on who is asking,
    /// so it is the more fundamental of the two refusals; and when the lattice
    /// allows the step, the role's ceiling is the only reason left, so asking it
    /// second never hides anything. Asking the role first would report "that is
    /// not your step" for a move the channels would not allow anyone to make.
    pub fn may_promote(self, from: ReleaseChannel, to: ReleaseChannel, gates_ok: bool) -> bool {
        can_promote(from, to, gates_ok) && self.may_reach(to)
    }

    /// Does the flow **end** at this role, or continue to someone else?
    ///
    /// A producer's answer is `true`, and it is the answer that makes a handoff
    /// work: the producer is done when the handoff is emitted. Treating that as
    /// incomplete is what makes a producer wait for a party it cannot see.
    pub fn terminates_here(self) -> bool {
        matches!(self, Self::CandidateProducer)
    }

    /// Does this role perform the step of **publishing** a release?
    ///
    /// Asked separately from [`Self::may_reach`] because the two differ for a
    /// producer: it may carry material as far as `Candidate`, and it still does
    /// not publish. Publishing is what the rest of the chain is for.
    pub fn publishes(self) -> bool {
        matches!(self, Self::FullPublisher)
    }
}

impl fmt::Display for ReleaseRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A role that does not exist.
///
/// Says what does, because a caller who typed a role that is not there is
/// deciding who publishes their software, and a message that only says "no"
/// makes that decision for them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReleaseRole {
    /// What the caller wrote.
    pub given: String,
    /// Every role that exists.
    pub known: &'static [&'static str],
}

impl fmt::Display for UnknownReleaseRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "«{}» no es un rol de release que este nucleo sepa declarar; los que \
             hay son: {}",
            self.given,
            self.known.join(", ")
        )
    }
}

impl std::error::Error for UnknownReleaseRole {}

/// What a producer hands to whoever certifies and promotes it.
///
/// **Opaque on purpose.** Everything in here is a fact about the release, and
/// none of it is a verdict about it. The concrete meaning of a candidate —what a
/// `CandidateId` means in the system that produced it, which sequence means
/// what, which tool is allowed to consume it— stays with the producer, and SDDK
/// carrying the envelope is the whole extent of what it claims to understand.
///
/// A handoff that carried a verdict would be a second authority: the producer's
/// own rules, re-expressed by a tool that does not have them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateHandoff {
    /// The target this material is about.
    pub target: ReleaseTarget,
    /// The version of the product the material carries.
    pub product_version: ProductVersion,
    /// The reference the material is published under.
    pub reference: ReleaseRef,
    /// Which candidate of the version it is, when it is a candidate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<CandidateSequence>,
    /// Where the source was when the material was built.
    pub source_revision: SourceRevision,
    /// The artifacts, content-addressable.
    pub artifacts: Vec<HandoffArtifact>,
    /// Evidence references backing the producer's own gates.
    pub evidence_refs: Vec<String>,
    /// What kind of handoff this is, in the **producer's** words.
    ///
    /// Free text on purpose: it is provenance for whoever consumes the envelope,
    /// not a discriminator this kernel is entitled to interpret. A kernel that
    /// understood the concrete type would be a kernel that has adopted one
    /// producer's vocabulary.
    pub external_handoff_type: String,
    /// A digest over the handoff as the producer serialised it.
    pub external_handoff_digest: String,
}

/// One artifact inside a handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffArtifact {
    /// What the producer calls it, in its own words.
    pub kind: String,
    /// Content-addressable: `sha256:<hex>` of the bytes.
    pub digest: String,
    /// Where it is, when the producer wants to say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Why a role cannot do what was asked of it.
///
/// Every variant names the role, because the operator's question is "what am I
/// supposed to be, then" and not "what went wrong".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleRefusal {
    /// The role's ceiling does not reach the requested channel.
    BeyondCeiling {
        /// The role that was asked.
        role: ReleaseRole,
        /// Its ceiling.
        ceiling: ReleaseChannel,
        /// The channel that was requested.
        requested: ReleaseChannel,
    },
    /// The channel lattice does not allow that step, whatever the role is.
    StepNotInLattice {
        /// The channel the release is on.
        from: ReleaseChannel,
        /// The channel it was asked to reach.
        to: ReleaseChannel,
    },
    /// The role must not perform this step at all — a certifier that promotes,
    /// or a producer that publishes.
    NotThisRolesStep {
        /// The role that was asked.
        role: ReleaseRole,
        /// What it was asked to do, in plain words.
        step: String,
    },
}

impl fmt::Display for RoleRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeyondCeiling {
                role,
                ceiling,
                requested,
            } => write!(
                f,
                "this target declares the role `{}`, whose responsibility ends at \
                 `{ceiling:?}`; carrying a release to `{requested:?}` is somebody \
                 else's step. Declare the role that owns it, or stop at `{ceiling:?}`",
                role.name()
            ),
            Self::StepNotInLattice { from, to } => write!(
                f,
                "the channel lattice does not allow `{from:?}` to `{to:?}` in one \
                 step, whoever the role is: that is the channel rule, not a \
                 permission"
            ),
            Self::NotThisRolesStep { role, step } => write!(
                f,
                "the role `{}` does not {step}. Saying yes and doing it are two \
                 responsibilities, and a party that does both is not certifying \
                 anything",
                role.name()
            ),
        }
    }
}

impl std::error::Error for RoleRefusal {}

/// Whether a role may take one step of the channel lattice.
///
/// The free-function twin of [`ReleaseRole::may_promote`], for callers that hold
/// a role as a value and do not want to reach for a method. Same question, same
/// order, one implementation — a free function **and** a method that answered
/// this independently would be two answers to the same question, which is the
/// defect this module exists to prevent.
///
/// Note what this does *not* decide: whether the release may be **published**.
/// That is [`may_publish`], and a step that is legal is not a publication.
pub fn may_promote(
    role: ReleaseRole,
    from: ReleaseChannel,
    to: ReleaseChannel,
    gates_ok: bool,
) -> bool {
    role.may_promote(from, to, gates_ok)
}

/// Whether a role may publish a release, with the reason when it may not.
///
/// The **one** question a caller asks before publishing, and it is asked as a
/// question rather than a boolean for the reason every other refusal in this
/// codebase is: a `false` without a reason is a hundred indistinguishable
/// failures wearing one name.
pub fn may_publish(
    role: ReleaseRole,
    from: ReleaseChannel,
    to: ReleaseChannel,
    gates_ok: bool,
) -> Result<(), RoleRefusal> {
    // El retículo primero, por el mismo motivo que en `may_promote`: su regla
    // no depende de quien pregunta, luego es el rechazo más fundamental de los
    // dos, y cuando lo permite el techo del rol es el único motivo que queda.
    if !can_promote(from, to, gates_ok) {
        return Err(RoleRefusal::StepNotInLattice { from, to });
    }
    if !role.may_reach(to) {
        return Err(RoleRefusal::BeyondCeiling {
            role,
            ceiling: role.ceiling(),
            requested: to,
        });
    }
    // Ceiling reached, lattice allows it, gates passed — and the role still may
    // not be the one to publish it. A certifier and a promoter both reach
    // `Stable`; neither publishes.
    if !role.publishes() {
        return Err(RoleRefusal::NotThisRolesStep {
            role,
            step: "publish a release".to_owned(),
        });
    }
    Ok(())
}
