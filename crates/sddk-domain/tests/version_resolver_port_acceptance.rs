//! Acceptance for the version resolver port: a new mechanism added WITHOUT
//! touching the engine.
//!
//! # What has to be demonstrated
//!
//! The whole point of the port is that a way of learning a version is added
//! by implementing a trait and registering it. So this file registers a
//! provider the engine has never heard of, and requires that:
//!
//! - it is discovered;
//! - it can answer `NotApplicable`;
//! - it can answer `Declared`;
//! - it can answer `Invalid`;
//! - it resolves together with another provider;
//! - and no decision anywhere depends on its identifier.
//!
//! # Why the identifiers here are deliberately unnamed
//!
//! The fake providers are called `probe-a`, `probe-b`. If a rule ever keyed
//! off a provider's name, these tests would be the ones to notice — and a
//! test suite whose fixtures are named after the technology would be unable
//! to notice, because every name in it would look legitimate.

use sddk_domain::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ProductVersion, ProviderError, ReleaseTarget, VersionAuthority,
    VersionEvidence, VersionObservation, VersionProbe, VersionResolverPort,
    VersionResolverRegistry, reduce,
};

fn v(value: &str) -> ProductVersion {
    ProductVersion::new(value).expect("version valida en el test")
}

fn evidence(source_kind: &str) -> VersionEvidence {
    VersionEvidence {
        source_kind: source_kind.to_owned(),
        digest: Some(format!("sha256:{}", "a".repeat(64))),
        location: Some(".".to_owned()),
    }
}

/// What a programmable provider answers when asked about a target.
type Answer = Box<dyn Fn(&ReleaseTarget) -> Result<VersionProbe, ProviderError> + Send + Sync>;

/// A provider that answers whatever it is told to. It has no idea what it is
/// observing, which is the point: the port carries no technology.
struct Programmable {
    id: String,
    capabilities: Vec<String>,
    answer: Answer,
}

impl Programmable {
    fn new(
        id: &str,
        answer: impl Fn(&ReleaseTarget) -> Result<VersionProbe, ProviderError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            id: id.to_owned(),
            capabilities: vec![PRODUCT_VERSION_OBSERVATION.to_owned()],
            answer: Box::new(answer),
        }
    }

    fn without_capability(mut self) -> Self {
        self.capabilities = vec!["otra.cosa/v1".to_owned()];
        self
    }
}

impl VersionResolverPort for Programmable {
    fn provider_id(&self) -> &str {
        &self.id
    }
    fn provider_version(&self) -> &str {
        "0.1.0"
    }
    fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
    fn observe(&self, target: &ReleaseTarget) -> Result<VersionProbe, ProviderError> {
        (self.answer)(target)
    }
}

fn target() -> ReleaseTarget {
    ReleaseTarget::at_root("runtime")
}

// ---------------------------------------------------------------------------
// Acceptance
// ---------------------------------------------------------------------------

#[test]
fn un_provider_nuevo_se_descubre_sin_tocar_el_engine() {
    let mut registry = VersionResolverRegistry::new();
    assert!(registry.is_empty());

    registry.register(Box::new(Programmable::new("probe-a", |_| {
        Ok(VersionProbe::Declared {
            version: v("0.47.0"),
            evidence: evidence("fuente-opaca"),
        })
    })));

    assert_eq!(registry.len(), 1);
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert_eq!(authority.version(), Some(&v("0.47.0")), "{authority:?}");
}

#[test]
fn puede_responder_not_applicable() {
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Programmable::new("probe-a", |_| {
        Ok(VersionProbe::NotApplicable {
            reason: "este target no es asunto mio".to_owned(),
        })
    })));
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert!(
        matches!(authority, VersionAuthority::Unresolved { .. }),
        "{authority:?}"
    );
}

#[test]
fn puede_responder_invalid_y_eso_falla_cerrado() {
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Programmable::new("probe-a", |_| {
        Ok(VersionProbe::Invalid {
            reason: "existe algo y no se pudo leer".to_owned(),
        })
    })));
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert!(
        matches!(authority, VersionAuthority::Invalid { .. }),
        "{authority:?}"
    );
}

#[test]
fn un_provider_que_falla_al_ejecutarse_tambien_falla_cerrado() {
    // El provider no pudo responder. Eso NO es «el target no declara nada»:
    // es un provider que no houbo, y la ley dice que la falta de informacion
    // no se convierte en una version.
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Programmable::new("probe-a", |_| {
        Err(ProviderError::Unavailable {
            provider_id: "probe-a".to_owned(),
            reason: "no me puedo ejecutar".to_owned(),
        })
    })));
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert!(
        matches!(authority, VersionAuthority::Invalid { .. }),
        "{authority:?}"
    );
    assert_eq!(authority.version(), None);
}

#[test]
fn participa_en_la_resolucion_con_otro_provider() {
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Programmable::new("probe-a", |_| {
        Ok(VersionProbe::Declared {
            version: v("1.0.0"),
            evidence: evidence("a"),
        })
    })));
    registry.register(Box::new(Programmable::new("probe-b", |_| {
        Ok(VersionProbe::Declared {
            version: v("1.0.0"),
            evidence: evidence("b"),
        })
    })));
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert!(authority.was_cross_validated(), "{authority:?}");
}

