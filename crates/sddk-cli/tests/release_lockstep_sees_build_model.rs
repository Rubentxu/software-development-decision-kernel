//! `--build-tool` dice lo que hace.
//!
//! # Qué se está midiendo
//!
//! Que la herramienta que se nombra es la que se ejecuta, y que un nombre que
//! SDDK no sepa preguntar es un error de la línea de comandos.
//!
//! MEDIDO antes del arreglo, con un ejecutable instrumentado en el `PATH`:
//! `--build-tool mvn` sobre un build Gradle hizo que `mvn` recibiera
//! `properties --offline` —un goal que no existe en Maven— y el informe
//! atribuyera la respuesta a `././build.gradle`, un fichero que Maven nunca
//! abrió. Cada cara era correcta por separado; juntas fabricaron una evidencia.
//!
//! **Hermético:** estos tests no tocan el `PATH` ni lanzan una herramienta real,
//! porque un test que necesita una JVM —o un `PATH` global— para comprobar una
//! bandera es un test que no se ejecuta en la mitad de las máquinas. El caso que
//! sí necesita un binario en el `PATH` se mide en el recibo, no aquí.
//!
//! # Por qué los fixtures declaran `gradle.properties`
//!
//! MEDIDO, al escribir este fichero: un repo con solo `build.gradle` **no es un
//! release target**, porque `build.gradle` no es fuente declarable y nadie
//! declara versión. `release_targets` lo rechaza con `no release target found`
//! y el test mide el fixture equivocado.
//!
//! `gradle.properties` sí es declarable, así que da al repo un target sin
//! cambiar nada de lo que estos tests miden: los ficheros de build de cada
//! dialecto siguen siendo los suyos.

use std::fs;
use std::path::Path;

use sddk_cli::{CommandOutput, run_from};
use tempfile::TempDir;

/// Un repo que es target (declara versión) y es un build de Gradle, sin `pom.xml`.
fn repo_gradle() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("build.gradle"), "version = '1.2.3'\n").expect("write build");
    fs::write(dir.path().join("gradle.properties"), "version=1.2.3\n").expect("write props");
    dir
}

/// Un repo que es target y tiene `pom.xml`, pero **no** `build.gradle`.
///
/// Es el caso donde Gradle **no** debe engancharse: no porque la bandera falte,
/// sino porque este repo no es suyo.
fn repo_solo_maven() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("pom.xml"), "<project/>\n").expect("write pom");
    fs::write(dir.path().join("gradle.properties"), "version=1.2.3\n").expect("write props");
    dir
}

/// Un repo Maven que **solo** es Maven: ni `build.gradle` ni `gradle.properties`.
///
/// MEDIDO, al construir este bloque: un repo asi no era un release target, y el
/// provider de Maven **nunca se ejecutaba**. Era un deadlock: para preguntar al
/// build tool hace falta un target, y para tener un target hace falta declarar
/// version, y para declararla hay que preguntar al build tool.
fn repo_maven_puro() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("pom.xml"), "<project/>\n").expect("write pom");
    dir
}

fn inspect(root: &Path, extra: &[&str]) -> CommandOutput {
    let mut args: Vec<String> = vec![
        "sddk".into(),
        "release".into(),
        "version".into(),
        "inspect".into(),
        "--root".into(),
        root.to_str().expect("utf-8").into(),
        "--scope".into(),
        ".".into(),
    ];
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_from(refs)
}

fn stdout_de(root: &Path, extra: &[&str]) -> String {
    let out = inspect(root, extra);
    assert_eq!(
        out.status, 0,
        "un informe que no sale es un informe que no mide nada: stderr: {}",
        out.stderr
    );
    out.stdout
}

/// La linea `providers_answering:` del informe.
fn respondieron(stdout: &str) -> &str {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("providers_answering:"))
        .map(str::trim)
        .unwrap_or("<sin linea providers_answering>")
}

/// Que ningun provider de **build model** haya contestado.
///
/// No `respondieron() == "none"`: un repo que declara version por
/// `gradle.properties` tiene un provider que SI contesta, y confundir «contesto
/// el que leia el fichero» con «contesto el que pregunta a la herramienta» seria
/// volver a medir la mitad de la cosa. Lo que estos tests vigilan es uno solo: el
/// build model.
fn build_model_no_respondio(stdout: &str) -> bool {
    !respondieron(stdout).contains("build-model")
}

/// Que ninguna linea `declared at` apunte a `fichero`.
///
/// No `!stdout.contains("declared at")`: un repo que declara version por
/// `gradle.properties` SI tiene un `declared at`, y ese es legitimo. Lo que no
/// puede pasar es que apunte al fichero de OTRA herramienta.
fn atribuido_a(stdout: &str, fichero: &str) -> bool {
    stdout
        .lines()
        .filter(|l| l.contains("declared at"))
        .any(|l| l.contains(fichero))
}

// ── El falsador del defecto medido ────────────────────────────────────────

#[test]
fn maven_sobre_un_build_de_gradle_no_atribuye_a_gradle() {
    let dir = repo_gradle();
    let salida = stdout_de(dir.path(), &["--evaluate-build", "--build-tool", "maven"]);

    assert!(
        build_model_no_respondio(&salida),
        "Maven no se engancha a un repo que no tiene pom.xml, asi que el build \
         model no contesta nada:\n{salida}"
    );
    assert!(
        !atribuido_a(&salida, "build.gradle"),
        "el unico `declared at` que puede aparecer es el de `gradle.properties`, \
         que lo lee un fichero. Atribuir a `build.gradle` es exactamente el \
         defecto medido —Maven respondiendo por un fichero de Gradle—:\n{salida}"
    );
    assert!(
        salida.contains("no tiene fichero de build de maven"),
        "y tiene que decir POR QUE no pregunto, que es lo que permite saber que \
         el repo es de otra herramienta:\n{salida}"
    );
}

