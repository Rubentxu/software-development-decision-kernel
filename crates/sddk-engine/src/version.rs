//! Version module — exposes the crate version as a stable compile-time constant
//! and the workspace-vs-tag lockstep rule used by the release pipeline.
//!
//! The crate's own version is a compile-time constant —see [`version`]— and it
//! is not what the lockstep rule is about. That rule asks a different question:
//! what version does the **product being released** declare, and does the
//! release reference name it?
//!
//! The lockstep rule (`ensure_version_lockstep`) was extracted from
//! `sddk-cli::release_cmd` per INC-024 (god-class smell at 906 LOC; now ~1820).
//! Keeping the lockstep check in the engine substrate makes it reusable from
//! any caller (CLI today; future daemon / CI gate tomorrow).
//!
//! # What this module knows, and what it stopped knowing
//!
//! It knows one thing about the shape of a release reference: it may carry a
//! leading `v`. It does not know where the product's version comes from, which
//! manifests exist, or what any of them are called. Those questions are asked
//! of a [`VersionResolverRegistry`] the caller supplies, and the answers are
//! reduced by the kernel's pure model
//! ([`sddk_domain::version_authority::reduce`]) before this module ever sees
//! them.
//!
//! That split is why the rules below can be tested with three lines of setup
//! instead of a temporary directory: a lockstep question has no business
//! needing a filesystem.

use sddk_domain::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ReleaseTarget, VersionAuthority, VersionObservation,
    VersionResolverRegistry,
};

/// Returns the SDDK engine version string (e.g. `"1.42.5"`).
///
/// Read at compile time from this crate's own declared version, which for a
/// workspace member resolves to the workspace's. That is the version of **the
/// tool**, not of the product a release publishes, and nothing in this module
/// compares the two: doing so is the defect this module's rule exists to
/// prevent.
///
/// This function is useful for runtime version reporting where a `&'static str`
/// is needed rather than the const value.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Typed error for version lockstep failures — names both workspace and tag versions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VersionLockstepError {
    pub workspace_version: String,
    pub tag_version: String,
    pub message: String,
}

impl std::fmt::Display for VersionLockstepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VersionLockstepError {}

/// The one release-reference shape convention this module has.
///
/// A leading `v` is stripped, and **nothing else is**. That is the whole
/// function on purpose: it is a declared convention rather than a coercion,
/// and a coercion is the thing that lets two different values be made to look
/// like one. No suffix is trimmed, no pattern is matched, and a value that
/// differs by anything other than this prefix fails and says so.
///
/// ## Why it is written down rather than done properly yet
///
/// Stating a relation between a product version and a release reference is
/// real work — a `ReleaseRef` that can be `v1.2.3-rc2` on a channel, with the
/// relation between it and the product version declared rather than guessed.
/// Until that exists, the honest thing is one narrow, visible convention with
/// a comment saying where it came from, so that nobody adds a second one next
/// to it later.
pub fn product_version_of_release_ref(release_ref: &str) -> &str {
    release_ref.strip_prefix('v').unwrap_or(release_ref)
}

/// Ensures the release tag matches the product's declared version.
///
/// The lockstep rule: the product's version and the release reference name the
/// same thing.
///
/// A project that declares **no** product version, and says so —because its
/// ecosystem keeps the version on the release reference, or because the
/// project wrote that down— yields
/// [`VersionAuthority::ReleaseRefIsAuthority`]: there was nothing to compare,
/// and the verdict says so instead of reporting a check that never happened.
/// A target where nobody declared anything fails closed, because silence is
/// not a declaration.
pub fn ensure_version_lockstep(
    registry: &VersionResolverRegistry,
    target: &ReleaseTarget,
    release_ref: &str,
) -> Result<(), VersionLockstepError> {
    ensure_version_lockstep_detailed(registry, target, release_ref).map(|_| ())
}

