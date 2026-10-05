//! Fitness and conformance for the generic version authority model.
//!
//! Two things are checked here, and they are different in kind:
//!
//! 1. **Fitness** — a static scan proving the decision module names no
//!    concrete technology. This is the part that cannot be reviewed by
//!    reading: a denylist is only worth having if something enforces it, and
//!    the failure mode it prevents (a tool name leaking into the kernel) is
//!    exactly the kind that arrives one "harmless" convenience at a time.
//! 2. **Conformance** — the reducer's whole truth table, including the
//!    properties that are easy to state and easy to lose: that it does not
//!    depend on observation order, and that a provider's identity buys it
//!    nothing.

use sddk_domain::version_authority::{
    ProductVersion, ReleaseTarget, VersionAuthority, VersionEvidence, VersionObservation,
    VersionProbe, reduce,
};

// ---------------------------------------------------------------------------
// 1. Fitness
// ---------------------------------------------------------------------------

/// Names that must never appear in the module that decides.
///
/// Written as literal substrings rather than a clever pattern, because the
/// failure this guards against is somebody writing the name once in a
/// comment and a reviewer not objecting. `manifest` is deliberately absent:
/// it is a generic word, and a fitness that banned ordinary English would
/// push people to evade it rather than to comply.
const FORBIDDEN: &[&str] = &[
    // languages
    "rust",
    "python",
    "kotlin",
    "groovy",
    "java",
    "typescript",
    "javascript",
    "go_lang",
    "golang",
    "csharp",
    "c_plus_plus", // build systems
    "gradle",
    "maven",
    "cargo",
    "npm",
    "yarn",
    "pnpm",
    "bazel",
    "cmake",
    "msbuild",
    "dotnet",
    "pip",
    "poetry", // concrete files
    "cargo.toml",
    "package.json",
    "pyproject.toml",
    "gradle.properties",
    "build.gradle",
    "pom.xml",
    "directory.build.props",
    "cmakelists.txt",
    "go.mod",
    "module.bazel",
    // ecosystems as the kernel would name them
    "jvm_gradle",
    "cpp_cmake",
];

#[test]
fn the_decision_module_names_no_concrete_technology() {
    let source = include_str!("../src/version_authority.rs").to_lowercase();
    let mut hits: Vec<&str> = Vec::new();
    for needle in FORBIDDEN {
        if source.contains(needle) {
            hits.push(needle);
        }
    }
    assert!(
        hits.is_empty(),
        "el modulo de decision nombra tecnologia concreta: {hits:?}. \
         El kernel define las preguntas; los providers saben obtener la evidencia."
    );
}

