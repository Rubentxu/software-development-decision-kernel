//! Conformance for the version providers.
//!
//! # What "preserving behaviour" has to mean here
//!
//! Moving the file knowledge out of the crate that decides is a refactor that
//! can quietly change what SDDK believes about a project. So this file is not
//! a smoke test: it is the previously certified behaviour, written against the
//! new providers, one case per ecosystem that the release path depends on.
//!
//! # Why every case here goes through the registry
//!
//! The first version of this suite called one provider with a directory and
//! read its answer. That provider is gone, and the suite had to change shape
//! with it, which is the point: **there is no longer a call that "resolves the
//! repository"**. There is a set of providers, each with a subject, and a
//! reducer that decides. A test that could still call a single provider and
//! get a verdict was a test that could pass while the deciding half was
//! untested.
//!
//! The cases that matter most are the ones whose ANSWER is not a version:
//! - an ecosystem that declares nothing must not block a repository that
//!   declares somewhere else;
//! - a file that exists without declaring must not stop the others;
//! - a file that cannot be read must fail closed;
//! - two files that declare different versions must not be resolved by
//!   picking one;
//! - a target that declares that its version lives on the release reference is
//!   not the same as one that said nothing, and must not be reported the same
//!   way.

use sddk_domain::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ReleaseTarget, VersionAuthority, VersionObservation, VersionProbe,
    VersionResolverPort,
};
use sddk_gateway::version_provider::{
    DECLARED_AUTHORITY_PATH, DEFAULT_DECLARATIONS, DeclaredAuthorityProvider,
    SingleDeclarationProvider, default_version_registry,
};
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut f = File::create(&path).unwrap();
    writeln!(f, "{body}").unwrap();
}

fn target(root: &Path) -> ReleaseTarget {
    ReleaseTarget::at("repo", root.display().to_string())
}

fn authority(root: &Path) -> VersionAuthority {
    default_version_registry().resolve(PRODUCT_VERSION_OBSERVATION, &target(root))
}

/// The observations whose probe is the given shape, for asserting on WHAT said
/// something rather than only on the verdict.
fn observations_where<F>(root: &Path, predicate: F) -> Vec<VersionObservation>
where
    F: Fn(&VersionProbe) -> bool,
{
    let all = match authority(root) {
        VersionAuthority::Resolved { observations, .. }
        | VersionAuthority::CrossValidated { observations, .. }
        | VersionAuthority::Ambiguous { observations, .. }
        | VersionAuthority::Invalid { observations, .. }
        | VersionAuthority::Unresolved { observations }
        | VersionAuthority::ReleaseRefIsAuthority { observations, .. } => observations,
    };
    all.into_iter().filter(|o| predicate(&o.probe)).collect()
}

fn version_of(root: &Path) -> String {
    match authority(root) {
        VersionAuthority::Resolved { version, .. }
        | VersionAuthority::CrossValidated { version, .. } => version.to_string(),
        other => panic!("se esperaba una declaracion, vino {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Una declaracion por ecosistema, leida por los providers nuevos
// ---------------------------------------------------------------------------

#[test]
fn lee_una_declaracion_por_ecosistema() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "Cargo.toml",
            "[workspace.package]\nversion = \"1.2.3\"",
            "1.2.3",
        ),
        ("package.json", r#"{"name":"x","version":"4.5.6"}"#, "4.5.6"),
        (
            "pyproject.toml",
            "[project]\nname = \"x\"\nversion = \"7.8.9\"",
            "7.8.9",
        ),
        (
            "gradle.properties",
            "org.gradle.caching=true\nversion=2.3.4\n",
            "2.3.4",
        ),
        (
            "Directory.Build.props",
            "<Project><PropertyGroup><Version>5.6.7</Version></PropertyGroup></Project>",
            "5.6.7",
        ),
        (
            "CMakeLists.txt",
            "project(demo VERSION 6.7.8 LANGUAGES C)",
            "6.7.8",
        ),
    ];
    for (file, body, expected) in cases {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), file, body);
        assert_eq!(&version_of(d.path()), expected, "{file}");
    }
}

#[test]
fn las_dos_dialectos_de_python_se_leen_por_separado() {
    let pep621 = tempfile::tempdir().unwrap();
    write(
        pep621.path(),
        "pyproject.toml",
        "[project]\nname = \"x\"\nversion = \"1.0.0\"",
    );
    assert_eq!(version_of(pep621.path()), "1.0.0");

    let poetry = tempfile::tempdir().unwrap();
    write(
        poetry.path(),
        "pyproject.toml",
        "[tool.poetry]\nname = \"x\"\nversion = \"2.0.0\"",
    );
    assert_eq!(version_of(poetry.path()), "2.0.0");
}

