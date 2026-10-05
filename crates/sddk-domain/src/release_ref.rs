//! A release reference, and how it comes to name a product version.
//!
//! # The problem this module exists for
//!
//! Until now, deciding whether a release reference named a product version was
//! one line: strip a leading `v`, compare strings. It worked for every case the
//! repository happened to contain, which is not the same as being right, and it
//! was right by accident rather than by decision. Three things were wrong with
//! it at once:
//!
//! 1. **The convention was invisible.** A prefix strip that happens to be one
//!    character looks exactly like a coercion, and a coercion is how two
//!    different values end up presented as one — which is the worst kind of
//!    green, because a release gets authorised on a number that is not the
//!    project's.
//! 2. **`-rc2` was not modelled.** `v1.0.0-rc2` and `v1.0.0` differ by a suffix
//!    and by a world: one is a candidate, the other is stable. Treating that as
//!    a string to normalise throws away the only thing that distinguishes them.
//! 3. **The channel was not part of the reference.** SDDK already had a
//!    canonical [`ReleaseChannel`] with a promotion lattice, and the reference
//!    did not mention it — so a candidate reference and a stable reference were
//!    the same value wearing two names.
//!
//! # The shape of the answer
//!
//! A [`ReleaseRef`] is a name **kept verbatim**. It is not parsed into a
//! version, and there is no function anywhere that turns a reference into a
//! product version by removing text from it.
//!
//! How a reference *names* a version is a [`VersionNaming`] — a **declared**
//! policy with one default that reproduces what this repository has always
//! done. The relation is then checked by construction, in both directions: the
//! naming produces the name a version would have, and the two are compared
//! exactly. Nothing is trimmed on the way there or on the way back.
//!
//! That direction matters. Building the expected name from the version and
//! comparing is a different operation from parsing the found name into a
//! version and comparing, and only the first one can be exhaustive: there is no
//! way to enumerate the ways a string can be cut to produce a value, and a
//! check that cannot be enumerated cannot be falsified.
//!
//! [`ReleaseChannel`]: crate::channel::ReleaseChannel

use crate::channel::ReleaseChannel;
use crate::version_authority::ProductVersion;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::num::NonZeroU32;

/// The Nth candidate of a product version.
///
/// A **counter, not an identity**. It says "this is the second candidate of
/// version X", and nothing else: not which pipeline produced it, not which
/// branch, not who built it. Keeping it a counter is what makes it general —
/// the moment it starts naming a specific system's sequence, it stops being a
/// number and becomes that system's vocabulary inside the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CandidateSequence(NonZeroU32);

impl CandidateSequence {
    /// The first candidate is `1`, not `0`.
    ///
    /// Non-zero on purpose: `-rc0` would be a reference that claims to be a
    /// candidate of something and has no predecessor, which is a question
    /// nobody asked and nobody can answer.
    pub fn first() -> Self {
        Self(NonZeroU32::new(1).expect("1 es un entero positivo"))
    }

    /// The Nth candidate, if there is one.
    pub fn nth(n: u32) -> Option<Self> {
        NonZeroU32::new(n).map(Self)
    }

    /// The count, as a plain number.
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for CandidateSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A name a release is referred to by.
///
/// **Verbatim.** The name is stored as given and never rewritten. What the
/// kernel knows about it is its channel and, when the naming says so, its
/// candidate sequence — both of which are facts about the reference rather than
/// things recovered from its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseRef {
    name: String,
    channel: ReleaseChannel,
    sequence: Option<CandidateSequence>,
}

impl ReleaseRef {
    /// A reference with a channel and no candidate sequence.
    pub fn new(name: impl Into<String>, channel: ReleaseChannel) -> Self {
        Self {
            name: name.into(),
            channel,
            sequence: None,
        }
    }

    /// A candidate reference: the Nth candidate of something, on a channel.
    ///
    /// The sequence is stored, **not parsed out of the name**. Constructing one
    /// does not check that the name mentions it: that check belongs to
    /// [`VersionNaming`], which is where the policy about names lives, and
    /// mixing them here would make the reference depend on a convention it is
    /// supposed to be independent of.
    pub fn candidate(
        name: impl Into<String>,
        channel: ReleaseChannel,
        sequence: CandidateSequence,
    ) -> Self {
        Self {
            name: name.into(),
            channel,
            sequence: Some(sequence),
        }
    }