/// The fitness above scans the module *source*. This one proves the scanner
/// is capable of finding a name, so a future `hits.is_empty()` cannot pass
/// because the denylist rotted or the `include_str!` stopped resolving.
///
/// The control injects a synthetic haystack carrying a listed name and runs
/// the SAME predicate over it. A control that greps a literal nobody ever
/// wrote —which is what the first version of this test did— passes or fails
/// for reasons unrelated to the scanner.
#[test]
fn the_fitness_scanner_can_actually_see_a_name() {
    let haystack = format!("un fichero de ejemplo y otro de {} al lado", FORBIDDEN[0]);
    let found: Vec<&str> = FORBIDDEN
        .iter()
        .copied()
        .filter(|needle| haystack.contains(needle))
        .collect();
    assert_eq!(
        found,
        vec![FORBIDDEN[0]],
        "el scanner no encuentra un nombre que esta en la lista: el fitness de arriba \
         no miraria nada"
    );
    // Y la lista no esta vacia, que es la forma mas tonta de pasar.
    assert!(
        FORBIDDEN.len() >= 10,
        "la lista de prohibidos se ha vaciado"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn v(value: &str) -> ProductVersion {
    ProductVersion::new(value).expect("version valida en el test")
}

fn evidence(source_kind: &str) -> VersionEvidence {
    VersionEvidence {
        source_kind: source_kind.to_owned(),
        digest: Some("sha256:".to_owned() + &"0".repeat(64)),
        location: Some(".".to_owned()),
    }
}

fn observation(provider: &str, probe: VersionProbe) -> VersionObservation {
    VersionObservation {
        provider_id: provider.to_owned(),
        provider_version: "1.0.0".to_owned(),
        capability: "product-version.observation/v1".to_owned(),
        probe,
    }
}

fn declared(provider: &str, version: &str) -> VersionObservation {
    VersionObservation {
        provider_id: provider.to_owned(),
        provider_version: "1.0.0".to_owned(),
        capability: "product-version.observation/v1".to_owned(),
        probe: VersionProbe::Declared {
            version: v(version),
            evidence: evidence("fixture"),
        },
    }
}

fn not_applicable(provider: &str) -> VersionObservation {
    VersionObservation {
        provider_id: provider.to_owned(),
        provider_version: "1.0.0".to_owned(),
        capability: "product-version.observation/v1".to_owned(),
        probe: VersionProbe::NotApplicable {
            reason: "no es asunto mio".to_owned(),
        },
    }
}

fn undeclared(provider: &str) -> VersionObservation {
    VersionObservation {
        provider_id: provider.to_owned(),
        provider_version: "1.0.0".to_owned(),
        capability: "product-version.observation/v1".to_owned(),
        probe: VersionProbe::Undeclared {
            reason: "lo mire y no declara".to_owned(),
        },
    }
}

fn invalid(provider: &str) -> VersionObservation {
    VersionObservation {
        provider_id: provider.to_owned(),
        provider_version: "1.0.0".to_owned(),
        capability: "product-version.observation/v1".to_owned(),
        probe: VersionProbe::Invalid {
            reason: "existe y no se pudo leer".to_owned(),
        },
    }
}

// ---------------------------------------------------------------------------
// 2. The reducer's truth table
// ---------------------------------------------------------------------------

#[test]
fn sin_observaciones_es_unresolved() {
    assert_eq!(
        reduce(vec![]),
        VersionAuthority::Unresolved {
            observations: vec![]
        }
    );
}

#[test]
fn not_applicable_no_declara_nada() {
    let a = reduce(vec![not_applicable("p1"), not_applicable("p2")]);
    assert!(matches!(a, VersionAuthority::Unresolved { .. }), "{a:?}");
    assert_eq!(a.version(), None);
}

#[test]
fn undeclared_no_inventa_una_version() {
    let a = reduce(vec![undeclared("p1"), undeclared("p2")]);
    assert!(matches!(a, VersionAuthority::Unresolved { .. }), "{a:?}");
    assert_eq!(a.version(), None);
}

#[test]
fn una_declaracion_resuelve() {
    let a = reduce(vec![declared("p1", "3.4.0")]);
    assert_eq!(a.version(), Some(&v("3.4.0")));
    assert!(!a.was_cross_validated());
    assert!(!a.is_failure());
}

#[test]
fn lo_que_no_declara_no_impide_a_otro_que_si_declara() {
    // Este es el caso que descubrió el consumidor: una fuente de
    // configuración presente y sin versión no puede tapar a la que sí
    // declara. Antes de que el reducer existiera, ese caso era un abort.
    let a = reduce(vec![undeclared("p1"), declared("p2", "0.47.0")]);
    assert_eq!(a.version(), Some(&v("0.47.0")), "{a:?}");
}

#[test]
fn dos_declaraciones_iguales_cruzan_validacion() {
    let a = reduce(vec![declared("p1", "3.4.0"), declared("p2", "3.4.0")]);
    assert_eq!(a.version(), Some(&v("3.4.0")));
    assert!(a.was_cross_validated(), "{a:?}");
    // Y solo guarda las que cohieren: la veracidad de «lo verificaron» depende
    // de no contar como acuerdo a quien no declaró.
    match a {
        VersionAuthority::CrossValidated { observations, .. } => assert_eq!(observations.len(), 2),
        other => panic!("se esperaba CrossValidated, vino {other:?}"),
    }
}

#[test]
fn dos_declaraciones_distintas_son_ambiguas_y_no_se_elige_una() {
    let a = reduce(vec![declared("p1", "3.4.0"), declared("p2", "8.1.0")]);
    assert_eq!(a.version(), None, "no se puede elegir: {a:?}");
    match a {
        VersionAuthority::Ambiguous { candidates, .. } => {
            assert_eq!(candidates.len(), 2);
        }
        other => panic!("se esperaba Ambiguous, vino {other:?}"),
    }
}

#[test]
fn un_provider_invalido_falla_cerrado_aunque_otro_declare() {
    // La ley se comprueba PRIMERO, y no por el orden en que lleguen: un
    // `Declared` posterior no puede legitimar un `Invalid` previo.
    let a = reduce(vec![invalid("p1"), declared("p2", "3.4.0")]);
    assert!(a.is_failure());
    match a {
        VersionAuthority::Invalid { failures, .. } => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].0, "p1");
        }
        other => panic!("se esperaba Invalid, vino {other:?}"),
    }
}