/// Un ecosistema cuya convencion es que la version la lleva la release ref.
///
/// Antes esto era `TagIsTheOnlyAuthority` en el motor y `Undeclared` en el
/// provider, y las dos cosas se confundieron el tiempo suficiente para que
/// `release plan` imprimiera `version: null` con `cross_checked` al lado. Ahora
/// es un veredicto con nombre, y el nombre dice lo que paso.
#[test]
fn un_ecosistema_sin_version_de_producto_declara_que_la_lleva_la_release_ref() {
    for file in ["go.mod", "MODULE.bazel", "WORKSPACE"] {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), file, "module ejemplo\ngo 1.21\n");
        let authority = authority(d.path());
        assert!(
            matches!(authority, VersionAuthority::ReleaseRefIsAuthority { .. }),
            "{file} declara que su version la lleva la release ref, no que no diga nada: {authority:?}"
        );
        assert!(
            !authority.is_failure(),
            "un target que declaro su convencion esta resuelto, no roto: {authority:?}"
        );
        assert!(authority.version().is_none(), "{authority:?}");
        assert!(
            !authority.was_cross_validated(),
            "y no hubo nada que contrastar: {authority:?}"
        );
        assert!(
            !authority.release_ref_declarations().is_empty(),
            "el veredicto tiene que decir POR QUE: {authority:?}"
        );
    }
}

#[test]
fn un_repositorio_vacio_no_declara_nada() {
    let d = tempfile::tempdir().unwrap();
    let authority = authority(d.path());
    assert!(
        matches!(authority, VersionAuthority::Unresolved { .. }),
        "sin ningun fichero no hay sujeto y no hay version: {authority:?}"
    );
    assert!(
        authority.release_ref_declarations().is_empty(),
        "y no es lo mismo que declarar una convencion: {authority:?}"
    );
    assert!(
        observations_where(d.path(), |p| matches!(p, VersionProbe::Undeclared { .. })).is_empty(),
        "nadie miro y encontro silencio: {authority:?}"
    );
}

// ---------------------------------------------------------------------------
// El caso que motivo todo: presente sin declarar no bloquea
// ---------------------------------------------------------------------------

#[test]
fn una_configuracion_que_no_declara_version_no_tapa_a_la_que_si() {
    // Es el caso del consumidor: un fichero de configuracion legitimo existe
    // y no declara version, y al lado hay otro que si. Antes esto abortaba la
    // resolucion entera.
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "gradle.properties",
        "org.gradle.caching=true\nkotlin.code.style=official",
    );
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"3.4.0"}"#,
    );
    assert_eq!(version_of(d.path()), "3.4.0");
}

#[test]
fn un_escrito_de_construccion_que_existe_no_tapa_a_la_que_si() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "build.gradle.kts", "version = \"0.47.0\"");
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"3.4.0"}"#,
    );
    // El `build.gradle.kts` esta declarado como NO fuente declarable, luego
    // no se le inventa un 0.47.0 leyendo Kotlin con un regex. Lo que se exige
    // es que no MOLESTE.
    assert_eq!(version_of(d.path()), "3.4.0");
}

/// El defecto que el consumidor stringify, medido: un `setup.py` se parseaba
/// como TOML porque lo declaraba el ecosistema, el parseo fallaba, y la
/// version que `package.json` ya habia declarado se perdia. El fichero es
/// Python y no se le inventa un lector.
#[test]
fn un_escrito_de_python_no_destruye_lo_que_ya_se_declaro() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"3.4.0"}"#,
    );
    write(
        d.path(),
        "setup.py",
        "from setuptools import setup\nsetup(name='x', version='1.0.0')",
    );
    assert_eq!(
        version_of(d.path()),
        "3.4.0",
        "un fichero de otro lenguaje no puede borrar la version que ya se leyo"
    );
}