#[test]
fn gradle_sobre_un_repo_solo_maven_no_atribuye_a_gradle() {
    let dir = repo_solo_maven();
    let salida = stdout_de(dir.path(), &["--evaluate-build", "--build-tool", "gradle"]);

    assert!(
        salida.contains("no tiene fichero de build de gradle"),
        "el motivo tiene que nombrar la herramienta que se pidio, no otra:\n{salida}"
    );
    assert!(
        !salida.contains("declared at ./"),
        "y no puede declarar nada, porque Gradle no se engancha a un repo sin \
         build.gradle:\n{salida}"
    );
}

// ── Un nombre que no se sabe es un error, no un Gradle ────────────────────

#[test]
fn un_build_tool_desconocido_falla_y_no_pregunta_a_gradle() {
    let dir = repo_gradle();
    let out = inspect(
        dir.path(),
        &["--evaluate-build", "--build-tool", "gradlew-mas-args"],
    );

    assert_ne!(
        out.status, 0,
        "un nombre que SDDK no sabe preguntar tiene que fallar. Antes de este \
         arreglo se aceptaba cualquier palabra: stdout={} stderr={}",
        out.stdout, out.stderr
    );
    assert!(
        out.stderr.contains("gradle, maven"),
        "el error tiene que decir cuales SI se saben preguntar: {}",
        out.stderr
    );
    assert!(
        !out.stderr.contains("build.gradle"),
        "y no puede haber construido ningun provider: no se llego a preguntar: {}",
        out.stderr
    );
}

#[test]
fn evaluate_build_sin_build_tool_falla_visible() {
    let dir = repo_gradle();
    let out = inspect(dir.path(), &["--evaluate-build"]);

    assert_ne!(
        out.status, 0,
        "`--evaluate-build` sin `--build-tool` no es una pregunta a nadie, y una \
         bandera que no pregunta tiene que decirlo: stdout={} stderr={}",
        out.stdout, out.stderr
    );
    assert!(
        out.stderr.contains("--build-tool"),
        "y el error tiene que decir que bandera falta: {}",
        out.stderr
    );
}

// ── Lo que el arreglo NO puede haber roto ────────────────────────────────

#[test]
fn sin_build_tool_no_se_pregunta_a_nadie() {
    let dir = repo_gradle();
    let salida = stdout_de(dir.path(), &[]);

    assert!(
        build_model_no_respondio(&salida),
        "sin la bandera ningun provider de build model contesta:\n{salida}"
    );
    assert!(
        salida.contains("the build tool was NOT asked"),
        "y el informe tiene que DECIR que no se evaluo, porque no evaluarlo es \
         una decision y callarla seria otra:\n{salida}"
    );
}

#[test]
fn un_build_tool_sin_la_bandera_no_dispara_ningun_proceso() {
    let dir = repo_gradle();
    let salida = stdout_de(dir.path(), &["--build-tool", "maven"]);

    assert!(
        build_model_no_respondio(&salida),
        "`--build-tool` sin `--evaluate-build` no pregunta: el dialecto solo se \
         registra cuando alguien pidio preguntar.\n{salida}"
    );
    assert!(
        salida.contains("the build tool was NOT asked"),
        "y el informe lo sigue declarando, que es lo que importa cuando no se \
         pregunto:\n{salida}"
    );
}

/// El coste de la capacidad, declarado.
///
/// MEDIDO: un repo Maven puro (`pom.xml` y nada mas) **no** es un release target
/// sin `--evaluate-build`. No es un descuido: es lo que cuesta la capacidad, y
/// por eso no se paga sin que alguien la pida.
///
/// Este test es el que cae si alguien convierte la pregunta en un default. Un
/// default que paga 3 s y una JVM por target sin pedirlo es exactamente lo que
/// los flags `--naming` y `--role` evitan en el resto del comando.
#[test]
fn un_repo_maven_puro_no_es_target_sin_preguntar() {
    let dir = repo_maven_puro();
    let out = inspect(dir.path(), &[]);

    assert_ne!(
        out.status, 0,
        "sin preguntar, un repo que solo tiene `pom.xml` no declara version y no \
         es target. Si esto pasara, alguien habria convertido `--evaluate-build` \
         en un default: stdout={} stderr={}",
        out.stdout, out.stderr
    );
    assert!(
        out.stderr.contains("no release target"),
        "y el motivo tiene que decir que no hay target, no que la herramienta \
         fallo: {}",
        out.stderr
    );
}

/// Y el dialecto equivocado tampoco digitaliza un repo que no es suyo.
#[test]
fn un_repo_maven_puro_no_se_digitaliza_preguntando_a_gradle() {
    let dir = repo_maven_puro();
    let out = inspect(dir.path(), &["--evaluate-build", "--build-tool", "gradle"]);

    assert_ne!(
        out.status, 0,
        "preguntar a Gradle por un repo Maven no produce una version: Gradle no \
         tiene nada que abrir ahi. Antes de este arreglo la respuesta habria \
         sido un valor atribuido a un fichero que Gradle nunca abrio.\n{}",
        out.stdout
    );
}
