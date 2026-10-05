//! Why a version resolved, or did not — in a form an agent can read.
//!
//! # The problem this module exists for
//!
//! Everything above it can already explain itself to a human who reads the
//! code: the reducer is a function of a set, every verdict carries its
//! observations, and the plan prints the authority. What none of that gives is
//! an answer to the question somebody actually has when a release is refused:
//!
//! > *why* did it resolve that, and what did it **not** look at?
//!
//! Answering that today means reading `version_provider.rs`, and an agent that
//! opens a file to understand a verdict is an agent whose answer depends on a
//! refactor. So the explanation is built once, here, from what the registry
//! already knows.
//!
//! # The three things this will not do
//!
//! **It does not decide.** The verdict comes from [`reduce`] and this module
//! only describes it. A second reduction would be a second authority that
//! diverges.
//!
//! **It does not certify.** Finding a `ProductVersion` is not the same as
//! certifying a release, and the report says so in its own output rather than
//! in a doc comment nobody reads while a release is failing.
//!
//! **It does not recommend.** It never says «add `version=` to X». The report
//! describes what was observed; what to do about it is the operator's decision,
//! and a tool that decides which file a project should have is a tool that has
//! adopted one ecosystem's layout as the truth.
//!
//! [`reduce`]: crate::version_authority::reduce

use crate::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ProductVersion, ReleaseTarget, VersionAuthority,
    VersionObservation, VersionProbe,
};
use serde::{Deserialize, Serialize};

/// How much was actually established about one thing.
///
/// ## Six values, and why they are not a single scale
///
/// They look like a severity ladder and they are not, which is why this is an
/// enum with six cases and not a number:
///
/// - [`Observed`](Self::Observed) — someone declared it. **One** declaration.
/// - [`CrossValidated`](Self::CrossValidated) — two independent sources agreed.
///   Strictly stronger, and kept apart because "found" is not "verified".
/// - [`Conflict`](Self::Conflict) — two sources disagreed, and nobody was
///   chosen. A refusal with the disagreement attached.
/// - [`Invalid`](Self::Invalid) — a source was found and could not be read.
///   Fails closed.
/// - [`NotApplicable`](Self::NotApplicable) — nobody was asked, because it was
///   not this provider's subject. **Not** a failure.
/// - [`NotChecked`](Self::NotChecked) — nobody looked, and that is worth
///   knowing. The absence of a check and the absence of a result are different
///   facts, and a report that merges them claims a thoroughness nobody performed.
///
/// `NotChecked` is the one that makes this module worth writing. Everything else
/// a plan can already print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssuranceLevel {
    /// One source declared a value and nothing contradicted it.
    Observed,
    /// Two or more independent sources declared the same value.
    CrossValidated,
    /// Sources disagreed. No winner was chosen, by design.
    Conflict,
    /// A source was found and could not be read.
    Invalid,
    /// Not this provider's subject. Nobody was asked.
    NotApplicable,
    /// Nobody looked, and the report says so rather than staying silent.
    NotChecked,
}

/// What one provider had to say about one target.
///
/// The identity is **provenance**, carried through and never used as a decision
/// input — the same rule the reducer follows. The point of showing it here is
/// the opposite one: a report that says "some provider said 1.2.3" without
/// saying which is a report nobody can act on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderFinding {
    /// The provider's stable identity.
    pub provider_id: String,
    /// The provider's own version, for reproducibility.
    pub provider_version: String,
    /// What the provider answered.
    pub level: AssuranceLevel,
    /// The declared value, when there was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_version: Option<ProductVersion>,
    /// Where the value was observed, when the provider said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// A content-addressable reference to the evidence, when there was any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Whether this finding carries anything that can be pointed at.
    ///
    /// **Not** the same as "has evidence": a provider always names its
    /// `source_kind`, and that is a description, not a handle. This is true
    /// only when there is a digest or a location, which is what makes the claim
    /// re-checkable.
    pub recheckable: bool,
    /// The provider's own words for what it did, in every case.
    ///
    /// **Always present**, including for `Declared`. A provider that reports a
    /// value and says nothing about where it came from has reported something
    /// that cannot be re-checked, and the report has to be able to say that.
    pub detail: String,
}