#[test]
fn un_manifiesto_que_no_declara_version_sigue_siendo_un_fallo_cerrado() {
    // La ley: ausente y no-declarado ceden, ROTO no.
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "package.json", "{ esto no es json");
    let authority = authority(d.path());
    assert!(
        matches!(authority, VersionAuthority::Invalid { .. }),
        "un manifiesto corrupto tiene que fallar, no declararse ausente: {authority:?}"
    );
    match authority {
        VersionAuthority::Invalid { failures, .. } => {
            assert!(
                failures.iter().any(|(who, _)| who.contains("package.json")),
                "el fallo tiene que nombrar el fichero: {failures:?}"
            );
            assert!(
                failures.iter().any(|(_, why)| !why.trim().is_empty()),
                "y decir por que: {failures:?}"
            );
        }
        other => panic!("se esperaba Invalid, vino {other:?}"),
    }
}

#[test]
fn dos_ficheros_que_declaran_distinto_producen_ambiguedad_y_no_una_elegida() {
    // El provider NO decide esto: cada fichero es un provider y sus dos
    // respuestas llegan al reducer, que nombra el conflicto. Un provider
    // unico que leyera los dos ficheros tendria que elegir, y elegir es una
    // politica que no le corresponde.
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"3.4.0"}"#,
    );
    write(
        d.path(),
        "pyproject.toml",
        "[project]\nname = \"y\"\nversion = \"8.1.0\"\n",
    );
    let authority = authority(d.path());
    assert!(
        matches!(authority, VersionAuthority::Ambiguous { .. }),
        "{authority:?}"
    );
    assert_eq!(
        authority.version(),
        None,
        "no se puede elegir: {authority:?}"
    );
    match authority {
        VersionAuthority::Ambiguous { candidates, .. } => {
            assert_eq!(candidates.len(), 2, "{candidates:?}");
        }
        other => panic!("se esperaba Ambiguous, vino {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// La declaracion del propio proyecto
// ---------------------------------------------------------------------------

/// El rescate que el motor hacia con un `match` sobre su propio fallo, y que
/// ahora es un provider mas. Lo que se conserva no es el codigo sino la LEY:
/// solo rescata una ausencia, nunca una fuente ilegible.
#[test]
fn el_proyecto_puede_declarar_que_la_release_ref_lleva_su_version() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "gradle.properties", "org.gradle.caching=true");
    write(
        d.path(),
        DECLARED_AUTHORITY_PATH,
        r#"{"schema_version": 1, "authority": "tag"}"#,
    );
    let authority = authority(d.path());
    assert!(
        matches!(authority, VersionAuthority::ReleaseRefIsAuthority { .. }),
        "una declaracion explicita es una ausencia declarada, no un fallo: {authority:?}"
    );
    assert!(!authority.is_failure(), "{authority:?}");
}

/// Y no es una prioridad: si ademas hay una version declarada, esa manda.
#[test]
fn una_declaracion_del_proyecto_no_tapa_a_una_version_leida() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"3.4.0"}"#,
    );
    write(
        d.path(),
        DECLARED_AUTHORITY_PATH,
        r#"{"schema_version": 1, "authority": "tag"}"#,
    );
    assert_eq!(
        version_of(d.path()),
        "3.4.0",
        "declarar que la release ref lleva la version no borra la que se leyo"
    );
}

/// Una declaracion que no se entiende es un fallo cerrado, no un rescate.
#[test]
fn una_declaracion_que_no_se_entiende_no_se_ignora() {
    for body in [
        r#"{"schema_version": 2, "authority": "tag"}"#,
        r#"{"schema_version": 1, "authority": "lo que sea"}"#,
        r#"{"schema_version": 1}"#,
        r#"{ esto no es json"#,
    ] {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), DECLARED_AUTHORITY_PATH, body);
        let authority = authority(d.path());
        assert!(
            matches!(authority, VersionAuthority::Invalid { .. }),
            "una declaracion ilegible o de otra version tiene que fallar cerrado \
             y no degradarse a «nadie declaro nada». Body: {body} -> {authority:?}"
        );
    }
}

#[test]
fn un_proyecto_sin_declarar_no_inventa_una() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "gradle.properties", "org.gradle.caching=true");
    let authority = authority(d.path());
    assert!(
        matches!(authority, VersionAuthority::Unresolved { .. }),
        "un fichero legitimo que no declara version no es una convencion: {authority:?}"
    );
    assert!(authority.is_failure(), "{authority:?}");
    assert!(authority.version().is_none(), "{authority:?}");
}

// ---------------------------------------------------------------------------
// El provider habla el puerto, y el registry lo reduce
// ---------------------------------------------------------------------------

