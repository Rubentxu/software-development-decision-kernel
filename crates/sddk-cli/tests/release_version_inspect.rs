//! El informe de `release version inspect`, en la forma en que lo lee una persona.
//!
//! # Qué está midiendo y por qué aquí
//!
//! La lógica de la inspección vive en el dominio y su falsador está en
//! `version_inspection_falsification.rs`. Lo que se mide **aquí** es una
//! decisión que no es del dominio: **cómo se reparte en secciones un informe
//! que es correcto**.
//!
//! MEDIDO antes de que esta prueba existiera: el informe era una sola lista con
//! las catorce providers del registry, de modo que un conflicto entre dos
//! declaraciones —las dos únicas líneas que contestaban— salía debajo de doce
//! «CMakeLists.txt no esta en este target». El informe era correcto y el que lo
//! leía no llegaba a la respuesta.
//!
//! La separación que se fija aquí es por `level` y no por «trae version»:
//! `Invalid` no trae versión y se queda arriba, porque fallo cerrado es una
//! respuesta y esconderlo bajo las ausencias sería esconder el único motivo por
//! el que a este comando le interessaría un código de salida distinto.
//!
//! **No se llama `e2e` a propósito:** corre por `run_from`, en proceso. No hay
//! binario en el otro lado, y el nombre tiene que decir lo que hay.

use std::fs;
use std::path::Path;

use sddk_cli::run_from;
use tempfile::TempDir;

/// Un target con dos declaraciones que **coinciden**, más todos los ficheros de
/// build que este repositorio no tiene.
fn agreeing_repo() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"demo","version":"2.1.0"}"#,
    )
    .expect("write package.json");
    fs::write(dir.path().join("gradle.properties"), "version=2.1.0\n").expect("write gradle");
    dir
}

/// El mismo repositorio con las dos declaraciones en **discrepancia**.
fn conflicting_repo() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"demo","version":"2.1.0"}"#,
    )
    .expect("write package.json");
    fs::write(dir.path().join("gradle.properties"), "version=9.9.9\n").expect("write gradle");
    dir
}

/// Un repositorio sin ninguna declaración.
fn silent_repo() -> TempDir {
    TempDir::new().expect("tempdir")
}

fn inspect(root: &Path) -> String {
    let out = run_from([
        "sddk",
        "release",
        "version",
        "inspect",
        "--root",
        root.to_str().expect("utf-8 root"),
        "--scope",
        ".",
    ]);
    assert_eq!(
        out.status, 0,
        "la inspección se hizo, así que sale con 0 aunque el veredicto sea Conflict: \
         «la inspección falló» es un hecho distinto de «hay dos declaraciones que \
         discrepan». stderr: {}",
        out.stderr
    );
    out.stdout
}

