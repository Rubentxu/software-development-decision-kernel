//! Acceptance for target selection: which product, when a repository holds
//! more than one.
//!
//! # The property under test
//!
//! A repository is not a product. This file pins the decision that follows
//! from taking that seriously: **asked about two products with nothing to tell
//! them apart, the tool says there are two and stops.** Every case here is a
//! different way that sentence can be weakened, and the falsifier next door
//! mutates each one back.
//!
//! The second property is structural and is the reason the selection lives
//! outside the reducer: **two targets never see each other's observations.**
//! Two products with different versions is the normal shape of a monorepo, not
//! a contradiction, and the only way to keep it that way is to make it
//! impossible for one target's evidence to reach another's decision.

use sddk_domain::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ProductVersion, ProviderError, ReleaseTarget,
    TargetSelectionError, TargetSelector, VersionEvidence, VersionProbe, VersionResolverPort,
    VersionResolverRegistry, select_target,
};

fn target(id: &str, root: &str) -> ReleaseTarget {
    ReleaseTarget::at(id, root)
}

fn targets(ids: &[(&str, &str)]) -> Vec<ReleaseTarget> {
    ids.iter().map(|(id, root)| target(id, root)).collect()
}

/// A provider that reads a version out of a table, keyed by the target's root.
///
/// Keyed rather than fixed on purpose: a provider that returned the same
/// answer whatever it was asked about could not tell one target from another,
/// and the isolation test would pass without anything having been isolated.
struct Served {
    answers: Vec<(String, String)>,
}

impl VersionResolverPort for Served {
    fn provider_id(&self) -> &str {
        "test/served"
    }
    fn provider_version(&self) -> &str {
        "test"
    }
    fn capabilities(&self) -> &[String] {
        Box::leak(vec![PRODUCT_VERSION_OBSERVATION.to_owned()].into_boxed_slice())
    }
    fn observe(&self, target: &ReleaseTarget) -> Result<VersionProbe, ProviderError> {
        match self.answers.iter().find(|(root, _)| root == target.root()) {
            Some((_, version)) => Ok(VersionProbe::Declared {
                version: ProductVersion::new(version).expect("version valida"),
                evidence: VersionEvidence {
                    source_kind: "test".to_owned(),
                    digest: None,
                    location: Some(target.root().to_owned()),
                },
            }),
            None => Ok(VersionProbe::NotApplicable {
                reason: format!("no se decia nada en {}", target.root()),
            }),
        }
    }
}

fn registry_of(answers: &[(&str, &str)]) -> VersionResolverRegistry {
    let mut registry = VersionResolverRegistry::new();
    registry.register(Box::new(Served {
        answers: answers
            .iter()
            .map(|(root, version)| ((*root).to_owned(), (*version).to_owned()))
            .collect(),
    }));
    registry
}

// ---------------------------------------------------------------------------
// Un target, sin pedir nada
// ---------------------------------------------------------------------------

/// Un unico target no es una adivinanza: es la unica respuesta posible.
#[test]
fn un_unico_target_se_resuelve_sin_preguntar() {
    let available = targets(&[("runtime", "packages/runtime")]);
    let chosen = select_target(&available, &TargetSelector::unsolicited()).expect("hay uno");
    assert_eq!(chosen.id(), "runtime");
}

/// Cero targets no es un conflicto: no hay sujeto.
#[test]
fn cero_targets_no_es_ambiguedad() {
    let err = select_target(&[], &TargetSelector::unsolicited()).unwrap_err();
    assert_eq!(err, TargetSelectionError::NoTarget);
    assert!(
        !err.is_ambiguous(),
        "«no hay ninguno» y «hay varios» son dos reparaciones distintas: {err:?}"
    );
    assert!(err.candidates().is_empty());
}

// ---------------------------------------------------------------------------
// Varios targets, sin pedir nada
// ---------------------------------------------------------------------------

/// El caso que el exit gate pide: dos productos y nada que diga cual, cerrado.
///
/// Lo que se afirma es la negativa: que no hay version. Un plan que sale con
/// una version aqui seria un release autorizado sobre un producto que nadie
/// eligio.
#[test]
fn dos_targets_sin_selector_fallan_cerrado() {
    let available = targets(&[("alpha", "packages/alpha"), ("beta", "packages/beta")]);
    let err = select_target(&available, &TargetSelector::unsolicited()).unwrap_err();
    assert!(err.is_ambiguous(), "{err:?}");
    assert_eq!(
        err.candidates(),
        ["alpha", "beta"],
        "el error tiene que NOMBRAR a los dos, porque elegir es del operador: {err:?}"
    );
}