#[test]
fn el_provider_responde_a_la_capability_versionada() {
    let provider = SingleDeclarationProvider::for_spec(&DEFAULT_DECLARATIONS[0]);
    assert!(
        provider
            .provider_id()
            .starts_with("sddk.gateway.declaration-file/"),
        "la identidad es procedencia y dice QUE se observo: {}",
        provider.provider_id()
    );
    assert!(
        provider
            .capabilities()
            .iter()
            .any(|c| c == PRODUCT_VERSION_OBSERVATION),
        "{:?}",
        provider.capabilities()
    );
    let declared = DeclaredAuthorityProvider::new();
    assert!(
        declared
            .capabilities()
            .iter()
            .any(|c| c == PRODUCT_VERSION_OBSERVATION),
        "la declaracion del proyecto contesta la MISMA pregunta: {:?}",
        declared.capabilities()
    );
}

#[test]
fn a_traves_del_registry_el_veredicto_es_resolved() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"2.11.4\"",
    );
    let authority = authority(d.path());
    assert!(
        matches!(authority, VersionAuthority::Resolved { .. }),
        "{authority:?}"
    );
    assert_eq!(
        authority.version().map(|v| v.to_string()),
        Some("2.11.4".into()),
        "{authority:?}"
    );
}

#[test]
fn una_declaracion_no_es_un_cross_check() {
    // Un solo fichero que declara NO es verificacion cruzada, y el reducer
    // tiene que decirlo. Antes el motor llamaba «CrossChecked» a una
    // declaracion unica, y ese nombre hacia que una lectura se leyera como
    // una comprobacion.
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"1.0.0\"",
    );
    let authority = authority(d.path());
    assert_eq!(
        authority.version().map(|v| v.to_string()),
        Some("1.0.0".into()),
        "{authority:?}"
    );
    assert!(
        !authority.was_cross_validated(),
        "una lectura no es una verificacion: {authority:?}"
    );
}

#[test]
fn dos_lectores_que_coinciden_si_cruzan_validacion() {
    // El otro lado de la misma ley: cuando hay dos fuentes INDEPENDIENTES que
    // dicen lo mismo, el veredicto tiene que decirlo, porque es lo unico que
    // distingue «encontrado» de «verificado».
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"1.0.0\"",
    );
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"1.0.0"}"#,
    );
    let authority = authority(d.path());
    assert!(
        authority.was_cross_validated(),
        "dos fuentes independientes que coinciden SI se cruzaron: {authority:?}"
    );
    assert_eq!(
        authority.version().map(|v| v.to_string()),
        Some("1.0.0".into()),
        "{authority:?}"
    );
}

#[test]
fn un_provider_por_fichero_no_es_evidencia_repetida() {
    // El registro por defecto registra un provider por fichero conocido MAS el
    // de la declaracion del proyecto, y eso NO es corroboracion: son N
    // fuentes Potenciales, y solo cuentan las que contestan.
    let registry = default_version_registry();
    assert_eq!(
        registry.len(),
        DEFAULT_DECLARATIONS.len() + 1,
        "uno por fichero declarado, mas la declaracion del proyecto"
    );
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"1.0.0\"",
    );
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &target(d.path()));
    assert!(!authority.was_cross_validated(), "{authority:?}");
}

#[test]
fn resolver_es_read_only() {
    // Ninguna observacion puede dejar el arbol como estaba: la ley es que la
    // resolucion es de solo lectura, y un provider que escribe no es un
    // provider, es un mutador con buen nombre.
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"1.0.0\"",
    );
    write(d.path(), "gradle.properties", "org.gradle.caching=true");
    write(
        d.path(),
        "setup.py",
        "from setuptools import setup\nsetup(name='x')",
    );
    write(
        d.path(),
        DECLARED_AUTHORITY_PATH,
        r#"{"schema_version": 1, "authority": "tag"}"#,
    );

    let before = snapshot(d.path());
    let _ = authority(d.path());
    let after = snapshot(d.path());

    assert_eq!(before, after, "observar modifico el arbol");
}

/// Every file under `root`, by relative path and content.
///
/// Recursive on purpose, and not as a nicety: the project's own declaration
/// lives in a subdirectory, so a flat listing of the root would have skipped
/// the one file a provider is most likely to want to write — and a read-only
/// test that cannot see the write it is looking for is decoration.
fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let relative = path.strip_prefix(root).unwrap().display().to_string();
                out.push((relative, std::fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}
