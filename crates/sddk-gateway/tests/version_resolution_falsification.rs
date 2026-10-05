//! Falsador de la resolución de versión.
//!
//! # Una nota sobre una cita que se quitó
//!
//! Este fichero citaba `INC-DEBT-078` en su primera versión. No existe tal
//! fichero, y no existe porque **no hay esa deuda**: el defecto que ese
//! conjunto de casos mide —que un candidato presente sin declarar detenga la
//! resolución y se pierda lo que otro ya había declarado— se corrigió en el
//! mismo bloque que escribió este fichero, y por eso lo que toca es un
//! falsador, no un pendiente.
//!
//! Una cita a una deuda que no existe es una autoridad fantasma: un agente
//! que la encuentre irá a buscarla, no la encontrará, y tendrá que decidir por
//! su cuenta si existe. El coste de quitar la frase es cero y el de dejarla no.
//!
//! # QUÉ PREGUNTA, y por qué son seis casos y no uno
//!
//! # Corre contra el camino REAL de producción
//!
//! `default_version_registry()` reducido por el kernel y pasado por
//! `ensure_version_lockstep_detailed`, que es lo que llama `release plan`— y
//! no contra una copia ni contra un doble. Un falsador que ejercita otra cosa
//! que la que se publica mide la otra cosa.
//!
//! # Por qué vive aquí y no en el motor
//!
//! Este fichero mutaba el módulo que decidía. Ese módulo ya no existe: el
//! registro de nombres de fichero se fue al adapter y la decisión, al modelo
//! puro. Un falsador que se queda en el sitio viejo mientras el sujeto se
//! mudaría sería un fichero que sigue verde sin mirar nada, que es la forma
//! más común de que un control se convierta en decorado.
//!
//! Este fichero mutaba `sddk_engine::version_source`, el módulo que decidía. Ese
//! módulo ya no existe: el registro de nombres de fichero se fue al adapter y
//! la decisión, al modelo puro. Un falsador que se queda en el sitio viejo
//! mientras el sujeto se mudaría sería un fichero que sigue verde sin mirar
//! nada, que es la forma más común de que un control se convierta en decorado.
//!
//! # QUÉ PREGUNTA, y por qué son seis casos y no uno
//!
//! 1. ¿Un fichero que EXISTE pero NO DECLARA detiene la resolución? Si la
//!    detiene, el segundo y el tercer fichero de la lista no se miran nunca.
//! 2. ¿Un `setup.py` se lee con el formato de otro fichero? No es un caso
//!    raro: es la forma habitual de un proyecto Python sin `pyproject.toml`.
//! 3. ¿La versión declarada por otro mecanismo sobrevive a un fichero
//!    irrelevante? Es lo que separa «este repositorio está roto» de «este
//!    repositorio tiene un fichero que no es lo que esperábamos».
//! 4. ¿La ley relaja algo que DEBÍA seguir fallando cerrado?
//! 5. ¿Dos ficheros que declaran distinto se resuelven por precedencia?
//! 6. ¿Un target que declara que su versión la lleva la release ref sigue
//!    publicando, y se distingue de uno que no declaró nada?

use sddk_domain::release_ref::VersionNaming;
use sddk_domain::version_authority::{
    PRODUCT_VERSION_OBSERVATION, ReleaseTarget, VersionAuthority,
};
use sddk_engine::version::ensure_version_lockstep_detailed;
use sddk_gateway::version_provider::default_version_registry;
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

fn target(dir: &Path) -> ReleaseTarget {
    ReleaseTarget::at("repo", dir.display().to_string())
}

/// El veredicto como texto, para que un fallo diga QUÉ pasó.
///
/// Se compara sobre el mensaje y no sobre el código porque el mensaje es lo
/// que ve el operador cuando esto falla de verdad, y porque es donde viven las
/// distinciones que estos casos existen para fijar.
fn verdict(dir: &Path) -> String {
    let authority = default_version_registry().resolve(PRODUCT_VERSION_OBSERVATION, &target(dir));
    match &authority {
        VersionAuthority::Resolved { version, .. } => {
            format!("RESUELTO {version}")
        }
        VersionAuthority::CrossValidated { version, .. } => {
            format!("CRUZADO {version}")
        }
        VersionAuthority::ReleaseRefIsAuthority { declarations, .. } => {
            format!("SOLO-RELEASE-REF {declarations:?}")
        }
        VersionAuthority::Ambiguous { candidates, .. } => {
            format!("DIVERGENTE {candidates:?}")
        }
        VersionAuthority::Invalid { failures, .. } => format!("INVALIDO {failures:?}"),
        VersionAuthority::Unresolved { .. } => "SIN-RESOLVER".to_owned(),
    }
}

