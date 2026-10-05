//! El provider que **pregunta** al build tool, y lo que no puede hacer.
//!
//! # Qué se está midiendo
//!
//! Que la versión sale de la herramienta y no de una lectura del script.
//!
//! El riesgo de este provider no es que falle: es que **funcione en los casos
//! fáciles y calle en los difíciles**, que es donde nadie mira. Su versión
//! anterior —un lector de ficheros— no fallaba: leía `build.gradle.kts` y
//! devolvía algo, y ese algo era una suposición. La diferencia entre preguntar
//! y suponer es que preguntar puede decir «no lo sé», y suponer no.
//!
//! Los casos de abajo son los que un lector no podría tener, y son la razón de
//! que este provider sea un provider:
//!
//! - la herramienta contesta `unspecified` (**MEDIDO**), que no es una versión;
//! - la herramienta falla, y su motivo está **después** de un banner que es
//!   cierto para todos los fallos que han ocurrido jamás (**MEDIDO**);
//! - un subdirectorio contesta por **él**, no por el build raiz (**MEDIDO**).
//!
//! **Hermético por construcción:** se usa una herramienta falsa —un script que
//! contesta lo que el test dice— y no `gradle`. Una ley que necesita la
//! herramienta instalada para falsificarse es una ley que nadie falsifica, y
//! un provider que arranca una JVM en cada test es un provider que nadie ejecuta.

use std::fs;
use std::path::{Path, PathBuf};

use sddk_domain::version_authority::{
    ProductVersion, ReleaseTarget, VersionProbe, VersionResolverPort,
};
use sddk_gateway::version_provider::{
    BuildModelAnswer, BuildModelInvocation, BuildModelProvider, GRADLE_BUILD_FILES,
    UNSPECIFIED_WORD, parse_build_model,
};
use tempfile::TempDir;

/// A fake build tool. Three lines, and every law below is testable without a JVM.
struct FakeTool {
    _dir: TempDir,
    program: PathBuf,
}

impl FakeTool {
    /// A tool that answers `stdout` and exits 0.
    fn answering(stdout: &str) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let program = dir.path().join("fake-build-tool");
        fs::write(
            &program,
            format!("#!/bin/sh\ncat <<'SDDK_EOF'\n{stdout}\nSDDK_EOF\nexit 0\n"),
        )
        .expect("write tool");
        make_executable(&program);
        Self { _dir: dir, program }
    }

    /// A tool that fails the way Gradle fails: banner first, reason after.
    fn failing(stderr: &str, code: i32) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let program = dir.path().join("fake-build-tool");
        fs::write(
            &program,
            format!("#!/bin/sh\ncat <<'SDDK_EOF' >&2\n{stderr}\nSDDK_EOF\nexit {code}\n"),
        )
        .expect("write tool");
        make_executable(&program);
        Self { _dir: dir, program }
    }

    fn invocation(&self) -> BuildModelInvocation {
        BuildModelInvocation {
            program: self.program.display().to_string(),
            args: vec!["properties".to_owned(), "--offline".to_owned()],
        }
    }

    fn provider(&self) -> BuildModelProvider {
        BuildModelProvider::new(self.invocation())
    }
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).expect("stat").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("chmod");
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// A directory that IS a Gradle build, because it has a build file of its own.
fn gradle_target(name: &str) -> (TempDir, ReleaseTarget) {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("build.gradle"), "version = '1.2.3'\n").expect("write build");
    let target = ReleaseTarget::at(name, dir.path().display().to_string());
    (dir, target)
}

/// A directory that is NOT a Gradle build.
fn plain_target(name: &str) -> (TempDir, ReleaseTarget) {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("README.md"), "nothing here\n").expect("write readme");
    let target = ReleaseTarget::at(name, dir.path().display().to_string());
    (dir, target)
}

fn declared_version(probe: &VersionProbe) -> Option<&ProductVersion> {
    match probe {
        VersionProbe::Declared { version, .. } => Some(version),
        _ => None,
    }
}

// ── B1: la herramienta manda ─────────────────────────────────────────────

#[test]
fn b1_la_version_la_manda_la_herramienta() {
    let (_dir, target) = gradle_target("root");
    let tool = FakeTool::answering("Root project 'probe'\nversion: 9.9.9\ngroup: com.example\n");
    let probe = tool
        .provider()
        .observe(&target)
        .expect("la herramienta responde");

    let version = declared_version(&probe).expect("version declarada");
    assert_eq!(
        version.as_str(),
        "9.9.9",
        "el valor es el de la herramienta, no el que hay escrito en el build file: \
         el provider no lee el script para nada"
    );
    // Y el proyecto que contestó viaja en la procedencia. MEDIDO: desde un
    // subdirectorio Gradle contesta `Project ':lib-b'`, y un informe que no
    // dice de que proyecto es el valor no dice de quien es.
    if let VersionProbe::Declared { evidence, .. } = &probe {
        assert!(
            evidence.source_kind.ends_with("probe"),
            "la procedencia tiene que terminar en el proyecto que contestó, tal y \
             como la herramienta lo nombró —aquí `Root project 'probe'`, y en un \
             submódulo `Project ':lib-b'`—: {}",
            evidence.source_kind
        );
    }
    assert!(
        !probe_is_recheckable_by_bytes(&probe),
        "el valor no sale de los bytes de un fichero: poner el digest del build \
         file afirmaria que esos bytes determinan el valor, que es lo falso que \
         este provider vino a evitar"
    );
}

