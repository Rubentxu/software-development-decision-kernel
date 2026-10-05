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
fn con_la_bandera_se_registra_el_dialecto_que_se_pidio() {
    // Lo que este test afirmaba —«la version que trae el informe es la que
    // contesta la herramienta»— exijo meter un binario falso por `--build-tool`.
    // Esa via ya no existe, y no se pierde: es lo que el arreglo quito, porque un
    // path no es un dialecto.
    //
    // Lo que SI se puede afirmar desde la CLI es que la bandera registra el
    // dialecto pedido y que la linea `NOT_CHECKED` desaparece con ella. Que la
    // herramienta responda, y que su respuesta se atribuya al fichero SUYO, se
    // verifica donde se puede sustituir el binario: en
    // `crates/sddk-gateway/tests/version_build_model_contract.rs`.
    let (dir, _tool) = repo();
    let out = inspect(dir.path(), &["--evaluate-build", "--build-tool", "gradle"]);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);

    assert!(
        out.stdout.contains("sddk.gateway.build-model/gradle"),
        "la bandera registra el dialecto que se pidio, y el informe lo nombra \
         aunque no conteste: un provider mirado y no se sabe como responde se \
         distingue del que ni se miro.\n{}",
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
    // Lo que este test afirmaba —«un path que no existe es una fuente que no se
    // pudo leer, veredicto Invalid, salida 0»— era correcto para una capa que
    // aceptaba cualquier path. Ya no se puede expresar desde aqui, y no es una
    // perdida: es exactamente lo que el arreglo quito.
    //
    // Un path no es un dialecto, así que ahora es un error de la LINEA DE
    // COMANDOS, antes de que exista ningun provider. Y esa distincion es mejor
    // que la que sustituye: «escribiste mal el nombre» y «la herramienta fallo»
    // son dos reparaciones distintas, y antes las dos salian por el mismo texto.
    let (dir, _tool) = repo();
    let out = inspect(
        dir.path(),
        &[
            "--evaluate-build",
            "--build-tool",
            "/nonexistent/sddk-build-tool",
        ],
    );
    assert_ne!(
        out.status, 0,
        "un path no es un dialecto conocido, y eso falla antes de preguntar: \
         stdout={} stderr={}",
        out.stdout, out.stderr
    );
    assert!(
        out.stderr.contains("unknown build tool"),
        "el error tiene que decir que el nombre no se sabe, no que la fuente no \
         se pudo leer: {}",
        out.stderr
    );
    assert!(
        out.stderr.contains("gradle, maven"),
        "y tiene que decir cuales SI se saben preguntar: {}",
        out.stderr
    );
    assert!(
        !out.stdout.contains("Invalid"),
        "y no puede haber veredicto: no se llego a inspeccionar nada.\n{}",
        out.stdout
    );
}