/// Lo mismo, pero por la puerta de verdad: la que usa `release plan`, con su
/// mensaje de rechazo. Es la mitad que importa, porque un veredicto correcto
/// con un mensaje que no dice nada sigue dejando al operador sin poder actuar.
fn refusal(dir: &Path, tag: &str) -> String {
    // La convencion la declara quien llama, y este es quien llama. Todos los
    // tags de la tabla llevan el prefijo `v` porque su sujeto es la resolucion
    // y no la forma del nombre, luego la unica naming bajo la cual nombran sus
    // versiones es la de prefijo.
    match ensure_version_lockstep_detailed(
        &default_version_registry(),
        &target(dir),
        tag,
        &VersionNaming::v_prefixed(),
    ) {
        Ok(authority) => format!("OK {authority:?}"),
        Err(e) => format!("ERROR {e}"),
    }
}

fn arena() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// ── CASO 1: presente-pero-no-declara NO detiene la resolución ──────────────

/// Un fichero de configuración legítimo que no declara versión junto a otro
/// que tampoco, sin nada que las declare.
///
/// ## Lo que se afirma, y lo que NO
///
/// Antes este caso fallaba con «could not find `version` of
/// .../gradle.properties»: la resolución abortaba en el primero y el segundo
/// fichero —el siguiente de la MISMA lista— no se leía nunca. Eso es lo que se
/// comprueba aquí: que llega al final y NOMBRA lo que encontró.
///
/// Lo que **no** se afirma es que aparezca una versión. El `build.gradle.kts`
/// está declarado como fichero que existe y no es fuente declarable, porque su
/// versión solo es observable EVALUANDO el modelo de Gradle y SDDK no
/// interpreta Kotlin con un regex. Resolverlo de verdad es un provider que
/// consulta la herramienta, y hasta que exista la respuesta honesta es «lo
/// encontré y no lo sé leer», no un 0.47.0 fabricado.
#[test]
fn un_fichero_presente_sin_declarar_no_detiene_la_resolucion() {
    let d = arena();
    write(
        d.path(),
        "gradle.properties",
        "org.gradle.caching=true\nkotlin.code.style=official",
    );
    write(d.path(), "build.gradle.kts", "version = \"0.47.0\"");

    let v = verdict(d.path());
    assert!(
        v.contains("SIN-RESOLVER"),
        "sin nada que declare no hay version, y eso no es un fallo de formato: {v}"
    );
    assert!(
        !v.contains("INVALIDO"),
        "un fichero que existe y no declara version NO es una fuente ilegible: {v}"
    );
    assert!(
        !v.contains("0.47.0"),
        "no se debe fabricar la versión de un fichero que no se sabe leer: {v}"
    );

    // Y el rechazo tiene que decir qué se buscó, no solo «no se encontró».
    let r = refusal(d.path(), "v1.0.0");
    assert!(r.contains("ERROR"), "{r}");
    assert!(
        r.contains("presentes-pero-sin-declarar"),
        "sin la lista de presentes-pero-no-declarar el diagnóstico no dice qué falta: {r}"
    );
    assert!(
        r.contains("gradle.properties") && r.contains("build.gradle.kts"),
        "el diagnóstico tiene que NOMBRAR los dos, no decir «no encontrado»: {r}"
    );
}

/// El caso del consumidor, en su forma completa: una configuración auxiliar que
/// no declara versión y, al lado, una autoridad legítima que sí.
#[test]
fn un_fichero_sin_version_no_bloquea_a_otro_que_si_declara() {
    let d = arena();
    write(d.path(), "gradle.properties", "org.gradle.caching=true");
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"3.4.0"}"#,
    );
    assert!(
        verdict(d.path()).contains("3.4.0"),
        "un fichero de configuracion que no declara version no puede impedir que \
         otro mecanismo resuelva"
    );
    assert!(
        refusal(d.path(), "v3.4.0").starts_with("OK"),
        "y la puerta local tiene que dejar pasar: {}",
        refusal(d.path(), "v3.4.0")
    );
}

// ── CASO 2: el formato pertenece al FICHERO, no al ecosistema ─────────────

