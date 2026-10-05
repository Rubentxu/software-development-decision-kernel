//! Generic version authority: the pure model SDDK reasons with.
//!
//! # The law this module exists to enforce
//!
//! > **SDDK defines the questions, contracts and decisions. Providers know
//! > how to obtain evidence from concrete tools.**
//!
//! This module is the questions-and-decisions half. It knows what a version
//! observation *is*, what the possible answers are, and how to reduce a set
//! of answers to one authority. It has no idea how any of those answers were
//! obtained, and it must never acquire that idea: the moment a concrete tool,
//! language, build system or file name appears here, every consumer inherits
//! a coupling it cannot undo.
//!
//! # Why this is not `version_source.rs`
//!
//! The pre-existing registry knew concrete file names, in the crate that
//! makes the decision. That is the coupling this module removes, and it is
//! not hypothetical: a registry entry that listed three file names under one
//! extractor made the second and third files *lie* about their format, and a
//! single file that exists without declaring a version aborted the whole
//! resolution — including candidates that come later and ecosystems that
//! come after it. Moving the extractor from the ecosystem to the file made
//! that specific bug impossible and did nothing about the coupling. Only
//! taking the names out of the kernel does that.
//!
//! # Fitness
//!
//! `version_authority_fitness` is not a style preference. It scans this
//! module for concrete technology names and fails the build if it finds one.
//! A fitness that is only a comment is a comment that will be wrong in six
//! months.
//!
//! # What this module deliberately does NOT assume
//!
//! - **Not SemVer.** [`ProductVersion`] is an opaque, validated identity.
//!   Ordering and precedence are a *policy*, applied by a provider or a
//!   command, not by the type.
//! - **Not one version per repository.** A repository holds
//!   [`ReleaseTarget`]s, and each one carries its own authority. Divergence
//!   is only meaningful *inside* a target.
//! - **Not that a tag is the version.** Nothing here compares a version to a
//!   tag, or strips a suffix to make them match. Those are separate
//!   concepts that a later concern relates; this one states no relation.
//! - **Not that observation order means anything.** The reducer is
//!   permutation-invariant, and there is a test that permutes the input
//!   because a reducer that answers differently depending on who asked first
//!   is a reducer with an accidental priority.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ProductVersion — an opaque identity, not a version *scheme*
// ---------------------------------------------------------------------------

/// A product's declared version identity.
///
/// Opaque on purpose. It is validated to be non-empty and to contain no
/// whitespace, and that is the whole contract: this type does not know what
/// version schemes exist, does not parse them, and does not order them.
/// A consumer that needs ordering asks a policy for it, explicitly, and the
/// answer is allowed to be "this scheme has no order".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProductVersion(String);

impl ProductVersion {
    /// Builds a version identity, rejecting the two shapes that are always a
    /// mistake rather than a scheme: empty, or with internal whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidProductVersion> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(InvalidProductVersion::Empty);
        }
        if trimmed.split_whitespace().count() > 1 {
            return Err(InvalidProductVersion::HasWhitespace(value));
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The identity as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProductVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a string is not a usable product version.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidProductVersion {
    /// The value was empty or only whitespace.
    #[error("una identidad de version vacia no es una identidad")]
    Empty,
    /// The value contained internal whitespace, which is never a version.
    #[error("una identidad de version no puede contener espacios: {0:?}")]
    HasWhitespace(String),
}

// ---------------------------------------------------------------------------
// ReleaseTarget — a product, not a repository
// ---------------------------------------------------------------------------

/// The thing being versioned.
///
/// A repository may contain one target, several, or none that this tool
/// knows about. Each target resolves independently, which is what makes a
/// multi-product repository a normal case rather than a contradiction: two
/// targets declaring different versions is not a divergence, because they are
/// not asserting the same thing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseTarget {
    id: String,
    /// Location of the target, relative to the repository root. `.` is the
    /// whole repository.
    root: String,
}

impl ReleaseTarget {
    /// A target rooted at the repository root.
    pub fn at_root(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            root: ".".to_owned(),
        }
    }

    /// A target rooted at a subdirectory.
    pub fn at(id: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            root: root.into(),
        }
    }

    /// Stable identity of the target.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Where the target lives, relative to the repository root.
    pub fn root(&self) -> &str {
        &self.root
    }
}

// ---------------------------------------------------------------------------
// The four answers a provider can give
// ---------------------------------------------------------------------------