    /// The name, exactly as it will be written.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The channel this reference belongs to.
    pub fn channel(&self) -> ReleaseChannel {
        self.channel
    }

    /// The candidate sequence, when this is a candidate.
    pub fn sequence(&self) -> Option<CandidateSequence> {
        self.sequence
    }

    /// Whether this reference names a candidate rather than a stable release.
    pub fn is_candidate(&self) -> bool {
        self.sequence.is_some()
    }
}

impl fmt::Display for ReleaseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// How a release reference names a product version.
///
/// A **declared policy**, not a library of heuristics. Each variant is one
/// complete rule about how to build a reference's name from a version, and
/// there is no variant that removes text from something that was not built by
/// the rule in the first place.
///
/// The default is [`VersionNaming::v_prefixed`], which reproduces what SDDK has
/// always done. It is named, documented and testable instead of being a
/// one-character strip buried in a comparison, and that is the whole change:
/// the behaviour is identical, the *decision* is now visible and can be argued
/// with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "style")]
pub enum VersionNaming {
    /// `v{version}` — the default. A prefix and nothing else.
    VPrefixed {
        /// The prefix. One character by convention, and any string by
        /// declaration: a project that tags `release-1.2.3` declares that.
        prefix: String,
    },
    /// `{version}` — the name IS the version. No prefix, no suffix.
    Exact,
    /// `v{version}-rc{sequence}` on a candidate channel.
    ///
    /// The sequence is a **declared part of the name**, compared as a number.
    /// That is what makes a candidate distinguishable from its version instead
    /// of being a version with text to be trimmed.
    PrefixedCandidate {
        /// The prefix before the version.
        prefix: String,
        /// The separator between the version and the sequence marker.
        separator: String,
        /// The marker before the sequence number, conventionally `rc`.
        marker: String,
    },
}

impl VersionNaming {
    /// The default: a leading `v` and nothing else.
    pub fn v_prefixed() -> Self {
        Self::VPrefixed {
            prefix: "v".to_owned(),
        }
    }

    /// A naming that declares a candidate's sequence as part of the name.
    ///
    /// ## Por qué este constructor existe ahora y no en ADR-0159
    ///
    /// Porque `PrefixedCandidate` se podía **usar** —`name_for` y `binds` lo
    /// conocían— pero no **construir**: el único `parse` acepta dos estilos y
    /// ninguno es éste. MEDIDO en `sddk release handoff`: un handoff de candidato
    /// es imposible, porque `--sequence` sólo puede producir un
    /// `NamingHasNoRoomForCandidates`.
    ///
    /// O sea: la convención estaba declarada, documentada y falsificada, y era
    /// inalcanzable desde fuera del crate. Es la clase de hueco que un tipo
    /// exercising y su falsador no detectan, porque los dos viven **dentro** del
    /// módulo que lo declara.
    ///
    /// Y se construye con sus tres partes explícitas, sin un atajo: `rc` es una
    /// convención, no una ley, y un atajo sería un default invisible en el
    /// crate que sostiene los demás defaults.
    pub fn prefixed_candidate(
        prefix: impl Into<String>,
        separator: impl Into<String>,
        marker: impl Into<String>,
    ) -> Self {
        Self::PrefixedCandidate {
            prefix: prefix.into(),
            separator: separator.into(),
            marker: marker.into(),
        }
    }