fn probe_is_recheckable_by_bytes(probe: &VersionProbe) -> bool {
    match probe {
        VersionProbe::Declared { evidence, .. } => evidence.digest.is_some(),
        _ => false,
    }
}

// ── B2: `unspecified` NO es una versión — el mutante peligroso ───────────

#[test]
fn b2_unspecified_no_es_una_version() {
    // MEDIDO: Gradle contesta exactamente esto cuando el build no declara
    // version. Un provider que lo tomara por un valor publicaria una version
    // llamada `unspecified`, que es la forma mas silenciosa de mentir que tiene
    // una herramienta: nadie ve un error, y el numero sale.
    let (_dir, target) = gradle_target("root");
    let tool = FakeTool::answering("Root project 'probe'\nversion: unspecified\n");
    let probe = tool
        .provider()
        .observe(&target)
        .expect("la herramienta responde");

    assert!(
        declared_version(&probe).is_none(),
        "`unspecified` es la palabra con la que una herramienta dice que no tiene \
         nada. Tomarla por un valor es inventar una version llamada `unspecified`."
    );
    assert!(
        matches!(probe, VersionProbe::Undeclared { .. }),
        "y lo que es es una AUSENCIA DECLARADA, que el reducer ya sabe reducir: \
         {:?}",
        probe
    );

    // El mutante, escrito. Y tiene que morir donde el codigo no.
    fn mutante_unspecified_es_version(stdout: &str) -> Option<String> {
        stdout
            .lines()
            .filter_map(|l| l.trim().strip_prefix("version:"))
            .map(|v| v.trim().to_owned())
            .next()
    }
    assert_eq!(
        mutante_unspecified_es_version("Root project 'p'\nversion: unspecified\n").as_deref(),
        Some("unspecified"),
        "el mutante devuelve la palabra como si fuera una version"
    );
    assert!(
        declared_version(&probe).is_none(),
        "y el codigo real no: por eso el mutante esta muerto"
    );
}

#[test]
fn b2b_el_proyecto_sin_version_dice_cual_y_que_no_tiene() {
    let (_dir, target) = gradle_target("root");
    let tool = FakeTool::answering("Project ':lib-b'\nversion: unspecified\n");
    let VersionProbe::Undeclared { reason } = tool.provider().observe(&target).expect("responde")
    else {
        panic!("una respuesta `unspecified` no es una version");
    };
    assert!(
        reason.contains(":lib-b") && reason.contains(UNSPECIFIED_WORD),
        "el motivo nombra el proyecto Y la palabra de la herramienta, porque un \
         motivo que no dice cual de los dos es un motivo que no se puede \
         comprobar: {reason}"
    );
}

// ── B3: el motivo es el de la herramienta, no su banner ──────────────────

#[test]
fn b3_el_motivo_no_es_el_banner() {
    // MEDIDO, y es el defecto que una v1 de este provider tenia: Gradle imprime
    // `FAILURE: Build failed with an exception.` antes de la causa, y esa linea
    // es cierta para todos los fallos de Gradle que han ocurrido jamás. Un
    // provider que falla cerrado con eso no falla cerrado: dice «algo fallo» y
    // manda al operador a buscar entre cien causas.
    let (_dir, target) = gradle_target("root");
    let tool = FakeTool::failing(
        "FAILURE: Build failed with an exception.\n\n* What went wrong:\nDirectory '/x' does not contain a Gradle build.\n",
        1,
    );
    let error = tool.provider().observe(&target).expect_err("falla cerrado");
    let reason = error.to_string();

    assert!(
        reason.contains("does not contain a Gradle build"),
        "el motivo tiene que ser el de la herramienta: {reason}"
    );
    assert!(
        !reason.contains("Build failed with an exception"),
        "y NO el banner, que no distingue nada: {reason}"
    );
}

#[test]
fn b3b_una_herramienta_que_no_usa_la_frase_de_gradle_tambien_habla() {
    // La frase `What went wrong:` es de Gradle. Exigírsela a toda herramienta
    // seria asumir el vocabulario de una en el sitio donde el punto es no
    // asumir el de ninguna.
    let (_dir, target) = gradle_target("root");
    let tool = FakeTool::failing("no such task: properties\n", 2);
    let reason = tool
        .provider()
        .observe(&target)
        .expect_err("falla cerrado")
        .to_string();
    assert!(
        reason.contains("no such task"),
        "una herramienta que dice otra cosa tambien dice algo: {reason}"
    );
}

