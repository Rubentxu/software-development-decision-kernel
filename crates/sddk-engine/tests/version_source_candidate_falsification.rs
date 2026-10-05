//! Falsador del modelo de candidatos de version (INC-DEBT-078).
//!
//! Corre contra el codigo REAL de `sddk_engine::version_source`, no contra
//! una copia: usa los mismos helpers que los tests que ya viven en el
//! modulo (`write` y `version_of` se replican aqui solo porque son
//! privados del `mod tests`, y la superficie que se ejercita —
//! `resolve_project_version` — es la de produccion).
//!
//! QUE PREGUNTA, y por que son cuatro casos y no uno
//!
//! 1. ¿Un candidato que EXISTE pero NO DECLARA detiene la resolucion?
//!    Si la detiene, entonces el segundo y el tercer fichero de la lista
//!    no se miran nunca, y tampoco los ecosistemas que vienen despues en
//!    el registro: un solo `gradle.properties` sin `version=` apaga la
//!    resolucion del proyecto entero.
//! 2. ¿Un Python `setup.py` se parsea como TOML? El registro declara
//!    `manifest_paths: ["pyproject.toml", "setup.py"]` con UN solo
//!    `ManifestFormat::Toml`, luego un `setup.py` se lee con el parser de
//!    TOML. No es un caso raro: es la forma habitual de un proyecto Python
//!    sin `pyproject.toml`.
//! 3. ¿La version declarada por OTRO ecosistema sobrevive? Es el que
//!    separa "este repositorio esta roto" de "este repositorio tiene un
//!    fichero que no es lo que el registro cree".
//! 4. ¿La ley nueva relaja algo que DEBIA seguir fallando cerrado? Un
//!    fichero corrupto, o dos candidatos que declaran distinto, tienen que
//!    seguir dando fallo. Sin este caso, "fail closed, but not fail first"
//!    podria haberse escrito como "fail never".

use std::fs::File;
use std::io::Write;
use std::path::Path;

use sddk_engine::version_source::{resolve_project_version, VersionAuthority};

fn write(dir: &Path, name: &str, body: &str) {
    let mut f = File::create(dir.join(name)).unwrap();
    writeln!(f, "{body}").unwrap();
}

/// Resuelve y devuelve el veredicto como texto, para que el fallo diga QUE
/// paso y no solo que algo no fue como se esperaba.
fn verdict(dir: &Path) -> String {
    match resolve_project_version(dir) {
        Ok(VersionAuthority::CrossChecked { version, .. }) => format!("RESUELTO {version}"),
        Ok(VersionAuthority::TagIsTheOnlyAuthority { ecosystems }) => {
            format!("SOLO-TAG {ecosystems:?}")
        }
        Ok(other) => format!("OK-OTRA {other:?}"),
        // Se compara sobre el MENSAJE, no sobre el codigo: `VersionSourceError`
        // es un enum `thiserror` sin accessor de codigo, y el mensaje es
        // ademas lo que ve el operador cuando esto falla de verdad.
        Err(e) => format!("ERROR {e}"),
    }
}

fn arena() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// ── CASO 1: presente-pero-no-declara NO debe detener la resolucion ────────

/// Un `gradle.properties` legítimo que no declara version —solo configuracion
/// de Gradle— junto a un `build.gradle.kts` que sí la declara.
///
/// ## Lo que se afirma, y lo que NO
///
/// Antes de este arreglo este caso fallaba con «could not find `version` of
/// .../gradle.properties»: el `?` abortaba la resolución y el
/// `build.gradle.kts` —el siguiente candidato de la MISMA lista— no se leía
/// nunca. Eso es lo que se comprueba aquí.
///
/// Lo que **no** se afirma es que la resolución devuelva `0.47.0`. Y no la
/// devuelve, a propósito: `build.gradle.kts` está registrado como candidato
/// SIN extractor, porque su versión solo es observable EVALUANDO el modelo de
/// Gradle, y SDDK no interpreta Kotlin con un regex. Resolverlo de verdad es
/// un provider que consulta la herramienta (bloque aparte), y hasta que exista
/// la respuesta honesta es «lo encontré y no lo sé leer», no un 0.47.0
/// fabricado.
///
/// La diferencia entre un abort y un diagnóstico que nombra los dos ficheros
/// es exactamente la que separa un fallo de una afirmación.
#[test]
fn un_candidato_presente_sin_declarar_no_detiene_la_resolucion() {
    let d = arena();
    write(d.path(), "gradle.properties", "org.gradle.caching=true\nkotlin.code.style=official");
    write(d.path(), "build.gradle.kts", "version = \"0.47.0\"");
    let v = verdict(d.path());

    // 1. Ya no aborta en el primero. El síntoma viejo nombraba el locator.
    assert!(
        !v.contains("could not find `version`"),
        "el candidato presente-pero-sin-declarar sigue abortando la resolución: {v}"
    );
    // 2. Llega hasta el final y dice qué encontró, que es lo que permite
    //    arreglarlo sin adivinar.
    assert!(
        v.contains("presentes-pero-sin-declarar"),
        "sin lista de presentes-pero-no-declarar el diagnóstico no dice qué falta: {v}"
    );
    assert!(
        v.contains("gradle.properties") && v.contains("build.gradle.kts"),
        "el diagnóstico tiene que NOMBRAR los dos, no decir «no encontrado»: {v}"
    );
    // 3. Y no inventa la versión del fichero que sabe leer.
    assert!(
        !v.contains("0.47.0"),
        "no se debe fabricar la versión de un fichero que no se sabe leer: {v}"
    );
}