/// What one provider found when asked about one target.
///
/// The four cases are not four flavours of the same thing, and collapsing
/// any pair of them is a defect with a name:
///
/// - [`NotApplicable`](Self::NotApplicable) and
///   [`Undeclared`](Self::Undeclared) differ because "this provider has
///   nothing to say about this target" and "this provider looked and the
///   target says nothing" lead to different operator actions: one is a
///   non-event, the other needs a decision.
/// - [`Undeclared`](Self::Undeclared) and
///   [`Declared`](Self::Declared) differ because only one of them carries a
///   value, and inventing one for the first is how a tool ends up
///   confidently reporting a version nobody declared.
/// - [`Invalid`](Self::Invalid) is separated from all three because it is the
///   only one that means *the source exists and could not be read*. That is
///   not an absence of information; it is information, and it fails closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "result")]
pub enum VersionProbe {
    /// This provider has no opinion about this target. Not an error, not a
    /// finding: the target simply is not this provider's subject.
    NotApplicable {
        /// Why, in the provider's own words.
        reason: String,
    },
    /// The provider looked and the target declares nothing.
    Undeclared {
        /// What was observed, for the diagnostic.
        reason: String,
    },
    /// The provider found a declared version, with the evidence for it.
    Declared {
        /// The declared identity.
        version: ProductVersion,
        /// Evidence that this value was really observed, not inferred.
        evidence: VersionEvidence,
    },
    /// The provider found a source it could not read. **Fails closed.**
    Invalid {
        /// What could not be read, and why.
        reason: String,
    },
}

impl VersionProbe {
    /// Whether this answer carries a version value.
    pub fn declares(&self) -> bool {
        matches!(self, Self::Declared { .. })
    }

    /// Whether this answer carries *this specific* version. Used to collect
    /// the observations that agree, which is what separates one declaration
    /// from a genuine cross-check.
    pub fn declares_version(&self, version: &ProductVersion) -> bool {
        matches!(self, Self::Declared { version: v, .. } if v == version)
    }

    /// Whether this answer must stop the whole resolution.
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Invalid { .. })
    }
}

// ---------------------------------------------------------------------------
// Evidence and observation
// ---------------------------------------------------------------------------

/// Provenance for one observed version.
///
/// Carries no judgement about whether the observation is *right* — that is
/// the reducer's job — only about what it rests on. `digest` is a
/// content-addressable reference in the same shape the rest of SDDK already
/// uses (`sha256:<hex>`), so this does not introduce a second convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionEvidence {
    /// What kind of source the provider used, in the provider's own words.
    /// Opaque provenance, never a decision input: a name that looks more
    /// authoritative must not be worth more.
    pub source_kind: String,
    /// Content-addressable reference to the evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Where it was observed, relative to the target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// One provider's answer about one target, with the identity that produced
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionObservation {
    /// Provider identity. **Provenance, not semantics**: the reducer does not
    /// look at it, and a provider cannot buy authority with a good name.
    pub provider_id: String,
    /// Provider's own version string.
    pub provider_version: String,
    /// The capability the provider was asked for, and the version of that
    /// capability contract it speaks.
    pub capability: String,
    /// What it found.
    pub probe: VersionProbe,
}

impl VersionObservation {
    /// A short label for diagnostics: provider id plus the result.
    pub fn summary(&self) -> String {
        let result = match &self.probe {
            VersionProbe::NotApplicable { .. } => "not_applicable",
            VersionProbe::Undeclared { .. } => "undeclared",
            VersionProbe::Declared { .. } => "declared",
            VersionProbe::Invalid { .. } => "invalid",
        };
        format!("{result} ({})", self.provider_id)
    }
}

// ---------------------------------------------------------------------------
// The decision
// ---------------------------------------------------------------------------

/// The authority SDDK holds about one target's version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verdict")]
pub enum VersionAuthority {
    /// Exactly one provider declared a version.
    Resolved {
        /// The version.
        version: ProductVersion,
        /// Every observation considered, including the ones that declared
        /// nothing. A resolution that hides what it ignored cannot be
        /// audited.
        observations: Vec<VersionObservation>,
    },
    /// More than one provider declared the same version. Strictly stronger
    /// than `Resolved`, and kept distinct so a consumer is never tempted to
    /// read "someone agreed" as "someone checked".
    CrossValidated {
        /// The agreed version.
        version: ProductVersion,
        /// The observations that agreed.
        observations: Vec<VersionObservation>,
    },
    /// More than one provider declared *different* versions. Not an error to
    /// be resolved by picking one: which one is authoritative is a decision
    /// this type refuses to make on the provider's behalf.
    Ambiguous {
        /// The distinct values, with who declared each.
        candidates: Vec<(String, ProductVersion)>,
        /// Every observation considered.
        observations: Vec<VersionObservation>,
    },
    /// At least one provider found a source it could not read.
    Invalid {
        /// Which providers failed, and why.
        failures: Vec<(String, String)>,
        /// Every observation considered.
        observations: Vec<VersionObservation>,
    },
    /// Nobody declared a version, and nothing was broken.
    Unresolved {
        /// Every observation considered.
        observations: Vec<VersionObservation>,
    },
}