/// A provider that was registered but never asked.
///
/// Kept because "who did not get a say" is a different question from "who got
/// a say and said nothing", and a report that lists only the second one reads
/// as a unanimous decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkippedProvider {
    /// The provider's stable identity.
    pub provider_id: String,
    /// What it can answer, in its own words.
    pub capabilities: Vec<String>,
}

/// Something nobody checked, stated as a fact about this report.
///
/// ## Why these are computed and not written by hand
///
/// A fixed list of caveats is a promise that does not track the code. These are
/// **derived from what actually happened** — a resolution from one source
/// produces the cross-validation caveat, a resolution from two does not — so if
/// the reducer's behaviour changes, these change with it, and a test can hold
/// them to it. A caveat that lies about what was skipped is worse than no
/// caveat, because it is believed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotChecked {
    /// A stable key, so a machine consumer can react without parsing prose.
    pub key: String,
    /// The same fact, for a human.
    pub statement: String,
}

/// Everything there is to know about how a target's version was resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionInspection {
    /// Which product this is about.
    pub target: ReleaseTarget,
    /// The capability that was requested. Declared, not inferred.
    pub capability: String,
    /// Every provider registered, before any filtering.
    pub providers_considered: Vec<String>,
    /// The providers that spoke this capability and were asked.
    pub providers_answering: Vec<String>,
    /// One entry per provider that was asked.
    pub findings: Vec<ProviderFinding>,
    /// Registered but filtered out because they speak another capability.
    pub skipped: Vec<SkippedProvider>,
    /// What was finally concluded.
    pub authority: AssuranceLevel,
    /// The version, when exactly one was established.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<ProductVersion>,
    /// The raw verdict, for a consumer that needs the machine shape.
    pub verdict: VersionAuthority,
    /// What nobody checked.
    pub not_checked: Vec<NotChecked>,
}

impl VersionInspection {
    /// Builds the report from a verdict and **the consultation that produced it**.
    ///
    /// `consulted` is the un-reduced set every provider that speaks the capability
    /// was asked about; `verdict` must be [`reduce`] applied to that very set. Both
    /// are arguments precisely because they are two different questions: `verdict`
    /// answers «what was concluded», `consulted` answers «who was asked, and what
    /// did they say» — and the verdict cannot answer the second one, because
    /// `reduce` deliberately throws the absences away.
    ///
    /// Passing a verdict that was **not** reduced from `consulted` produces a
    /// report that describes one conversation and concludes about another. Nothing
    /// here checks that at runtime, because the only caller is the registry and it
    /// builds them together; the falsification suite pins it instead, so the
    /// invariant is falsifiable rather than merely asserted in a comment.
    ///
    /// ## Why the verdict is an input and not a computation here
    ///
    /// Because the reducer is the only thing allowed to decide. If this
    /// function also reduced, there would be two answers to the same question
    /// and the second one would be the one the report prints — which means the
    /// report would be authoritative by virtue of being last.
    pub fn build(
        target: &ReleaseTarget,
        capability: &str,
        considered: Vec<String>,
        skipped: Vec<SkippedProvider>,
        verdict: VersionAuthority,
        consulted: &[VersionObservation],
    ) -> Self {
        // `consulted`, not the verdict's own observations. The verdict prunes —
        // `CrossValidated` keeps only the sources that agreed — and a report
        // read off a pruned verdict is **more complete the worse the outcome
        // was**. `consulted` is the whole conversation; `verdict` is what came
        // of it. Both are here, and a law in the falsification suite pins that
        // the verdict is exactly `reduce(consulted)`, so the two cannot drift
        // into being two different resolutions of the same target.
        let findings = consulted.iter().map(finding_of).collect::<Vec<_>>();
        let providers_answering = providers_that_answered(consulted);
        let authority = authority_level(&verdict);
        let version = verdict.version().cloned();
        let not_checked = not_checked_for(&verdict, &findings);

        Self {
            target: target.clone(),
            capability: capability.to_owned(),
            providers_considered: considered,
            providers_answering,
            findings,
            skipped,
            authority,
            version,
            verdict,
            not_checked,
        }
    }

