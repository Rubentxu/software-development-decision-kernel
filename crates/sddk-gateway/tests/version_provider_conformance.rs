//! Conformance for the declaration-file provider.
//!
//! # What "preserving behaviour" has to mean here
//!
//! Moving the file knowledge out of the crate that decides is a refactor that
//! can quietly change what SDDK believes about a project. So this file is not
//! a smoke test: it is the previously certified behaviour, written against the
//! new provider, one case per ecosystem that the release path depends on.
//!
//! The cases that matter most are the ones whose ANSWER is not a version:
//! - an ecosystem that declares nothing must not block a repository that
//!   declares somewhere else;
//! - a file that exists without declaring must not stop the others;
//! - a file that cannot be read must fail closed;
//! - two files that declare different versions must not be resolved by
//!   picking one.

use sddk_domain::version_authority::{
    VersionAuthority, VersionProbe, VersionResolverPort, PRODUCT_VERSION_OBSERVATION,
};
use sddk_gateway::version_provider::{default_version_registry, DeclarationFileProvider};
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) {
    let mut f = File::create(dir.join(name)).unwrap();
    writeln!(f, "{body}").unwrap();
}

fn target() -> sddk_domain::version_authority::ReleaseTarget {
    sddk_domain::version_authority::ReleaseTarget::at_root("repo")
}

fn observe(root: &Path) -> VersionProbe {
    DeclarationFileProvider::default()
        .observe_at(root)
        .expect("el provider no deberia fallar al ejecutarse")
}

fn version_of(root: &Path) -> String {
    match observe(root) {
        VersionProbe::Declared { version, .. } => version.to_string(),
        other => panic!("se esperaba una declaracion, vino {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Una declaracion por ecosistema, leida por el provider nuevo
// ---------------------------------------------------------------------------

#[test]
fn lee_una_declaracion_por_ecosistema() {
    let cases: &[(&str, &str, &str)] = &[
        ("Cargo.toml", "[workspace.package]\nversion = \"1.2.3\"", "1.2.3"),
        ("package.json", r#"{"name":"x","version":"4.5.6"}"#, "4.5.6"),
        ("pyproject.toml", "[project]\nname = \"y\"\nversion = \"7.8.9\"", "7.8.9"),
        (
            "gradle.properties",
            "org.gradle.caching=true\nversion=0.46.0",
            "0.46.0",
        ),
        (
            "Directory.Build.props",
            "<Project><PropertyGroup><Version>2.1.0</Version></PropertyGroup></Project>",
            "2.1.0",
        ),
        ("CMakeLists.txt", "project(nombre VERSION 3.2.1)", "3.2.1"),
    ];
    for (file, body, expected) in cases {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), file, body);
        assert_eq!(&version_of(d.path()), expected, "fichero {file}");
    }
}

#[test]
fn las_dos_dialectos_de_python_se_leen_por_separado() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "pyproject.toml", "[tool.poetry]\nversion = \"0.47.0\"");
    assert_eq!(version_of(d.path()), "0.47.0");
}

#[test]
fn un_ecosistema_solo_con_tag_no_declara_version() {
    for file in ["go.mod", "MODULE.bazel", "WORKSPACE"] {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), file, "module ejemplo\ngo 1.21\n");
        let probe = observe(d.path());
        assert!(
            matches!(probe, VersionProbe::Undeclared { .. }),
            "{file} declara que su tag es la autoridad y no debe aparecer como valor: {probe:?}"
        );
    }
}

#[test]
fn un_repositorio_vacio_no_es_aplicable() {
    let d = tempfile::tempdir().unwrap();
    let probe = observe(d.path());
    assert!(
        matches!(probe, VersionProbe::NotApplicable { .. }),
        "sin ningun fichero no hay sujeto: {probe:?}"
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
    write(d.path(), "gradle.properties", "org.gradle.caching=true\nkotlin.code.style=official");
    write(d.path(), "package.json", r#"{"name":"x","version":"3.4.0"}"#);
    assert_eq!(version_of(d.path()), "3.4.0");
}

#[test]
fn un_escrito_de_construccion_que_existe_no_tapa_a_la_que_si() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "build.gradle.kts", "version = \"0.47.0\"");
    write(d.path(), "package.json", r#"{"name":"x","version":"3.4.0"}"#);
    // El `build.gradle.kts` esta declarado como NO fuente declarable, luego
    // no se le inventa un 0.47.0 leyendo Kotlin con un regex. Lo que se exige
    // es que no MOLESTE.
    assert_eq!(version_of(d.path()), "3.4.0");
}

#[test]
fn un_manifiesto_que_no_declara_version_sigue_siendo_un_fallo_cerrado() {
    // La ley: ausente y no-declarado ceden, ROTO no.
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "package.json", "{ esto no es json");
    let result = DeclarationFileProvider::default().observe_at(d.path());
    assert!(
        result.is_err(),
        "un manifiesto corrupto tiene que fallar, no declararse ausente: {result:?}"
    );
}

