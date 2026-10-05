//! `--evaluate-build`: lo que cuesta, lo que se gana, y lo que se declara.
//!
//! # Qué se está midiendo
//!
//! Que preguntar al build tool sea una **decisión**, y que el informe diga si
//! se tomó.
//!
//! MEDIDO: `gradle properties --offline` tarda **3 segundos** y levanta una JVM,
//! por target. Eso no es un detalle de implementación, es lo que hace que la
//! bandera sea obligatoria en lugar de un default — y lo que hace que decir
//! «no se preguntó» sea parte del informe y no una ausencia de él.
//!
//! **Hermético:** se usa una herramienta falsa. Un test que necesita una JVM
//! para comprobar una bandera es un test que no se ejecuta en la mitad de las
//! máquinas.

use std::fs;
use std::path::{Path, PathBuf};

use sddk_cli::{CommandOutput, run_from};
use tempfile::TempDir;

/// Una herramienta falsa, y un target que SI es un build de Gradle.
fn repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("build.gradle"), "version = '1.2.3'\n").expect("write build");
    let tool = dir.path().join("fake-build-tool");
    fs::write(
        &tool,
        "#!/bin/sh\ncat <<'SDDK_EOF'\nRoot project 'probe'\nversion: 1.2.3\nSDDK_EOF\nexit 0\n",
    )
    .expect("write tool");
    make_executable(&tool);
    (dir, tool)
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

#[test]
fn sin_la_bandera_el_informe_declara_que_no_se_pregunto() {
    let (dir, _tool) = repo();
    let out = inspect(dir.path(), &[]);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);

    assert!(
        out.stdout.contains("the build tool was NOT asked"),
        "no preguntar al build tool es una decision —la paga quien pide la \
         bandera— y una decision que el informe no menciona se lee como una \
         fuente mas que se consulto:\n{}",
        out.stdout
    );
    assert!(
        !out.stdout.contains("build-model/"),
        "y sin la bandera el provider ni siquiera se registra: no puede haber \
        ejecutado por lo que no se lanzo:\n{}",
        out.stdout
    );
}

#[test]
fn con_la_bandera_la_version_llega_de_la_herramienta() {
    let (dir, tool) = repo();
    let out = inspect(
        dir.path(),
        &[
            "--evaluate-build",
            "--build-tool",
            tool.to_str().expect("utf-8"),
        ],
    );
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);

    assert!(
        out.stdout.contains("productVersion: 1.2.3"),
        "la version es la que contesta la herramienta:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("build-model/"),
        "y el informe nombra al provider que la trajo, porque un valor sin saber \
         de quien es no se puede comprobar:\n{}",
        out.stdout
    );
    assert!(
        !out.stdout.contains("the build tool was NOT asked"),
        "preguntado es preguntado: la linea de NOT_CHECKED se va cuando se \
         pregunta, porque si se queda el informe se contradice a si mismo:\n{}",
        out.stdout
    );
}

#[test]
fn una_herramienta_que_no_existe_no_se_confunde_con_una_que_falla() {
    let (dir, _tool) = repo();
    let out = inspect(
        dir.path(),
        &[
            "--evaluate-build",
            "--build-tool",
            "/nonexistent/sddk-build-tool",
        ],
    );
    assert_eq!(
        out.status, 0,
        "la inspeccion se hizo —aunque una fuente no se pudo leer— y eso sale con 0: \
         «la inspeccion fallo» es un hecho distinto de «una fuente fallo`. \
         stderr: {}",
        out.stderr
    );
    assert!(
        out.stdout.contains("Invalid"),
        "y el veredicto es Invalid, que falla cerrado:\n{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("no se pudo ejecutar"),
        "con NUESTRO motivo —no hay forma de que un programa inexistente diga \
         que no existe— y distinguido del de una herramienta que fallo:\n{}",
        out.stdout
    );
}