/// Same rule, but reports **where the version came from**.
///
/// The difference matters: an authority that was cross-validated by two
/// independent sources, one that resolved from a single declaration, and one
/// that holds no product version at all are three different facts, and a
/// caller that cannot tell them apart will read the last as the first.
pub fn ensure_version_lockstep_detailed(
    registry: &VersionResolverRegistry,
    target: &ReleaseTarget,
    release_ref: &str,
) -> Result<VersionAuthority, VersionLockstepError> {
    let tag_version = product_version_of_release_ref(release_ref).to_string();
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, target);

    if let Some(message) = refusal(&authority, target) {
        return Err(VersionLockstepError {
            workspace_version: String::new(),
            tag_version,
            message,
        });
    }

    let Some(product_version) = authority.version() else {
        return Ok(authority);
    };

    if product_version.as_str() != tag_version {
        let msg = format!(
            "VERSION LOCKSTEP FAILED: project={} vs tag={}. \
             Release planning refused until the lockstep rule is satisfied.",
            product_version, tag_version
        );
        return Err(VersionLockstepError {
            workspace_version: product_version.to_string(),
            tag_version,
            message: msg,
        });
    }
    // Kept explicit: the two arms return different values, and collapsing them
    // here is how the distinction between "checked" and "nothing to check"
    // would be lost.
    Ok(authority)
}

/// The refusal message for an authority that cannot authorise a release, or
/// `None` when it can.
///
/// ## Where the message comes from, and why
///
/// This used to be a `match` over the engine's own error enum, with the list
/// of files it had searched written into the message. That list was a list of
/// **file names**, so it could only be produced by a crate that knew them.
/// Now the list comes from the observations: every provider that said "not my
/// subject" said so in its own words, naming what it looked for. The operator
/// still gets the same information, and this crate still cannot name a single
/// technology.
///
/// Two failure shapes are deliberately different messages, and that difference
/// is the point: a target where nothing was found and a target where something
/// was found and could not be read are two problems with opposite fixes, and
/// the version that collapsed them is the substitution a prior falsifier
/// exploited on this exact code path.
fn refusal(authority: &VersionAuthority, target: &ReleaseTarget) -> Option<String> {
    let observations = observations_of(authority);
    match authority {
        VersionAuthority::Resolved { .. } | VersionAuthority::CrossValidated { .. } => None,
        // No hay version que comparar, pero el target se ha resuelto y ha
        // declarado por que. El mensaje lo compone quien llama, que es quien
        // sabe anadir la ayuda sin que este crate nombre un fichero.
        VersionAuthority::ReleaseRefIsAuthority { .. } => None,
        VersionAuthority::Ambiguous { candidates, .. } => {
            let detail = candidates
                .iter()
                .map(|(who, version)| format!("{who}={version}"))
                .collect::<Vec<_>>()
                .join("; ");
            Some(format!(
                "VERSION LOCKSTEP ERROR: {} declarations disagree about the version: {detail}. \
                 Which one is authoritative is a human decision, and sddk will not pick one \
                 (observations: {}).",
                candidates.len(),
                summarise(observations)
            ))
        }
        VersionAuthority::Invalid { failures, .. } => Some(format!(
            "VERSION LOCKSTEP ERROR: {} source(s) could not be read, so the version cannot be \
             trusted: {}. Failing closed: a source that exists and cannot be understood is not \
             the same as a source that is absent.",
            failures.len(),
            failures
                .iter()
                .map(|(who, why)| format!("{who}: {why}"))
                .collect::<Vec<_>>()
                .join("; ")
        )),
        VersionAuthority::Unresolved { .. } => {
            let silent: Vec<String> = observations
                .iter()
                .filter(|o| {
                    matches!(
                        o.probe,
                        sddk_domain::version_authority::VersionProbe::Undeclared { .. }
                    )
                })
                .map(describe)
                .collect();
            let searched: Vec<String> = observations
                .iter()
                .filter(|o| {
                    matches!(
                        o.probe,
                        sddk_domain::version_authority::VersionProbe::NotApplicable { .. }
                    )
                })
                .map(describe)
                .collect();
            let present_but_undeclared = if silent.is_empty() {
                String::new()
            } else {
                format!(" (presentes-pero-sin-declarar: {})", silent.join(", "))
            };
            Some(format!(
                "VERSION LOCKSTEP ERROR: no version declared for target `{}` under {}. \
                 {} provider(s) were asked{present_but_undeclared}; searched: {}.",
                target.id(),
                target.root(),
                observations.len(),
                searched.join(", ")
            ))
        }
    }
}

fn observations_of(authority: &VersionAuthority) -> &[VersionObservation] {
    match authority {
        VersionAuthority::Resolved { observations, .. }
        | VersionAuthority::CrossValidated { observations, .. }
        | VersionAuthority::Ambiguous { observations, .. }
        | VersionAuthority::Invalid { observations, .. }
        | VersionAuthority::Unresolved { observations }
        | VersionAuthority::ReleaseRefIsAuthority { observations, .. } => observations,
    }
}