#[test]
fn dos_providers_que_difieren_no_se_elige_uno() {
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Programmable::new("probe-a", |_| {
        Ok(VersionProbe::Declared {
            version: v("1.0.0"),
            evidence: evidence("a"),
        })
    })));
    registry.register(Box::new(Programmable::new("probe-b", |_| {
        Ok(VersionProbe::Declared {
            version: v("2.0.0"),
            evidence: evidence("b"),
        })
    })));
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert!(
        matches!(authority, VersionAuthority::Ambiguous { .. }),
        "{authority:?}"
    );
    assert_eq!(authority.version(), None);
}

#[test]
fn un_provider_que_no_habla_la_capability_no_se_pregunta() {
    // Negativa por conteo, no por mensaje: si se le preguntara, responderia
    // con una version y este test pasaria sinmirar nada.
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(
        Programmable::new("probe-ajeno", |_| {
            panic!("este provider no deberia consultarse")
        })
        .without_capability(),
    ));
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert!(
        matches!(authority, VersionAuthority::Unresolved { .. }),
        "{authority:?}"
    );
}

#[test]
fn un_target_recibido_llega_al_provider_intacto() {
    // El target es lo unico que SDDK controla y lo unico que el provider ve.
    // Si el registry lo reescribiera, un provider no podria ni situated.
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Programmable::new("probe-a", |t| {
        assert_eq!(t.id(), "anidado");
        assert_eq!(t.root(), "packages/ajeno");
        Ok(VersionProbe::Declared {
            version: v("5.0.0"),
            evidence: evidence("a"),
        })
    })));
    let authority = registry.resolve(
        PRODUCT_VERSION_OBSERVATION,
        &ReleaseTarget::at("anidado", "packages/ajeno"),
    );
    assert_eq!(authority.version(), Some(&v("5.0.0")));
}

#[test]
fn ninguna_decision_depende_del_identificador_del_provider() {
    // El MISMO comportamiento con los identificadores invertidos. Si algo
    // del registry o del reducer leyera el nombre, en una de las dos saldría un Resolved.
    fn resolve_with_ids(ids: (&str, &str)) -> VersionAuthority {
        let mut registry = VersionResolverRegistry::new();
        registry.register(Box::new(Programmable::new(ids.0, |_| {
            Ok(VersionProbe::Declared {
                version: v("1.0.0"),
                evidence: evidence("a"),
            })
        })));
        registry.register(Box::new(Programmable::new(ids.1, |_| {
            Ok(VersionProbe::Declared {
                version: v("9.9.9"),
                evidence: evidence("b"),
            })
        })));
        registry.resolve(PRODUCT_VERSION_OBSERVATION, &target())
    }

    // El MISMO conflicto, con los identificadores en un orden y en el otro.
    // Si algo del registry o del reducer leyera el nombre, en uno de los dos
    // saldría un `Resolved`.
    //
    // Y por qué NO se comparan las dos autoridades enteras: `Ambiguous` nombra
    // QUIEN declaro cada valor, así que exchanging los identificadores cambia
    // el informe de forma legítima. La propiedad es que en ninguna
    // disposición se elige una, no que los informes sean idénticos. Una
    // primera versión de este test comparaba las autoridades enteras y fallaba
    // por eso — el fallo era del test, no del código.
    let a = resolve_with_ids(("aaa", "zzz"));
    let b = resolve_with_ids(("zzz", "aaa"));
    assert!(matches!(a, VersionAuthority::Ambiguous { .. }), "{a:?}");
    assert!(matches!(b, VersionAuthority::Ambiguous { .. }), "{b:?}");
    assert_eq!(a.version(), None);
    assert_eq!(b.version(), None);
}

#[test]
fn un_registro_vacio_es_unresolved_y_no_un_default() {
    let registry = VersionResolverRegistry::new();
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target());
    assert_eq!(authority.version(), None, "{authority:?}");
    assert!(matches!(authority, VersionAuthority::Unresolved { .. }));
}

/// El reducer del dominio y el que usa el registry tienen que ser el MISMO.
/// Si el registry tuviera su propia copia de la reduccion, el nucleo tendria
/// dos autoridades y la segunda no la mediria nadie.
#[test]
fn el_registry_reduce_con_el_reducer_del_dominio() {
    let observations = vec![VersionObservation {
        provider_id: "probe-a".to_owned(),
        provider_version: "0.1.0".to_owned(),
        capability: PRODUCT_VERSION_OBSERVATION.to_owned(),
        probe: VersionProbe::Declared {
            version: v("1.0.0"),
            evidence: evidence("a"),
        },
    }];
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Programmable::new("probe-a", |_| {
        Ok(VersionProbe::Declared {
            version: v("1.0.0"),
            evidence: evidence("a"),
        })
    })));
    assert_eq!(
        registry.resolve(PRODUCT_VERSION_OBSERVATION, &target()),
        reduce(observations)
    );
}