/// `setup.py` es Python. El registro antiguo lo declaraba con el formato de su
/// compañero de lista, y por eso se leía con un parser que no era el suyo.
/// Este caso separa «hay un segundo fichero» de «el segundo fichero se lee con
/// el formato del primero».
#[test]
fn setup_py_no_se_parsea_con_el_formato_de_otro() {
    let d = arena();
    write(
        d.path(),
        "setup.py",
        "from setuptools import setup\nsetup(name='x', version='2.5.1')",
    );
    let v = verdict(d.path());
    // No exigimos 2.5.1: un lector de Python hecho con regex seria su propio
    // invento, que es el mismo defecto con otro disfraz. Exigimos que NO sea
    // un fallo de parseo, que es lo que el formato compartido provocaba.
    assert!(
        !v.contains("INVALIDO"),
        "setup.py se esta interpretando con el formato de otro fichero: {v}"
    );
}

// ── CASO 3: lo que otro ya declaró sobrevive ──────────────────────────────

/// El caso que hace valuable el defecto. Con el `?` de antes, el fallo de
/// parseo de un fichero irrelevante abortaba la resolución ENTERA y se perdía
/// la versión que ya se había encontrado: un fichero que no tiene nada que ver
/// podía borrar un dato ya obtenido.
#[test]
fn un_fichero_irrelevante_no_destruye_lo_que_ya_se_declaro() {
    let d = arena();
    write(
        d.path(),
        "package.json",
        r#"{"name":"x","version":"3.4.0"}"#,
    );
    write(
        d.path(),
        "setup.py",
        "from setuptools import setup\nsetup(name='x', version='2.5.1')",
    );
    assert!(
        verdict(d.path()).contains("3.4.0"),
        "la version que ya se habia leido no se pierde por un fichero posterior"
    );
}

// ── CASO 4: lo que DEBÍA seguir fallando cerrado, sigue fallando ──────────

/// La ley es «fail closed, but not fail first». Si «no me declaró» pasa a
/// ser «no me importa», un fichero CORRUPTO pasaría a ser también ignorable, y
/// eso sería exactamente lo contrario de fail-closed.
#[test]
fn un_manifiesto_corrupto_sigue_siendo_fallo_cerrado() {
    let d = arena();
    write(d.path(), "package.json", "{ esto no es json ");
    let v = verdict(d.path());
    assert!(
        v.contains("INVALIDO"),
        "un package.json corrupto tiene que ser un fallo, no un «no me declara»: {v}"
    );
    let r = refusal(d.path(), "v1.0.0");
    assert!(r.contains("ERROR"), "{r}");
    assert!(
        r.contains("could not be read") || r.contains("no se pudo"),
        "el rechazo tiene que decir QUE no se pudo leer: {r}"
    );
}

// ── CASO 5: dos que declaran distinto, es divergencia ──────────────────────

/// Si esto pasara a resolverse por precedencia, la precedencia estaría
/// decidiendo una contradicción. Y con el motor nuevo no hay precedencia que
/// aplicar: no hay orden.
#[test]
fn dos_ficheros_que_declaran_distinto_siguen_siendo_divergentes() {
    let d = arena();
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
    let v = verdict(d.path());
    assert!(
        v.contains("DIVERGENTE"),
        "3.4.0 y 8.1.0 son dos declaraciones que discrepan y no hay precedencia \
         que las ordene: {v}"
    );
    let r = refusal(d.path(), "v3.4.0");
    assert!(r.contains("ERROR"), "{r}");
    assert!(
        r.contains("declarations disagree") && r.contains("3.4.0") && r.contains("8.1.0"),
        "el rechazo tiene que nombrar LAS DOS, porque elegir una es una decision humana: {r}"
    );
}

// ── CASO 6: la ausencia declarada no es el silencio ───────────────────────

/// Un `go.mod` presente DECLARA que su versión no está en un manifiesto. Eso
/// es una respuesta, y publicable; un directorio vacío es silencio, y no.
#[test]
fn una_convencion_declarada_publica_y_el_silencio_no() {
    let go = arena();
    write(go.path(), "go.mod", "module example.com/f\n\ngo 1.22\n");
    assert!(
        verdict(go.path()).contains("SOLO-RELEASE-REF"),
        "un go.mod presente declara una convencion: {}",
        verdict(go.path())
    );
    assert!(
        refusal(go.path(), "v1.0.0").starts_with("OK"),
        "y un proyecto que declaro su convencion puede publicar: {}",
        refusal(go.path(), "v1.0.0")
    );

    let vacio = arena();
    assert_eq!(verdict(vacio.path()), "SIN-RESOLVER");
    let r = refusal(vacio.path(), "v1.0.0");
    assert!(
        r.contains("ERROR"),
        "un repositorio en el que no declara nadie NO puede publicar: {r}"
    );
    assert!(
        !r.contains("SOLO-RELEASE-REF"),
        "y el mensaje no puede disfrazar el silencio de convencion: {r}"
    );
}