#[test]
fn el_orden_de_las_observaciones_no_cambia_el_veredicto() {
    // La prueba de que no hay una prioridad escondida. Si el reducer
    // ordenase por identidad de provider, o se quedase con la primera,
    // una permutacion lo delataria.
    let base = vec![
        declared("alpha", "1.0.0"),
        undeclared("bravo"),
        declared("charlie", "1.0.0"),
    ];
    let expected = reduce(base.clone());
    for permutation in [
        vec![base[1].clone(), base[2].clone(), base[0].clone()],
        vec![base[2].clone(), base[0].clone(), base[1].clone()],
        vec![base[0].clone(), base[1].clone(), base[2].clone()],
    ] {
        assert_eq!(
            reduce(permutation.clone()),
            expected,
            "cambio con {permutation:?}"
        );
    }
}

#[test]
fn la_identidad_del_provider_compra_autoridad_a_nadie() {
    // Un provider con un nombre que suena mas oficial no pesa mas. Se
    // comprueba poniendolo PRIMERO: si el reducer ordenase por nombre,
    // declararia su version.
    let a = reduce(vec![
        declared("zzz-oficial", "9.9.9"),
        declared("aaa-oculto", "1.0.0"),
    ]);
    assert_eq!(a.version(), None, "no se puede elegir por el nombre: {a:?}");
    assert!(matches!(a, VersionAuthority::Ambiguous { .. }), "{a:?}");
}

#[test]
fn un_provider_consultado_dos_veces_no_es_un_segundo_voto() {
    // Dos observaciones del MISMO provider con la misma version son una
    // declaracion, no un acuerdo. Contarlas como dos haria que
    // `was_cross_validated` dijera «verificado» sin que hubiera
    // verificacion.
    let a = reduce(vec![declared("p1", "1.0.0"), declared("p1", "1.0.0")]);
    assert!(!a.was_cross_validated(), "{a:?}");
    assert_eq!(a.version(), Some(&v("1.0.0")));
}

// ---------------------------------------------------------------------------
// The types the model is made of
// ---------------------------------------------------------------------------

#[test]
fn una_version_vacia_o_con_espacios_no_es_una_version() {
    assert!(ProductVersion::new("").is_err());
    assert!(ProductVersion::new("   ").is_err());
    assert!(ProductVersion::new("1.0.0 rc1").is_err());
    assert!(
        ProductVersion::new(" 1.0.0 ").is_ok(),
        "el recorte no es un esquema"
    );
}

#[test]
fn una_version_opaca_no_afirma_ningun_esquema() {
    // El nucleo no sabe que es SemVer ni lo comprueba: una identidad que no
    // se parece a nada conocido es valida, porque la comprehension del
    // esquema es de quien la pide.
    for identity in ["2026.10.05", "0.47.0-rc2", "release-2024-a", "v1", "1"] {
        assert!(
            ProductVersion::new(identity).is_ok(),
            "{identity} deberia ser valido"
        );
    }
}

#[test]
fn un_target_es_un_producto_y_no_un_repositorio() {
    let root = ReleaseTarget::at_root("runtime");
    assert_eq!(root.id(), "runtime");
    assert_eq!(root.root(), ".");

    let nested = ReleaseTarget::at("sdk", "packages/sdk");
    assert_eq!(nested.root(), "packages/sdk");
    // Dos targets con la misma raiz siguen siendo dos targets: la identidad
    // la da quien los nombra, no donde viven.
    assert_ne!(root.id(), nested.id());
}