    /// The name this naming gives to `version`, or to a candidate of it.
    ///
    /// `None` when the naming **cannot express** the reference asked for — a
    /// candidate sequence under a naming with no room for one. Not a fallback
    /// and not a dropped sequence: a function that can return a name missing
    /// the sequence is a function that can lie, and the callers that need a
    /// name are exactly the ones that would then publish it.
    ///
    /// **This is the only function in the codebase that constructs a reference
    /// name.** Its counterpart — deciding whether a given reference binds a
    /// given version — is [`binds`], and it compares the two exactly.
    ///
    /// The asymmetry is the design: generating a name is enumerable, parsing a
    /// name is not.
    pub fn name_for(
        &self,
        version: &ProductVersion,
        sequence: Option<CandidateSequence>,
    ) -> Option<String> {
        match (self, sequence) {
            (Self::Exact, None) => Some(version.to_string()),
            (Self::Exact, Some(_)) => None,
            (Self::VPrefixed { prefix }, None) => Some(format!("{prefix}{version}")),
            // A sequence under `VPrefixed` would have to be dropped to produce
            // a name, and dropping it is what would make a candidate look like
            // its version. The project that wants candidates declares
            // `PrefixedCandidate`, which has room for them.
            (Self::VPrefixed { .. }, Some(_)) => None,
            (
                Self::PrefixedCandidate {
                    prefix,
                    separator,
                    marker,
                },
                Some(sequence),
            ) => Some(format!("{prefix}{version}{separator}{marker}{sequence}")),
            (
                Self::PrefixedCandidate {
                    prefix, separator, ..
                },
                None,
            ) => Some(format!("{prefix}{version}{separator}")),
        }
    }
}

impl Default for VersionNaming {
    /// The naming SDDK has always used, made visible.
    fn default() -> Self {
        Self::v_prefixed()
    }
}

/// A naming that a name alone cannot carry.
///
/// Not one naming that does not exist: [`VersionNaming::PrefixedCandidate`]
/// exists, takes three parameters, and is therefore **not reachable from a
/// string**. Passing its style is a different mistake from passing nonsense,
/// and the two are told apart here rather than collapsed into one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVersionNaming {
    /// What the caller wrote.
    pub given: String,
    /// Every naming a name alone can express.
    pub known: &'static [&'static str],
    /// The one that exists and still cannot be named this way.
    pub needs_parameters: &'static str,
}

impl fmt::Display for UnknownVersionNaming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.given == self.needs_parameters {
            return write!(
                f,
                "«{}» existe, pero no se puede declarar con un nombre: necesita \
                 prefijo, separador y marcador, y pasarlos dentro de una sola \
                 palabra los esconderia. Las que si se declaran por nombre son: {}",
                self.given,
                self.known.join(", ")
            );
        }
        write!(
            f,
            "«{}» no es una convencion de nombres que este nucleo sepa declarar; \
             las que si se declaran por nombre son: {}",
            self.given,
            self.known.join(", ")
        )
    }
}

impl std::error::Error for UnknownVersionNaming {}

impl VersionNaming {
    /// The namings a name alone can express.
    ///
    /// Two, and not three, on purpose. `PrefixedCandidate` is a real naming and
    /// is absent from this list because it takes three parameters: a name that
    /// carried them would hide them, and a convention nobody can read is a
    /// convention nobody can check. It is constructed in code, where the prefix,
    /// the separator and the marker are three visible values.
    pub const NAMED: &'static [&'static str] = &["v_prefixed", "exact"];

    /// The naming that exists and still cannot be named this way.
    pub const NEEDS_PARAMETERS: &'static str = "prefixed_candidate";

    /// This naming's own name.
    ///
    /// One spelling, used in three places: the CLI flag, the JSON, and the
    /// error above. They are the same word on purpose — a convention written
    /// `v-prefixed` on the command line and `v_prefixed` in the output is two
    /// spellings of one value, and a caller reading the output has to know
    /// which one to type.
    pub fn style(&self) -> &'static str {
        match self {
            Self::VPrefixed { .. } => "v_prefixed",
            Self::Exact => "exact",
            Self::PrefixedCandidate { .. } => "prefixed_candidate",
        }
    }

    /// The declared naming, read from its own name.
    ///
    /// See [`Self::NAMED`] for why this set is two and not three.
    pub fn parse(style: &str) -> Result<Self, UnknownVersionNaming> {
        match style {
            "v_prefixed" => Ok(Self::v_prefixed()),
            "exact" => Ok(Self::Exact),
            other => Err(UnknownVersionNaming {
                given: other.to_owned(),
                known: Self::NAMED,
                needs_parameters: Self::NEEDS_PARAMETERS,
            }),
        }
    }
}