/// La sección que empieza en `header`, hasta la siguiente línea en blanco.
fn section<'a>(text: &'a str, header: &str) -> &'a str {
    let start = text
        .find(header)
        .unwrap_or_else(|| panic!("falta la sección `{header}` en:\n{text}"));
    let rest = &text[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn las_ausencias_no_enterran_a_las_que_si_respondieron() {
    let dir = agreeing_repo();
    let text = inspect(dir.path());

    let observations = section(&text, "observations:\n");
    let absences = section(&text, "consulted_and_found_nothing:");

    assert!(
        observations.contains("package.json [Observed] 2.1.0"),
        "la declaración tiene que estar entre las que respondieron:\n{observations}"
    );
    assert!(
        observations.contains("gradle.properties [Observed] 2.1.0"),
        "las dos declaraciones coinciden y las dos cuentan:\n{observations}"
    );
    assert!(
        !absences.contains("package.json"),
        "un fichero que declaró una versión no puede aparecer también como ausencia \
         — eso sería decir dos veces lo contrario de lo mismo:\n{absences}"
    );
    assert!(
        !observations.contains("CMakeLists.txt"),
        "«el fichero no está aquí» no es una observación sobre la versión: es una \
         ausencia, y por eso va en su propia sección:\n{observations}"
    );
}

#[test]
fn el_conflicto_se_lee_sin_pasar_por_doce_ausencias() {
    let dir = conflicting_repo();
    let text = inspect(dir.path());

    let observations = section(&text, "observations:\n");
    assert!(
        observations.contains("package.json [Observed] 2.1.0")
            && observations.contains("gradle.properties [Observed] 9.9.9"),
        "las dos declaraciones en conflicto tienen que estar arriba y completas, \
         con su valor y su sitio:\n{observations}"
    );
    assert!(text.contains("authority: Conflict"), "{text}");
    assert!(
        text.contains("productVersion: none"),
        "un conflicto no elige ganador, y `none` es lo que dice:\n{text}"
    );
}

#[test]
fn un_repositorio_sin_declaraciones_no_llega_a_informe_y_dice_por_que() {
    // DECISIÓN, no omisión: este caso **no** produce un informe, y sale con 1.
    //
    // Se podría «arreglar» haciendo `VersionInspection::target` opcional y
    // emitiendo un informe sin target. Medido el coste: `target` es el campo que
    // todos los lectores del informe usan para saber **sobre qué** se habló, y
    // volverlo opcional obliga a cada uno a tratar un caso que ya tiene un
    // mensaje que dice la causa exacta —«la raiz no declara version»— y que
    // `release plan` devuelve igual. El arreglo debilita el tipo para todos a
    // cambio de uno que ya responde.
    //
    // Y el código de salida no cero es lo correcto: aquí **no hay versión**, y
    // un agente que se llevara un 0 leería «todo bien» de un repositorio que no
    // declara nada. Salir distinto es aquí la honestidad, no la torpeza.
    let dir = silent_repo();
    let out = run_from([
        "sddk",
        "release",
        "version",
        "inspect",
        "--root",
        dir.path().to_str().expect("utf-8 root"),
        "--scope",
        ".",
    ]);
    assert_eq!(
        out.status, 1,
        "sin declaración no hay versión, y eso no es un informe con veredicto: \
         es un fallo. stdout: {}",
        out.stdout
    );
    assert!(
        out.stderr.contains("no release target found"),
        "y el motivo tiene que ser el motivo: stdout: {} stderr: {}",
        out.stdout,
        out.stderr
    );
}

#[test]
fn el_cierre_no_repite_lo_que_ya_dice_not_checked() {
    let dir = agreeing_repo();
    let text = inspect(dir.path());

    assert!(
        !text.contains("note:"),
        "la primera versión cerraba con un `note:` escrito a mano que decía, \
         palabra por palabra, el segundo punto de `NOT_CHECKED`. Es decir: una \
         frase sobre lo que no se comprobó, escrita al lado de la lista de lo que \
         no se comprobó que se deriva. Dos sitios, una afirmación, y el día que \
         el reducer cambie uno se desactualiza sin que nada lo note:\n{text}"
    );

    // Y lo que sí tiene que estar: el bloque derivado, con su separador.
    let not_checked = section(&text, "NOT_CHECKED:\n");
    for esperada in [
        "the release reference was NOT compared",
        "nothing was certified",
        "the provider set is the one SDDK shipped",
    ] {
        assert!(
            not_checked.contains(esperada),
            "falta «{esperada}» en el bloque NOT_CHECKED:\n{not_checked}"
        );
    }
    // El separador: lo que hay y lo que no, separados por una línea en blanco.
    assert!(
        text.contains("\nauthority: "),
        "la línea en blanco antes de `authority` es lo que impide que un rechazo \
         se lea como el resultado:\n{text}"
    );
}

#[test]
fn el_json_no_pierde_ninguna_fila_que_el_texto_aparta() {
    let dir = conflicting_repo();
    let root = dir.path().to_str().expect("utf-8 root");
    let out = run_from([
        "sddk", "release", "version", "inspect", "--root", root, "--scope", ".", "--format", "json",
    ]);
    assert_eq!(out.status, 0, "stderr: {}", out.stderr);

    let json: serde_json::Value = serde_json::from_str(&out.stdout).expect("json valido");
    let findings = json["findings"].as_array().expect("findings es una lista");
    let answering = json["providers_answering"].as_array().expect("lista");

    // El texto aparta las ausencias a su propia sección; el JSON no puede
    // perderlas, porque una máquina no lee secciones.
    let ausencias = findings
        .iter()
        .filter(|f| f["level"] == "NOT_APPLICABLE")
        .count();
    assert!(
        ausencias > 0,
        "el registry trae providers de muchas tecnologías, así que una lista sin \
         ausencias significa que el JSON las perdió:\n{}",
        out.stdout
    );
    assert_eq!(
        findings.len(),
        answering.len() + ausencias,
        "cada provider consultado está o en los que contestaron o entre las \
         ausencias, y nowhere más"
    );
}