fn describe(observation: &VersionObservation) -> String {
    match &observation.probe {
        sddk_domain::version_authority::VersionProbe::NotApplicable { reason }
        | sddk_domain::version_authority::VersionProbe::Undeclared { reason } => reason.clone(),
        other => format!("{other:?}"),
    }
}

fn summarise(observations: &[VersionObservation]) -> String {
    observations
        .iter()
        .map(|o| format!("{}={}", o.provider_id, o.summary()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::version_authority::{
        ProductVersion, ProviderError, VersionEvidence, VersionProbe, VersionResolverPort,
    };

    /// A provider that says whatever the test tells it to.
    ///
    /// It exists because the lockstep rule never needed a filesystem and was
    /// only ever tested through one. A provider is three lines here and the
    /// whole rule is testable without a temporary directory.
    struct Fixed {
        id: &'static str,
        probe: VersionProbe,
    }

    impl Fixed {
        fn declaring(id: &'static str, version: &str) -> Self {
            Self {
                id,
                probe: VersionProbe::Declared {
                    version: ProductVersion::new(version).expect("version valida"),
                    evidence: VersionEvidence {
                        source_kind: "test".to_owned(),
                        digest: None,
                        location: None,
                    },
                },
            }
        }
    }

    impl VersionResolverPort for Fixed {
        fn provider_id(&self) -> &str {
            self.id
        }
        fn provider_version(&self) -> &str {
            "test"
        }
        fn capabilities(&self) -> &[String] {
            // One allocation per call, on a test-only type.
            Box::leak(vec![PRODUCT_VERSION_OBSERVATION.to_owned()].into_boxed_slice())
        }
        fn observe(&self, _target: &ReleaseTarget) -> Result<VersionProbe, ProviderError> {
            Ok(self.probe.clone())
        }
    }

    fn registry_of(providers: Vec<Fixed>) -> VersionResolverRegistry {
        let mut registry = VersionResolverRegistry::new();
        for p in providers {
            registry.register(Box::new(p));
        }
        registry
    }

    fn target() -> ReleaseTarget {
        ReleaseTarget::at("producto", "/tmp/un-repositorio")
    }

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn version_matches_cargo_pkg_version() {
        // CARGO_PKG_VERSION is set at compile time; this test verifies the
        // const and the macro resolve to the same value.
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn lockstep_passes_when_the_ref_matches() {
        let registry = registry_of(vec![Fixed::declaring("p", "1.42.5")]);
        ensure_version_lockstep(&registry, &target(), "v1.42.5").expect("coinciden");
    }

    #[test]
    fn lockstep_passes_when_the_ref_has_no_v_prefix() {
        let registry = registry_of(vec![Fixed::declaring("p", "1.42.5")]);
        ensure_version_lockstep(&registry, &target(), "1.42.5").expect("sin prefijo");
    }

    #[test]
    fn lockstep_fails_and_names_both_sides() {
        let registry = registry_of(vec![Fixed::declaring("p", "1.42.5")]);
        let err = ensure_version_lockstep(&registry, &target(), "v1.99.0").unwrap_err();
        assert_eq!(err.workspace_version, "1.42.5");
        assert_eq!(err.tag_version, "1.99.0");
        assert!(err.message.contains("LOCKSTEP FAILED"), "{}", err.message);
    }

    /// The convention is un `v` y NADA MAS.
    ///
    /// MEDIDO como el riesgo que es: un recorte mas —una `v` por dentro, un
    /// sufijo de candidata, un `=`— haria que dos valores distintos se
    /// presentaran como el mismo, y eso no es una comprobacion laxa: es una
    /// release autorizada sobre un numero que no es el del proyecto.
    #[test]
    fn la_convencion_no_recorta_nada_mas_que_el_prefijo() {
        let registry = registry_of(vec![Fixed::declaring("p", "1.42.5")]);
        for tag in [
            "v1.42.5-rc2",
            "release-1.42.5",
            "1.42.5+build",
            "vv1.42.5",
            "v 1.42.5",
        ] {
            let err = ensure_version_lockstep(&registry, &target(), tag)
                .expect_err("una referencia que NO es la version no puede autorizar un release");
            assert!(
                err.message.contains("LOCKSTEP FAILED"),
                "«{tag}» deberia fallar por lockstep, no por otra cosa: {}",
                err.message
            );
        }
    }

    /// Un target que declara que su version la lleva la release ref pasa, y
    /// devuelve un veredicto que lo dice. No es un verde silencioso: el
    /// veredicto no tiene version y no esta cross-validado.
    #[test]
    fn una_ausencia_declarada_pasa_y_lo_dice() {
        let registry = registry_of(vec![Fixed {
            id: "p",
            probe: VersionProbe::ReleaseRefIsAuthority {
                declared_by: "su convencion lo lleva la release ref".to_owned(),
            },
        }]);
        let authority = ensure_version_lockstep_detailed(&registry, &target(), "v1.42.5")
            .expect("no hay version que comparar");
        assert!(authority.version().is_none());
        assert!(!authority.was_cross_validated());
        assert!(!authority.is_failure());
        assert_eq!(
            authority.release_ref_declarations(),
            ["su convencion lo lleva la release ref"]
        );
    }

    /// Y el silencio no es lo mismo: sin declaracion, falla cerrado.
    #[test]
    fn el_silencio_no_es_una_declaracion() {
        let registry = registry_of(vec![Fixed {
            id: "p",
            probe: VersionProbe::NotApplicable {
                reason: "nada aqui que mirar".to_owned(),
            },
        }]);
        let err = ensure_version_lockstep(&registry, &target(), "v1.42.5").unwrap_err();
        assert!(
            err.message.contains("no version declared for target"),
            "{}",
            err.message
        );
        assert!(
            err.message.contains("nada aqui que mirar"),
            "el mensaje tiene que decir que se busco: {}",
            err.message
        );
    }

    /// Un fichero presente que no declara tampoco es una declaracion.
    #[test]
    fn un_fichero_que_no_declara_no_abre_la_puerta() {
        let registry = registry_of(vec![Fixed {
            id: "p",
            probe: VersionProbe::Undeclared {
                reason: "existe y no declara version".to_owned(),
            },
        }]);
        let err = ensure_version_lockstep(&registry, &target(), "v1.42.5").unwrap_err();
        assert!(
            err.message.contains("presentes-pero-sin-declarar"),
            "el mensaje tiene que distinguirlo de «no hay nada»: {}",
            err.message
        );
    }

    /// Dos fuentes que discrepan: se nombran las dos y no se elige.
    #[test]
    fn una_discrepancia_no_se_resuelve_eligiendo() {
        let registry = registry_of(vec![
            Fixed::declaring("p-a", "1.0.0"),
            Fixed::declaring("p-b", "2.0.0"),
        ]);
        let err = ensure_version_lockstep(&registry, &target(), "v1.0.0").unwrap_err();
        assert!(
            err.message.contains("declarations disagree"),
            "{}",
            err.message
        );
        assert!(err.message.contains("p-a=1.0.0"), "{}", err.message);
        assert!(err.message.contains("p-b=2.0.0"), "{}", err.message);
    }

    /// Dos fuentes que coinciden SI se cruzaron, y el veredicto lo dice.
    #[test]
    fn dos_fuentes_que_coinciden_se_cruzan() {
        let registry = registry_of(vec![
            Fixed::declaring("p-a", "1.0.0"),
            Fixed::declaring("p-b", "1.0.0"),
        ]);
        let authority =
            ensure_version_lockstep_detailed(&registry, &target(), "v1.0.0").expect("coinciden");
        assert!(authority.was_cross_validated(), "{authority:?}");
    }

    /// Un provider que no pudo responder no es evidencia de que no haya
    /// respuesta: falla cerrado, y dice cual fue el provider y por que.
    #[test]
    fn un_provider_que_no_pudo_responder_falla_cerrado_y_se_nombra() {
        struct Broken;
        impl VersionResolverPort for Broken {
            fn provider_id(&self) -> &str {
                "p-roto"
            }
            fn provider_version(&self) -> &str {
                "test"
            }
            fn capabilities(&self) -> &[String] {
                Box::leak(vec![PRODUCT_VERSION_OBSERVATION.to_owned()].into_boxed_slice())
            }
            fn observe(&self, _t: &ReleaseTarget) -> Result<VersionProbe, ProviderError> {
                Err(ProviderError::Unavailable {
                    provider_id: "p-roto".to_owned(),
                    reason: "el disco no estaba".to_owned(),
                })
            }
        }
        let mut registry = VersionResolverRegistry::new();
        registry.register(Box::new(Broken));
        let err = ensure_version_lockstep(&registry, &target(), "v1.0.0").unwrap_err();
        assert!(err.message.contains("p-roto"), "{}", err.message);
        assert!(
            err.message.contains("el disco no estaba"),
            "el motivo del provider no puede perderse en el hueco: {}",
            err.message
        );
    }
}