/// Why a reference does or does not name a version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindOutcome {
    /// The reference names this version, under the declared naming.
    Binds,
    /// It does not, and both sides are reported.
    ///
    /// Both sides, always. A mismatch that only says "no" sends the reader to
    /// guess which of the two is wrong; one that prints the expected name and
    /// the found name makes the correction mechanical. The whole class of
    /// "it says the wrong number" reports is unusable without both.
    DoesNotBind {
        /// The name this naming gives to the version.
        expected: String,
        /// The name that was actually presented.
        found: String,
    },
    /// The reference carries a candidate sequence and the naming has no room
    /// for one.
    ///
    /// Separate from [`DoesNotBind`](Self::DoesNotBind) because the repair is
    /// different: not "the name is wrong" but "this project has not declared how
    /// it names candidates". Reporting it as a plain mismatch would send
    /// someone to rename the reference instead of to declare the convention.
    NamingHasNoRoomForCandidates,
}

impl BindOutcome {
    /// Whether the reference names the version.
    pub fn is_bound(&self) -> bool {
        matches!(self, Self::Binds)
    }

    /// The message a release shows when the reference does not name the
    /// version.
    pub fn message(&self, version: &ProductVersion) -> String {
        match self {
            Self::Binds => format!("{version} is named by this release reference"),
            Self::DoesNotBind { expected, found } => format!(
                "this release reference does not name version {version}: the declared \
                 naming gives {expected}, and the reference is {found}"
            ),
            Self::NamingHasNoRoomForCandidates => format!(
                "this release reference names a candidate, and the declared naming has no \
                 place for a candidate sequence. Declare a naming that has one rather than \
                 trimming the reference's text to make it fit {version}."
            ),
        }
    }
}

/// Whether `reference` names `version` under `naming`.
///
/// **No substring matching, no trimming, no normalisation.** The check is one
/// string equality between the name the naming produces and the name the
/// reference has.
///
/// The cases that used to be wrong and are the reason this function exists:
///
/// - `v1.0.0-rc2` does **not** name `1.0.0` under [`VersionNaming::v_prefixed`].
///   It could only be made to agree by dropping `-rc2`, and dropping a
///   candidate suffix to authorise a stable release is the coercion that
///   `ADR-0157` was about in a different guise.
/// - `v1.0.0` does **not** name `1.0.0` under [`VersionNaming::Exact`]. Exact
///   means exact, in both directions.
/// - A reference with a sequence does not bind under a naming without room for
///   one, and that is reported as its own case because the repair is different.
pub fn binds(
    reference: &ReleaseRef,
    version: &ProductVersion,
    naming: &VersionNaming,
) -> BindOutcome {
    // `name_for` devuelve `None` cuando la naming no puede expresar lo que se le
    // pide, y ese caso tiene su propia salida porque la reparacion es distinta:
    // no es que el nombre este mal, es que el proyecto no ha declarado como
    // nombra candidatas.
    let Some(expected) = naming.name_for(version, reference.sequence()) else {
        return BindOutcome::NamingHasNoRoomForCandidates;
    };
    if expected == reference.name() {
        BindOutcome::Binds
    } else {
        BindOutcome::DoesNotBind {
            expected,
            found: reference.name().to_owned(),
        }
    }
}

/// The revision a release was cut from.
///
/// A distinct concept from a version and from a release reference, and the
/// distinction is load-bearing: a revision identifies *where the source was*,
/// a version identifies *what the product is*, and a reference identifies *what
/// this release is called*. A release carries all three, and none of them
/// substitutes for another — a build whose source moved and whose version did
/// not is a normal thing that happens, and a tool that treats the revision as a
/// version or the other way round cannot represent it.
///
/// The kernel knows nothing about version control. What a revision looks like
/// is a provider's business; what it *means* is not.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceRevision(String);

impl SourceRevision {
    /// A revision, if it is a non-empty identity.
    pub fn new(revision: impl Into<String>) -> Option<Self> {
        let revision = revision.into();
        if revision.trim().is_empty() {
            None
        } else {
            Some(Self(revision))
        }
    }

    /// The revision, verbatim.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourceRevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