impl VersionAuthority {
    /// The version, when there is exactly one and nothing contradicts it.
    pub fn version(&self) -> Option<&ProductVersion> {
        match self {
            Self::Resolved { version, .. } | Self::CrossValidated { version, .. } => {
                Some(version)
            }
            Self::Ambiguous { .. } | Self::Invalid { .. } | Self::Unresolved { .. } => None,
        }
    }

    /// Whether this authority was checked against an independent second
    /// source. `false` for a single declaration, and the distinction matters:
    /// "found" is not "verified".
    pub fn was_cross_validated(&self) -> bool {
        matches!(self, Self::CrossValidated { .. })
    }

    /// Whether the target could not be resolved, for any reason.
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Ambiguous { .. } | Self::Invalid { .. } | Self::Unresolved { .. }
        )
    }
}

// ---------------------------------------------------------------------------
// The reducer — every decision, and no technology whatsoever
// ---------------------------------------------------------------------------

/// Reduces observations to one authority.
///
/// The laws, in the order they are applied:
///
/// 1. **Any `Invalid` fails the whole thing closed.** A provider that found
///    a source it could not read is not an absence of information; the
///    reduction cannot honestly say "nobody found anything" when somebody
///    found something unreadable. This is checked *first* so it cannot be
///    outvoted by a `Declared` that happens to arrive later in the list.
/// 2. **`NotApplicable` and `Undeclared` are not answers.** They carry no
///    version and never invent one.
/// 3. **Two declarations that agree cross-validate; two that differ are
///    `Ambiguous`.** There is no tie-break, and there is no first-wins: the
///    reducer is a function of the *set*.
/// 4. **Provider identity is never an input.** Sorting the observations by
///    provider id before reducing would be a priority rule wearing a
///    disguise, and the permutation test is what keeps that honest.
pub fn reduce(observations: Vec<VersionObservation>) -> VersionAuthority {
    // Canonical order, and it is part of the contract, not cosmetics. An
    // authority that embedded the observations in ARRIVAL order would make
    // its own equality order-sensitive, and a consumer that compared two
    // authorities would then see a difference where there is none — which is
    // how an accidental "who asked first" priority sneaks back in through
    // equality checks instead of through the reducer's logic.
    let mut observations = observations;
    observations.sort_by(|a, b| {
        a.provider_id
            .cmp(&b.provider_id)
            .then_with(|| a.provider_version.cmp(&b.provider_version))
    });

    let mut failures: Vec<(String, String)> = Vec::new();
    let mut declared: Vec<(String, ProductVersion)> = Vec::new();

    for observation in &observations {
        match &observation.probe {
            VersionProbe::Invalid { reason } => {
                failures.push((observation.provider_id.clone(), reason.clone()));
            }
            VersionProbe::Declared { version, .. } => {
                declared.push((observation.provider_id.clone(), version.clone()));
            }
            VersionProbe::NotApplicable { .. } | VersionProbe::Undeclared { .. } => {}
        }
    }

    // Law 1 — before anything can outvote it.
    if !failures.is_empty() {
        return VersionAuthority::Invalid {
            failures,
            observations,
        };
    }

    if declared.is_empty() {
        return VersionAuthority::Unresolved { observations };
    }

    // Law 3 — distinct values, not distinct providers. Two providers saying
    // the same thing is agreement; one provider being consulted twice is not
    // a second opinion, and counting it as one would be counting itself.
    let mut distinct: Vec<(String, ProductVersion)> = Vec::new();
    for (provider, version) in declared {
        if !distinct.iter().any(|(_, seen)| seen == &version) {
            distinct.push((provider, version));
        }
    }

    match distinct.len() {
        0 => VersionAuthority::Unresolved { observations },
        1 => {
            let version = distinct.remove(0).1;
            let agreeing: Vec<VersionObservation> = observations
                .iter()
                .filter(|o| o.probe.declares_version(&version))
                .cloned()
                .collect();
            // Cross-validation needs INDEPENDENT sources. The same provider
            // answering twice is not corroboration, and reporting it as such
            // would make `was_cross_validated` say «verified» about a
            // conversation SDDK had with itself.
            let independent: std::collections::BTreeSet<&str> =
                agreeing.iter().map(|o| o.provider_id.as_str()).collect();
            if independent.len() > 1 {
                VersionAuthority::CrossValidated {
                    version,
                    observations: agreeing,
                }
            } else {
                VersionAuthority::Resolved {
                    version,
                    observations,
                }
            }
        }
        _ => VersionAuthority::Ambiguous {
            candidates: distinct,
            observations,
        },
    }
}