/// Y el orden en que se listaron no cambia ni el veredicto ni la lista.
///
/// Un `select_target` que ante dos candidatos devuelve el primero no es una
/// politica de la que alguien pueda depender: es el orden de un `read_dir`, que
/// no es un orden. Y un error cuyas variantes cambian al reordenar la lista no
/// se puede comparar con nada, luego tampoco puede comprobarse.
///
/// MEDIDO: este test fallo en su primera pasada porque la lista de candidatos
/// se construia en orden de llegada. El fallo no era del test: era un hueco
/// en la funcion, del mismo tipo que el que el orden canonico del reducer ya
/// habia cubierto y que aqui faltaba.
#[test]
fn el_orden_de_los_candidatos_no_decide() {
    let one_way = targets(&[("alpha", "packages/alpha"), ("beta", "packages/beta")]);
    let other_way = targets(&[("beta", "packages/beta"), ("alpha", "packages/alpha")]);
    let a = select_target(&one_way, &TargetSelector::unsolicited()).unwrap_err();
    let b = select_target(&other_way, &TargetSelector::unsolicited()).unwrap_err();
    assert_eq!(
        a.candidates(),
        b.candidates(),
        "los mismos dos productos en distinto orden tienen que ser el mismo error"
    );
    assert!(a.is_ambiguous() && b.is_ambiguous());
}

// ---------------------------------------------------------------------------
// Varios targets, con uno nombrado
// ---------------------------------------------------------------------------

/// Pedir uno lo resuelve, y solo ese.
#[test]
fn un_target_nombrado_se_resuelve_entre_varios() {
    let available = targets(&[
        ("alpha", "packages/alpha"),
        ("beta", "packages/beta"),
        ("gamma", "packages/gamma"),
    ]);
    let chosen = select_target(&available, &TargetSelector::named("beta")).expect("esta nombrado");
    assert_eq!(chosen.id(), "beta");
}

/// Un nombre que no existe NO se resuelve con «el unico que hay».
///
/// Es el defecto mas tentador de todos: el nombre se acepta, no casa con nada,
/// y el codigo devuelve el unico target que hay «porque total no hay otro».
/// Convierte el nombre en decoracion, y un operador que escribe mal el nombre
/// de un producto en un monorepo recibe la version de otro sin que nada lo
/// diga.
#[test]
fn un_nombre_que_no_existe_no_cae_al_unico_que_hay() {
    let available = targets(&[("runtime", "packages/runtime")]);
    let err = select_target(&available, &TargetSelector::named("runtine")).unwrap_err();
    assert!(!err.is_ambiguous(), "{err:?}");
    match &err {
        TargetSelectionError::UnknownTarget {
            requested,
            available,
        } => {
            assert_eq!(requested, "runtine");
            assert_eq!(
                available.as_slice(),
                ["runtime".to_owned()],
                "el error lista lo que SI hay: corregir es mecanico"
            );
        }
        other => panic!("un nombre que no casa no puede devolver el unico target: {other:?}"),
    }
}

/// Dos targets con la MISMA identidad no se resuelven eligiendo uno.
#[test]
fn una_identidad_repetida_es_ambiguedad_y_no_orden() {
    let available = targets(&[("runtime", "packages/a"), ("runtime", "packages/b")]);
    let err = select_target(&available, &TargetSelector::named("runtime")).unwrap_err();
    assert!(
        err.is_ambiguous(),
        "dos directorios con la misma identidad son una colision real: {err:?}"
    );
    assert_eq!(err.candidates(), ["runtime", "runtime"]);
}

// ---------------------------------------------------------------------------
// La propiedad estructural: dos targets no se ven
// ---------------------------------------------------------------------------