#[test]
fn dos_ficheros_que_declaran_distinto_producen_ambiguedad_y_no_una_elegida() {
    // El provider NO decide esto: cada fichero es un provider y sus dos
    // respuestas llegan al reducer, que nombra el conflicto. Un provider
    // unico que leyera los dos ficheros tendria que elegir, y elegir es una
    // politica que no le corresponde.
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "package.json", r#"{"name":"x","version":"3.4.0"}"#);
    write(d.path(), "pyproject.toml", "[project]\nname = \"y\"\nversion = \"8.1.0\"\n");
    let authority = default_version_registry().resolve(
        PRODUCT_VERSION_OBSERVATION,
        &sddk_domain::version_authority::ReleaseTarget::at("repo", d.path().display().to_string()),
    );
    assert!(matches!(authority, VersionAuthority::Ambiguous { .. }), "{authority:?}");
    assert_eq!(authority.version(), None, "no se puede elegir: {authority:?}");
    match authority {
        VersionAuthority::Ambiguous { candidates, .. } => {
            assert_eq!(candidates.len(), 2, "{candidates:?}");
        }
        other => panic!("se esperaba Ambiguous, vino {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// El provider habla el puerto, y el registry lo reduce
// ---------------------------------------------------------------------------

#[test]
fn el_provider_responde_a_la_capability_versionada() {
    let provider = DeclarationFileProvider::default();
    assert_eq!(provider.provider_id(), "sddk.gateway.declaration-files");
    assert!(
        provider
            .capabilities()
            .iter()
            .any(|c| c == PRODUCT_VERSION_OBSERVATION),
        "{:?}",
        provider.capabilities()
    );
}

#[test]
fn a_traves_del_registry_el_veredicto_es_Resolved() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "Cargo.toml", "[workspace.package]\nversion = \"2.11.4\"");
    let registry = default_version_registry();
    let authority = registry.resolve(PRODUCT_VERSION_OBSERVATION, &{
        sddk_domain::version_authority::ReleaseTarget::at("repo", d.path().display().to_string())
    });
    assert_eq!(authority.version().map(|v| v.to_string()), Some("2.11.4".into()), "{authority:?}");
}

#[test]
fn una_declaracion_no_es_un_cross_check() {
    // Un solo fichero que declara NO es verificacion cruzada, y el reducer
    // tiene que decirlo. Antes el motor llamaba «CrossChecked» a una
    // declaracion unica, y ese nombre hacia que una lectura se leyera como
    // una comprobacion.
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "Cargo.toml", "[workspace.package]\nversion = \"1.0.0\"");
    let authority = default_version_registry().resolve(
        PRODUCT_VERSION_OBSERVATION,
        &sddk_domain::version_authority::ReleaseTarget::at("repo", d.path().display().to_string()),
    );
    assert_eq!(authority.version().map(|v| v.to_string()), Some("1.0.0".into()), "{authority:?}");
    assert!(!authority.was_cross_validated(), "una lectura no es una verificacion: {authority:?}");
}

#[test]
fn un_provider_por_fichero_no_es_evidencia_repetida() {
    // El registro por defecto registra un provider por fichero conocido, y
    // eso NO es corroboracion: son N fuentes Potenciales, y solo cuentan las
    // que contestan.
    let registry = default_version_registry();
    assert_eq!(
        registry.len(),
        sddk_gateway::version_provider::DEFAULT_DECLARATIONS.len()
    );
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "Cargo.toml", "[workspace.package]\nversion = \"1.0.0\"");
    let authority = registry.resolve(
        PRODUCT_VERSION_OBSERVATION,
        &sddk_domain::version_authority::ReleaseTarget::at("repo", d.path().display().to_string()),
    );
    assert!(!authority.was_cross_validated(), "{authority:?}");
}

#[test]
fn resolver_es_read_only() {
    // Ninguna observacion puede dejar el arbol como estaba: la ley es que la
    // resolucion es de solo lectura, y un provider que escribe no es un
    // provider, es un mutador con buen nombre.
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "Cargo.toml", "[workspace.package]\nversion = \"1.0.0\"");
    write(d.path(), "gradle.properties", "org.gradle.caching=true");
    write(d.path(), "setup.py", "from setuptools import setup\nsetup(name='x')");

    let before: Vec<(String, Vec<u8>)> = std::fs::read_dir(d.path())
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();

    let _ = observe(d.path());

    let after: Vec<(String, Vec<u8>)> = std::fs::read_dir(d.path())
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();

    assert_eq!(before, after, "observar modifico el arbol");
}
