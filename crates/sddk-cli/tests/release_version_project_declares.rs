//! Un proyecto cuya tecnología SDDK no conoce puede publicar si se declara.
//!
//! # Qué se está midiendo
//!
//! Que la tabla de trece ficheros de `DEFAULT_DECLARATIONS` **no sea la única
//! puerta**, y que hacerlo no haya roto lo que ya pasaba por ella.
//!
//! MEDIDO antes del bloque, y estos son los dos hechos que lo motivaron:
//!
//! 1. Un repo con un `build.sbt` —Scala, fuera de las trece filas— y
//!    `.sddk/version-source.json` diciendo `{"authority":"version","version":"4.2.0"}`:
//!    `error: VERSION TARGET ERROR: no release target found`.
//! 2. Peor: un repo que declara `4.2.0` en `Cargo.toml` **y** en el JSON quedaba
//!    `authority: Invalid` y **no publicaba**. El `Invalid` de una forma que el
//!    build no entendía **tapaba** la `Declared` buena. Un fichero que el
//!    proyecto escribe para ser más explícito no puede dejar de ser legible por
//!    no caber en lo que el build entendía.
//!
//! **Hermético:** todo ocurre en `TempDir`s, sin red y sin herramientas externas.

use std::fs;
use std::path::Path;

use sddk_cli::{CommandOutput, run_from};
use tempfile::TempDir;

/// Scala: un `build.sbt` que ninguna de las trece filas sabe leer.
fn repo_escala(declaracion: Option<&str>) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("build.sbt"), "name := \"acme\"\n").expect("write sbt");
    if let Some(cuerpo) = declaracion {
        fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
        fs::write(dir.path().join(".sddk/version-source.json"), cuerpo).expect("write decl");
    }
    dir
}

/// Rust: un `Cargo.toml`, que sí está en la tabla.
fn repo_rust(declaracion: Option<&str>, version: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("Cargo.toml"),
        format!("[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"{version}\"\n"),
    )
    .expect("write cargo");
    if let Some(cuerpo) = declaracion {
        fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
        fs::write(dir.path().join(".sddk/version-source.json"), cuerpo).expect("write decl");
    }
    dir
}

fn salida(dir: &Path, args: &[&str]) -> CommandOutput {
    let mut todos: Vec<String> = vec![
        "sddk".into(),
        "release".into(),
        "version".into(),
        args[0].into(),
        "--root".into(),
        dir.to_str().expect("utf-8").into(),
        "--scope".into(),
        ".".into(),
    ];
    todos.extend(args[1..].iter().map(|s| (*s).to_owned()));
    let refs: Vec<&str> = todos.iter().map(String::as_str).collect();
    run_from(refs)
}

fn matches(dir: &Path, tag: &str) -> CommandOutput {
    salida(dir, &["matches", "--tag", tag])
}

fn inspect(dir: &Path) -> CommandOutput {
    salida(dir, &["inspect"])
}

/// La puerta de verdad: `release plan`, que es la que autoriza.
///
/// `matches` e `inspect` son **informes**: salen con 0 y dicen lo que han visto
/// en el cuerpo. MEDIDO: los tres primeros tests de este fichero fallaron
/// porque medían el código de salida de un diagnóstico, que es 0 con el
/// veredicto rojo. Una puerta medida por el informe de la puerta no mide la
/// puerta.
fn puerta(dir: &Path, tag: &str) -> CommandOutput {
    let todos: Vec<String> = vec![
        "sddk".into(),
        "release".into(),
        "plan".into(),
        "--root".into(),
        dir.to_str().expect("utf-8").into(),
        "--scope".into(),
        ".".into(),
        "--tag".into(),
        tag.into(),
        "--route".into(),
        "forge".into(),
        "--repo".into(),
        "owner/project".into(),
        "--fallback-seed".into(),
        "11111111-1111-4111-8111-111111111111".into(),
    ];
    let refs: Vec<&str> = todos.iter().map(String::as_str).collect();
    run_from(refs)
}

fn stdout_de(out: &CommandOutput) -> &str {
    assert_eq!(
        out.status, 0,
        "el comando tiene que salir con 0 para poder medir lo que responde: {}",
        out.stderr
    );
    &out.stdout
}

// ── La capacidad: una tecnología que SDDK no conoce publica ────────────────

#[test]
fn un_proyecto_escala_publica_por_declaracion() {
    let dir = repo_escala(Some(
        r#"{"schema_version":1,"authority":"version","version":"4.2.0"}"#,
    ));
    let out = matches(dir.path(), "v4.2.0");

    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(
        stdout_de(&out).contains("matches: true"),
        "un proyecto que declara su version tiene que poder publicar, y este \
         ecosistema no aparece en ninguna de las trece filas:\n{}",
        out.stdout
    );
}

#[test]
fn la_inspeccion_dice_que_la_trajo_la_declaracion_del_proyecto() {
    let dir = repo_escala(Some(
        r#"{"schema_version":1,"authority":"version","version":"4.2.0"}"#,
    ));
    let out = inspect(dir.path());
    let texto = stdout_de(&out);

    assert!(
        texto.contains("productVersion: 4.2.0"),
        "el valor es el que el proyecto declaro:\n{texto}"
    );
    assert!(
        texto.contains("declaracion-del-proyecto") || texto.contains("version-source.json"),
        "y el informe dice DE QUIEN es la respuesta, porque un valor sin saber de \
         quien es no se puede comprobar:\n{texto}"
    );
}

// ── La no-regresión que el PRE-FLIGHT declara como riesgo ──────────────────