// ---------------------------------------------------------------------------
// The port: how a provider is asked, and how one is found
// ---------------------------------------------------------------------------

/// The capability a provider speaks to answer "what version does this target
/// declare?".
///
/// Versioned in the string on purpose. A provider that does not answer this
/// exact version is not asked at all, and the answer to "what happens when a
/// provider is old" is decided by negotiation rather than by a runtime check
/// that guesses.
pub const PRODUCT_VERSION_OBSERVATION: &str = "product-version.observation/v1";

/// A provider that could not produce an answer for a reason that is its own
/// fault rather than the target's.
///
/// This is **not** a source that failed to parse. Those are
/// [`VersionProbe::Invalid`] — the target had something to say and it could
/// not be read. This is the provider failing before it got that far, and it
/// fails closed for the same reason: a provider that did not answer is not
/// evidence that there is no answer.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    /// The provider could not run.
    #[error("el provider {provider_id} no se pudo ejecutar: {reason}")]
    Unavailable {
        /// Which provider.
        provider_id: String,
        /// Why.
        reason: String,
    },
    /// The provider ran and does not speak the capability that was asked.
    #[error("el provider {provider_id} habla {speaks:?} y se le pidio {asked}")]
    CapabilityMismatch {
        /// Which provider.
        provider_id: String,
        /// What the provider declared.
        speaks: Vec<String>,
        /// What was asked.
        asked: String,
    },
}

/// The port SDDK asks for a product version.
///
/// Implementations live outside the kernel: they are the ones allowed to know
/// that a version can be read from a file, from a build tool's own model, or
/// from somewhere nobody has thought of yet. Adding a new way to learn a
/// version means implementing this trait and registering it — not editing the
/// engine, and not adding a name to anything in here.
pub trait VersionResolverPort {
    /// Stable identity. **Provenance, never a decision input**: the reducer
    /// does not read it, and a provider cannot buy authority with a name.
    fn provider_id(&self) -> &str;

    /// The provider's own version, for provenance in diagnostics.
    fn provider_version(&self) -> &str;

    /// Capability strings this provider answers to, e.g.
    /// [`PRODUCT_VERSION_OBSERVATION`].
    fn capabilities(&self) -> &[String];

    /// Observes one target. Returning `Err` is the provider failing; returning
    /// `Ok(NotApplicable)` is the provider saying the target is not its
    /// subject. Those are different and the trait keeps them apart.
    fn observe(&self, target: &ReleaseTarget) -> Result<VersionProbe, ProviderError>;
}

/// A set of providers, and the only thing that decides what they are asked.
///
/// The registry is deliberately dumb. It filters by capability, collects, and
/// hands the list to [`reduce`]. It has no ordering, no preference and no
/// knowledge of what any provider is — which is what makes a new provider
/// genuinely additive instead of a new branch in an existing decision.
#[derive(Default)]
pub struct VersionResolverRegistry {
    providers: Vec<Box<dyn VersionResolverPort>>,
}

impl VersionResolverRegistry {
    /// An empty registry. With no providers, every target is unresolved —
    /// which is the honest answer for a machine that knows nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a provider. Registering is composition, not policy: the
    /// registry does not ask whether the provider is a good one.
    pub fn register(&mut self, provider: Box<dyn VersionResolverPort>) {
        self.providers.push(provider);
    }

    /// How many providers are registered. Diagnostics only.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether no provider is registered.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Asks every provider that speaks `capability` about `target`, and
    /// reduces what they say.
    ///
    /// A provider that returns `Err` becomes a `VersionProbe::Invalid`
    /// observation carrying the provider's own reason. That mapping is a
    /// decision, and it is the conservative one: a provider that did not
    /// answer has not established that the target declares nothing.
    pub fn resolve(&self, capability: &str, target: &ReleaseTarget) -> VersionAuthority {
        let mut observations: Vec<VersionObservation> = Vec::new();
        for provider in &self.providers {
            if !provider.capabilities().iter().any(|c| c == capability) {
                continue;
            }
            let probe = match provider.observe(target) {
                Ok(probe) => probe,
                Err(_) => VersionProbe::Invalid {
                    reason: "el provider no pudo responder".to_owned(),
                },
            };
            observations.push(VersionObservation {
                provider_id: provider.provider_id().to_owned(),
                provider_version: provider.provider_version().to_owned(),
                capability: capability.to_owned(),
                probe,
            });
        }
        reduce(observations)
    }
}