    /// Whether this inspection found a usable product version.
    ///
    /// Deliberately narrower than "the release is fine": this is the question
    /// "is there a version", and conflating it with certification is how a
    /// found version turns into a certified release somewhere downstream.
    pub fn found_version(&self) -> bool {
        matches!(
            self.authority,
            AssuranceLevel::Observed | AssuranceLevel::CrossValidated
        )
    }
}

/// Which providers actually **answered**, as opposed to which were asked.
///
/// MEDIDO: this was `observations.map(provider_id)` — every consulted provider,
/// verbatim. So a repository with two declarations and twelve absent build files
/// reported fourteen `providers_answering`, twelve of which had answered
/// «that file is not in this target». A provider that says «I looked, there is
/// nothing here» has not answered the question; it has reported an absence, and
/// the report prints those separately under `consulted_and_found_nothing`.
///
/// [`Invalid`] counts as answering, deliberately. «I found a source and could
/// not read it» is the **most** informative thing a provider can say here — it
/// is what fails the release closed — and dropping it from the list of
/// providers that answered would make the report hardest to read in exactly the
/// case that needs reading.
fn providers_that_answered(consulted: &[VersionObservation]) -> Vec<String> {
    consulted
        .iter()
        .filter(|observation| {
            !matches!(
                observation.probe,
                VersionProbe::NotApplicable { .. } | VersionProbe::Undeclared { .. }
            )
        })
        .map(|observation| observation.provider_id.clone())
        .collect()
}

/// One observation, classified.
///
/// The classification of a `Declared` is the interesting one: **two** providers
/// declaring the same value is `CrossValidated`, and one is `Observed`. Saying
/// "declared" for both would make a single reading look like agreement, which is
/// the exact confusion `cross_validated` exists to prevent.
fn finding_of(observation: &VersionObservation) -> ProviderFinding {
    let level = match &observation.probe {
        VersionProbe::Declared { .. } => AssuranceLevel::Observed,
        VersionProbe::NotApplicable { .. } => AssuranceLevel::NotApplicable,
        VersionProbe::Invalid { .. } => AssuranceLevel::Invalid,
        VersionProbe::Undeclared { .. } | VersionProbe::ReleaseRefIsAuthority { .. } => {
            AssuranceLevel::NotChecked
        }
    };
    let (product_version, location, digest, recheckable) = match &observation.probe {
        VersionProbe::Declared { version, evidence } => (
            Some(version.clone()),
            evidence.location.clone(),
            evidence.digest.clone(),
            evidence.is_recheckable(),
        ),
        _ => (None, None, None, false),
    };
    ProviderFinding {
        provider_id: observation.provider_id.clone(),
        provider_version: observation.provider_version.clone(),
        level,
        product_version,
        location,
        digest,
        recheckable,
        detail: detail_of(&observation.probe),
    }
}

/// The provider's own words, in its own words.
///
/// Not ours: a report that replaces «no encuentro `version` en este fichero»
/// with its own summary is a report that already decided which fichero mattered.
fn detail_of(probe: &VersionProbe) -> String {
    match probe {
        VersionProbe::NotApplicable { reason } => reason.clone(),
        VersionProbe::Undeclared { reason } => reason.clone(),
        VersionProbe::Declared { version, evidence } => format!(
            "{version} declared at {}",
            evidence
                .location
                .as_deref()
                .unwrap_or("a location the provider did not name")
        ),
        VersionProbe::ReleaseRefIsAuthority { declared_by } => {
            format!("declared that the release reference carries the version: {declared_by}")
        }
        VersionProbe::Invalid { reason } => reason.clone(),
    }
}