/// El caso de PipelineK, en su forma COMPLETA, es de un bloque posterior:
/// un provider que evalúa el modelo de Gradle. Lo que se fija aquí es que la
/// infraestructura no lo impide: `gradle.properties` presente y sin versión
/// no bloquea a OTRO candidato que sí sepa leerse. Es el mismo defecto con un
/// final resoluble, y por eso no depende del provider que todavía no existe.
#[test]
fn un_candidato_sin_version_no_bloquea_a_otro_que_si_declara() {
    let d = arena();
    write(d.path(), "gradle.properties", "org.gradle.caching=true");
    write(d.path(), "package.json", r#"{"name":"x","version":"3.4.0"}"#);
    let v = verdict(d.path());
    assert!(
        v.contains("3.4.0"),
        "un fichero de configuracion que no declara version no puede impedir que \
         otro candidato resuelva: {v}"
    );
}

// ── CASO 2: el formato pertenece al CANDIDATO, no al ecosistema ──────────

/// `setup.py` es Python. El registro lo declara con `ManifestFormat::Toml`
/// porque comparte lista con `pyproject.toml`. Este caso es el que separa
/// "hay un segundo fichero" de "el segundo fichero se lee con el formato
/// del primero".
#[test]
fn setup_py_no_se_parsea_como_toml() {
    let d = arena();
    write(
        d.path(),
        "setup.py",
        "from setuptools import setup\nsetup(name='x', version='2.5.1')",
    );
    let v = verdict(d.path());
    // No exigimos 2.5.1: un resolver de Python leido por regex seria su
    // propio invento. Exigimos que NO sea un fallo de parseo, que es lo
    // que el formato compartido provoca hoy.
    assert!(
        !v.contains("could not be parsed") && !v.contains("unparsable"),
        "setup.py se esta parseando con el formato del ecosistema, no con el suyo: {v}"
    );
}

// ── CASO 3: lo que otro ecosistema ya declaro sobrevive ───────────────────

/// El caso que hace Valuable el defecto. `typescript` esta ANTES que
/// `python` en el registro, luego `package.json` ya ha declarado cuando el
/// bucle llega a `setup.py`. Con el `?` actual, el `Unparsable` de
/// `setup.py` aborta la resolucion ENTERA y se pierde la version que si se
/// habia encontrado. Es decir: un fichero irrelevante puede borrar un dato
/// ya obtenido.
#[test]
fn un_fichero_irrelevante_no_destruye_lo_que_ya_se_declaro() {
    let d = arena();
    write(d.path(), "package.json", r#"{"name":"x","version":"3.4.0"}"#);
    write(
        d.path(),
        "setup.py",
        "from setuptools import setup\nsetup(name='x', version='2.5.1')",
    );
    let v = verdict(d.path());
    assert!(
        v.contains("3.4.0"),
        "package.json declaro 3.4.0 antes de que el bucle llegara a setup.py, y la resolucion dio: {v}"
    );
}

// ── CASO 4: lo que DEBIA seguir fallando cerrado, sigue fallando ─────────

/// La ley es "fail closed, but not fail first". Si "no me declaro" pasa a
/// ser "no me importa", un fichero CORRUPTO pasaria a ser tambien
/// ignorable, y eso seria exactamente lo contrario de fail-closed.
#[test]
fn un_manifesto_corrupto_sigue_siendo_fallo_cerrado() {
    let d = arena();
    write(d.path(), "package.json", "{ esto no es json ");
    let v = verdict(d.path());
    assert!(
        v.contains("ERROR"),
        "un package.json corrupto tiene que ser un fallo, no un 'no me declara': {v}"
    );
}

/// Y el otro: dos candidatos que SI declaran, y declaran distinto, es
/// divergencia real. Si esto pasara a resolverse por precedencia, la
/// precedencia estaria decidiendo una contradiccion.
#[test]
fn dos_candidatos_que_declaran_distinto_siguen_siendo_divergentes() {
    let d = arena();
    write(d.path(), "package.json", r#"{"name":"x","version":"3.4.0"}"#);
    write(d.path(), "pyproject.toml", "[project]\nname = \"y\"\nversion = \"8.1.0\"\n");
    let v = verdict(d.path());
    assert!(
        v.contains("DIVERGENT") || v.contains("ERROR"),
        "3.4.0 y 8.1.0 son dos productos distintos y por tanto divergencia dentro de una misma autoridad, no una precedencia: {v}"
    );
}