// ── B4: una herramienta ausente NO es una herramienta que fallo ──────────

#[test]
fn b4_una_herramienta_ausente_dice_que_no_esta() {
    let (_dir, target) = gradle_target("root");
    let provider = BuildModelProvider::new(BuildModelInvocation {
        program: "/nonexistent/sddk-build-tool".to_owned(),
        args: vec!["properties".to_owned()],
    });
    let reason = provider
        .observe(&target)
        .expect_err("no se pudo ejecutar")
        .to_string();

    assert!(
        reason.contains("no se pudo ejecutar"),
        "«no tengo la herramienta» es NUESTRO motivo y lo es de verdad —el \
         programa no puede decir que no existe—: {reason}"
    );
    assert!(
        !reason.contains("no pudo responder por"),
        "y tiene que distinguirse de «la herramienta fallo», porque son dos \
         problemas con dos reparaciones distintas: {reason}"
    );
}

// ── B5: preguntar no escribe en el arbol ─────────────────────────────────

#[test]
fn b5_preguntar_no_escribe_en_el_arbol_del_proyecto() {
    let (dir, target) = gradle_target("root");
    let tool = FakeTool::answering("Root project 'root'\nversion: 1.2.3\n");
    let antes = snapshot(dir.path());
    tool.provider().observe(&target).expect("responde");
    assert_eq!(
        antes,
        snapshot(dir.path()),
        "este provider lanza un proceso, y un proceso que escribe en el proyecto \
         que se va a versionar convierte una lectura en una escritura"
    );
}

fn snapshot(root: &Path) -> Vec<(PathBuf, String)> {
    let mut entries = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = fs::read(&path) {
                entries.push((path.clone(), sha256_like(&bytes)));
            }
        }
    }
    entries.sort();
    entries
}

/// Not a digest: enough that two different contents never collide, which is all
/// the comparison needs.
fn sha256_like(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64-{hash:016x}")
}

// ── B6: un directorio que no es build no gasta un proceso ────────────────

#[test]
fn b6_un_directorio_que_no_es_build_no_se_consulta() {
    let (_dir, target) = plain_target("docs");
    let tool = FakeTool::answering("Root project 'nadie'\nversion: 9.9.9\n");
    let probe = tool.provider().observe(&target).expect("responde");

    assert!(
        matches!(probe, VersionProbe::NotApplicable { .. }),
        "sin fichero de build propio no hay modelo que preguntar: {:?}",
        probe
    );
    // Y el criterio es SOLO presencia, nunca contenido — decidir con el
    // contenido seria volver a leer el lenguaje.
    for nombre in GRADLE_BUILD_FILES {
        assert!(
            nombre.contains('.') || nombre.len() > 3,
            "`{nombre}` es un nombre de fichero, no un patron"
        );
    }
}

// ── B7: la funcion pura, que es donde vive la ley ───────────────────────

#[test]
fn b7_la_ley_vive_en_una_funcion_pura() {
    // MEDIDO, forma real de la salida: un encabezado de proyecto y despues
    // `clave: valor`. El resto son valores de objetos con `@` en medio, y un
    // `:` dentro de ellos no convierte la linea en una clave.
    let salida = "\
------------------------------------------------------------
Root project 'multi'
------------------------------------------------------------

allprojects: [root project 'multi']
buildDir: /tmp/x/build
group: com.example
version: 0.47.0
";
    let BuildModelAnswer::Declared { version, project } = parse_build_model(salida) else {
        panic!("una version declarada tiene que declararse");
    };
    assert_eq!(version.as_str(), "0.47.0");
    assert_eq!(project.as_deref(), Some("multi"));

    // Un subm-project contesta por su nombre, y eso es lo que permite que un
    // modulo que declara 2.0.0 no se lleve el 9.9.9 del padre.
    let sub = "Project ':lib-b'\nversion: 2.0.0\n";
    let BuildModelAnswer::Declared { version, project } = parse_build_model(sub) else {
        panic!("version declarada");
    };
    assert_eq!(version.as_str(), "2.0.0");
    assert_eq!(project.as_deref(), Some(":lib-b"));

    // Sin linea `version:` la herramienta contesto y no dijo nada: es un
    // silencio, no una ausencia declarada, y son dos hechos distintos.
    assert_eq!(
        parse_build_model("Root project 'x'\ngroup: com.example\n"),
        BuildModelAnswer::Silent,
        "salir con exito sin nombrar version no es lo mismo que decir \
         `unspecified`: una es que no hablo, la otra que no tengo"
    );

    // Y una clave que se parece mucho a `version` no es `version`.
    let parecido = "Project ':x'\nruntimeVersion: 11.0\nlibraryVersion: 3.1\n";
    assert_eq!(
        parse_build_model(parecido),
        BuildModelAnswer::Silent,
        "`runtimeVersion` contiene la palabra pero no ES la clave: un `contains` \
         en vez de un `==` se llevaria la primera"
    );
}