#[test]
fn la_evidencia_no_es_un_veredicto() {
    // Una evidencia sin digest es legitima —no todo lo que se observa se
    // puede hashear— y aun asi la declaracion cuenta. Lo que no se permite
    // es que la ausencia de digest la convierta en un default.
    let without_digest = VersionObservation {
        provider_id: "p1".to_owned(),
        provider_version: "1.0.0".to_owned(),
        capability: "product-version.observation/v1".to_owned(),
        probe: VersionProbe::Declared {
            version: v("1.0.0"),
            evidence: VersionEvidence {
                source_kind: "consulta".to_owned(),
                digest: None,
                location: None,
            },
        },
    };
    assert_eq!(reduce(vec![without_digest]).version(), Some(&v("1.0.0")));
}

/// Una version REAL leida le gana a una convencion que dice que no la hay.
///
/// El caso es `go.mod` al lado de un `package.json`: el primero declara que su
/// version vive en la etiqueta, el segundo publica `3.4.0`. Si la ausencia
/// declarada ganara, el release se autorizaria sin version, que es la clase de
/// fallo que el reducer entero existe para impedir.
#[test]
fn una_version_leida_no_se_tapa_con_una_ausencia_declarada() {
    let observations = vec![
        observation(
            "p-ref",
            VersionProbe::ReleaseRefIsAuthority {
                declared_by: "su version la lleva la etiqueta".to_owned(),
            },
        ),
        declared("p-value", "3.4.0"),
    ];
    let authority = reduce(observations);
    assert_eq!(
        authority.version(),
        Some(&v("3.4.0")),
        "una version observada no se descarta por una convencion ajena: {authority:?}"
    );
    assert!(
        !authority.was_cross_validated(),
        "una sola version observada no es cross-validacion: {authority:?}"
    );
}

/// La ausencia declarada es un veredicto PROPIO, no un `Unresolved`.
///
/// Un `Unresolved` es «nadie dijo nada, hay que decidir». Una ausencia
/// declarada es «alguien dijo que aqui no hay version y por que». Juntas, un
/// proyecto que declaro su convencion y uno que se dejo en blanco producen el
/// mismo plan de release, y el segundo se publica sin que nadie lo notara.
#[test]
fn una_ausencia_declarada_no_es_silencio() {
    let observations = vec![
        not_applicable("p-na"),
        observation(
            "p-ref",
            VersionProbe::ReleaseRefIsAuthority {
                declared_by: "este target no declara version de producto".to_owned(),
            },
        ),
    ];
    let authority = reduce(observations);
    assert_eq!(
        authority.release_ref_declarations(),
        ["este target no declara version de producto"],
        "la razon declarada es lo unico que dice por que: {authority:?}"
    );
    assert!(
        !authority.is_failure(),
        "un target que declaro su convencion esta resuelto, no roto: {authority:?}"
    );
    assert!(authority.version().is_none());
    assert!(
        !matches!(authority, VersionAuthority::Unresolved { .. }),
        "declarar la ausencia NO es lo mismo que no declarar nada: {authority:?}"
    );
}

/// Y aun asi no es exito de comprobacion: no hubo nada que contrastar.
#[test]
fn una_ausencia_declarada_no_es_un_exito_de_comprobacion() {
    let authority = reduce(vec![observation(
        "p-ref",
        VersionProbe::ReleaseRefIsAuthority {
            declared_by: "sin version de producto".to_owned(),
        },
    )]);
    assert!(!authority.was_cross_validated());
    assert!(!authority.is_failure());
    assert!(authority.version().is_none());
}

/// Un `Invalid` sigue ganando a todo, incluida una ausencia declarada.
///
/// El orden de las leyes no es estetico: si la ausencia se comprobara antes,
/// un manifiesto roto quedaria tapado por la convencion de otro fichero y la
/// resolucion pasaria en verde.
#[test]
fn lo_ilegible_no_se_tapa_con_una_ausencia_declarada() {
    let authority = reduce(vec![
        observation(
            "p-ref",
            VersionProbe::ReleaseRefIsAuthority {
                declared_by: "su version la lleva la etiqueta".to_owned(),
            },
        ),
        observation(
            "p-bad",
            VersionProbe::Invalid {
                reason: "existe y no se pudo leer".to_owned(),
            },
        ),
    ]);
    assert!(
        matches!(authority, VersionAuthority::Invalid { .. }),
        "un fichero ilegible falla cerrado aunque otro declare una convencion: {authority:?}"
    );
}