/// Dos productos con versiones distintas NO son un conflicto.
///
/// No es una afirmacion de que «se podria resolver»: es la afirmacion de que
/// la estructura lo hace imposible. Un unico registro resuelve cada target
/// por separado, y las observaciones de uno nunca llegan a la reduccion del
/// otro — luego no hay nada que «no choquen», porque no hay un unico
/// veredicto donde puedan encontrarse.
#[test]
fn dos_targets_con_versiones_distintas_no_se_confunden() {
    let available = targets(&[("alpha", "packages/alpha"), ("beta", "packages/beta")]);
    let registry = registry_of(&[("packages/alpha", "1.0.0"), ("packages/beta", "2.0.0")]);

    for (id, expected) in [("alpha", "1.0.0"), ("beta", "2.0.0")] {
        let chosen = select_target(&available, &TargetSelector::named(id)).expect("nombrado");
        let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, chosen);
        assert_eq!(
            authority.version().map(|v| v.to_string()),
            Some(expected.to_owned()),
            "el target {id} resolvio otra version: {authority:?}"
        );
        // Y la prueba de que no hubo mezcla: cada observacion viene del root
        // que se le pidio.
        for observation in match &authority {
            sddk_domain::version_authority::VersionAuthority::Resolved { observations, .. }
            | sddk_domain::version_authority::VersionAuthority::CrossValidated {
                observations,
                ..
            } => observations,
            other => panic!("{id} no resolvio: {other:?}"),
        } {
            let location = match &observation.probe {
                VersionProbe::Declared { evidence, .. } => evidence.location.clone(),
                other => panic!("observacion inesperada: {other:?}"),
            };
            assert_eq!(
                location.as_deref(),
                Some(chosen.root()),
                "una observacion de {id} vino de otro target: {observation:?}"
            );
        }
    }
}

/// Un target sin version propia no ve la version de su hermano.
///
/// El caso contrario del anterior, y el que importa mas: sin esto, un
/// repositorio con un producto sin declarar y otro con su declaracion
/// resolvería el primero con el número del segundo, y ese es exactamente el
/// defecto que abrió este bloque, reproducido un nivel más arriba.
#[test]
fn un_target_sin_version_no_hereda_la_de_su_hermano() {
    let available = targets(&[("alpha", "packages/alpha"), ("beta", "packages/beta")]);
    let registry = registry_of(&[("packages/beta", "2.0.0")]);

    let chosen = select_target(&available, &TargetSelector::named("alpha")).expect("nombrado");
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, chosen);
    assert!(
        authority.version().is_none(),
        "alpha no declara version y no puede tomar la de beta: {authority:?}"
    );
    assert!(authority.is_failure(), "{authority:?}");
}

/// Y la observacion de un target que no existe tampoco se cuela.
#[test]
fn un_target_inexistente_no_puede_resolver() {
    let available = targets(&[("alpha", "packages/alpha")]);
    let registry = registry_of(&[("packages/alpha", "1.0.0"), ("packages/ghost", "9.9.9")]);
    let err = select_target(&available, &TargetSelector::named("ghost")).unwrap_err();
    assert!(
        matches!(err, TargetSelectionError::UnknownTarget { .. }),
        "{err:?}"
    );
    // Y sin selector, resolver solo lo que hay.
    let chosen = select_target(&available, &TargetSelector::unsolicited()).expect("hay uno");
    assert_eq!(
        registry
            .resolve(PRODUCT_VERSION_OBSERVATION, chosen)
            .version()
            .map(|v| v.to_string()),
        Some("1.0.0".into()),
        "el registro puede saber de un target fantasma; la seleccion no lo inventa"
    );
}

// ---------------------------------------------------------------------------
// El selector es parte del contrato
// ---------------------------------------------------------------------------

#[test]
fn un_selector_sin_nombre_es_distinto_de_uno_con_nombre() {
    assert_eq!(TargetSelector::unsolicited(), TargetSelector::default());
    assert_eq!(TargetSelector::unsolicited().requested(), None);
    assert_eq!(TargetSelector::named("alpha").requested(), Some("alpha"));
}

/// El fitness del dominio escanea el modulo; esta es su contraparte, y falla si
/// el nombre del target se compara con algo que no sea su identidad.
#[test]
fn el_nombre_se_compara_como_identidad_y_no_como_prefijo() {
    let available = targets(&[
        ("alpha", "packages/alpha"),
        ("alphabet", "packages/alphabet"),
    ]);
    // Un target cuyo nombre empieza por el pedido no casa.
    let err = select_target(&available, &TargetSelector::named("alpha")).expect("casa exacto");
    assert_eq!(err.id(), "alpha");
    // Y un pedido que es prefijo de otro no lo arrastra.
    let err = select_target(&available, &TargetSelector::named("alpha-2")).unwrap_err();
    assert!(
        matches!(err, TargetSelectionError::UnknownTarget { .. }),
        "{err:?}"
    );
}