#[test]
fn un_proyecto_de_la_tabla_que_tampoco_se_declara_sigue_publicando() {
    let dir = repo_rust(None, "1.0.0");
    let out = matches(dir.path(), "v1.0.0");

    assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    assert!(
        stdout_de(&out).contains("matches: true"),
        "anadir una forma de declarar no puede romper la que ya funcionaba, que \
         es el riesgo que el PRE-FLIGHT de este bloque declara:\n{}",
        out.stdout
    );
}

#[test]
fn declararse_no_es_obligatorio_y_eso_cuesta() {
    let dir = repo_escala(None);
    let out = inspect(dir.path());

    assert_ne!(
        out.status, 0,
        "sin declararse, un proyecto de una tecnologia que SDDK no conoce no \
         publica. Es el coste declarado de esta capacidad, y por eso no hay \
         default:\n{}",
        out.stdout
    );
    assert!(
        out.stderr.contains("no release target"),
        "y el motivo tiene que decir que no hay target, no que la herramienta \
         fallo:\n{}",
        out.stderr
    );
}

// ── La ley: declarar no es ganar ──────────────────────────────────────────

#[test]
fn declarar_una_cosa_que_el_resto_desmiente_cierra_la_puerta() {
    let dir = repo_rust(
        Some(r#"{"schema_version":1,"authority":"version","version":"9.9.9"}"#),
        "4.2.0",
    );

    let inspector = inspect(dir.path());
    assert!(
        stdout_de(&inspector).contains("Conflict") || stdout_de(&inspector).contains("Ambiguous"),
        "una contradiccion no se resuelve eligiendo una: se nombra.\n{}",
        inspector.stdout
    );

    let cerrada = puerta(dir.path(), "v4.2.0");
    assert_ne!(
        cerrada.status, 0,
        "y una contradiccion no autoriza un release: la puerta tiene que cerrar."
    );
    assert!(
        cerrada.stderr.contains("disagree"),
        "el rechazo tiene que decir QUE discrepa, porque el operador tiene que ver \
         las dos declaraciones para poder decidir — que es justo lo que sddk se \
         niega a hacer por el:\n{}",
        cerrada.stderr
    );
    assert!(
        cerrada.stderr.contains("Cargo.toml") && cerrada.stderr.contains("version-source.json"),
        "y tiene que NOMBRAR LAS DOS, no solo la primera: un conflicto que solo \
         enseña un lado es indistinguible de que el otro no exista.\n{}",
        cerrada.stderr
    );
}

#[test]
fn declarar_lo_mismo_que_el_fichero_es_un_cruze_y_no_una_redundancia() {
    let dir = repo_rust(
        Some(r#"{"schema_version":1,"authority":"version","version":"4.2.0"}"#),
        "4.2.0",
    );
    let out = inspect(dir.path());

    assert!(
        stdout_de(&out).contains("CrossValidated"),
        "el JSON del proyecto y su `Cargo.toml` son dos fuentes independientes: que \
         coincidan es MAS evidencia, y degradarlo a una sola declaracion seria \
         descartar la mitad:\n{}",
        out.stdout
    );
}

// ── Un fichero mal escrito no puede tapar lo que sí funciona ───────────────

#[test]
fn una_declaracion_que_no_se_entiende_no_tapa_a_la_fila_buena() {
    // Este es el defecto MEDIDO que motivó el bloque: el `Invalid` de la forma
    // no entendida tapaba la `Declared` del `Cargo.toml` y la puerta rechazaba.
    let dir = repo_rust(
        Some(r#"{"schema_version":1,"authority":"no-existe","version":"4.2.0"}"#),
        "4.2.0",
    );
    let inspector = inspect(dir.path());
    assert!(
        stdout_de(&inspector).contains("Invalid"),
        "una forma que no existe es un error del proyecto y tiene que seguir \
         fallando cerrado: no se arregla eso silenciandolo.\n{}",
        inspector.stdout
    );

    let cerrada = puerta(dir.path(), "v4.2.0");
    assert_ne!(
        cerrada.status, 0,
        "y la puerta tiene que seguir cerrada: `Invalid` manda sobre `Declared` \
         porque un fichero que existe y no se entiende no es lo mismo que un \
         fichero que no esta"
    );
}

#[test]
fn un_esquema_que_este_build_no_entiende_lo_dice() {
    let dir = repo_escala(Some(
        r#"{"schema_version":99,"authority":"version","version":"4.2.0"}"#,
    ));
    // El camino con `schema_version` incompatible NO llega a inspeccionar: no hay
    // target que inspeccionar, porque la version que declara no se puede leer.
    // Por eso esto mira `stderr` y no la salida de un informe.
    let inspector = inspect(dir.path());
    assert_ne!(
        inspector.status, 0,
        "un schema que este build no entiende no produce version:\n{}",
        inspector.stdout
    );
    assert!(
        inspector.stderr.contains("schema_version"),
        "el motivo tiene que nombrar el campo que no encaja, porque «algo fallo» \
         con catorce providers de fondo no dice nada:\n{}",
        inspector.stderr
    );

    // Y esta es la ley que el arreglo de este bloque hizo cierta: el mensaje
    // tiene que distinguir «no declaro» de «declare y no lo lei».
    assert!(
        !inspector.stderr.contains("la raiz no declara version"),
        "MEDIDO, antes del arreglo: decia exactamente eso cuando la raiz SI \
         declaraba version. Un mensaje que manda a buscar un fichero que no \
         falta es la misma clase que el banner de Gradle:\n{}",
        inspector.stderr
    );

    let cerrada = puerta(dir.path(), "v4.2.0");
    assert_ne!(
        cerrada.status, 0,
        "y la puerta no puede autorizar sobre un contrato que no entiende"
    );
    assert!(
        cerrada.stderr.contains("no la pudo leer"),
        "y el rechazo tiene que decir que se leyo y no se pudo leer, no que no \
         hay nada declarado:\n{}",
        cerrada.stderr
    );
}