/// The verdict, as an assurance level.
fn authority_level(verdict: &VersionAuthority) -> AssuranceLevel {
    match verdict {
        VersionAuthority::Resolved { .. } => AssuranceLevel::Observed,
        VersionAuthority::CrossValidated { .. } => AssuranceLevel::CrossValidated,
        VersionAuthority::Ambiguous { .. } => AssuranceLevel::Conflict,
        VersionAuthority::Invalid { .. } => AssuranceLevel::Invalid,
        VersionAuthority::ReleaseRefIsAuthority { .. } | VersionAuthority::Unresolved { .. } => {
            AssuranceLevel::NotChecked
        }
    }
}

/// What nobody checked, **derived from what happened**.
///
/// Four things are always true of this report and worth saying out loud, and
/// one more that depends on the outcome:
///
/// 1. The release reference was not compared to the version here — that is the
///    lockstep rule's question, and this report does not ask it. An agent that
///    reads "resolved" here and concludes the tag was verified has read past
///    what the report said.
/// 2. Nothing was certified. Finding a version is not certifying a release, and
///    a report that does not say this is a report that will be quoted as if it
///    did.
/// 3. The provider set is what SDDK shipped. A tool nobody registered is a
///    source that was never consulted and cannot be counted as agreement.
/// 4. Resolution is read-only, so **no** aspect of the target was modified.
///    That one is not a caveat — it is a property, and it is listed because
///    "nothing was written" is a claim someone will want to verify.
fn not_checked_for(verdict: &VersionAuthority, findings: &[ProviderFinding]) -> Vec<NotChecked> {
    let mut items = vec![
        NotChecked {
            key: "release_reference_not_compared".to_owned(),
            statement: "the release reference was NOT compared to this version; that is the \
                 lockstep rule's question and this report does not ask it"
                .to_owned(),
        },
        NotChecked {
            key: "nothing_certified".to_owned(),
            statement: "nothing was certified: finding a product version is not certifying a \
                 release"
                .to_owned(),
        },
        NotChecked {
            key: "provider_set_is_sddks".to_owned(),
            statement: "the provider set is the one SDDK shipped; a tool nobody registered is \
                 a source that was never consulted"
                .to_owned(),
        },
        NotChecked {
            key: "read_only".to_owned(),
            statement: "resolution wrote nothing: the target is exactly as it was read".to_owned(),
        },
    ];

    // El quinto depende de lo que pasó, y por eso es el que se puede mentir sin
    // que nadie lo note: si declaro una sola fuente, «nadie mas confirmo» es un
    // hecho, no una cautela.
    let declaradores = findings
        .iter()
        .filter(|f| f.level == AssuranceLevel::Observed)
        .count();
    if declaradores <= 1 && matches!(verdict, VersionAuthority::Resolved { .. }) {
        items.insert(
            0,
            NotChecked {
                key: "no_independent_second_source".to_owned(),
                statement: "only one source declared this value; nothing corroborated it from \
                     an independent second source"
                    .to_owned(),
            },
        );
    }
    // El sexto depende de lo que pasó, y es el que nadie mira: una declaracion
    // sin nada que volver a mirar es una afirmacion, y una afirmacion no es una
    // comprobacion. MEDIDO: el unico provider que emite evidencia lo hace con
    // `digest: None` y `location` presente, y **nada lo exigia** — un provider
    // nuevo podia declarar sin las dos cosas y el informe no lo decia. No se
    // rechaza, porque el nucleo no tiene autoridad para llamar deshonesto a un
    // provider que perhaps es solo breve; se declara, porque eso si es un hecho.
    let sin_revisar = findings
        .iter()
        .filter(|f| f.level == AssuranceLevel::Observed && !f.recheckable)
        .count();
    if sin_revisar > 0 {
        items.insert(
            0,
            NotChecked {
                key: "declared_without_recheckable_evidence".to_owned(),
                statement: format!(
                    "{sin_revisar} declared value(s) carry no digest and no location, so \
                     the value cannot be re-checked from what the provider said"
                ),
            },
        );
    }
    items
}

/// The capability this module reports on, re-exported so a caller that is
/// building a report does not have to import two modules to name the same
/// string twice.
pub const INSPECTION_CAPABILITY: &str = PRODUCT_VERSION_OBSERVATION;
