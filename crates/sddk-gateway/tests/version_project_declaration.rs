//! La declaración del proyecto es una fuente más, y no un adorno.
//!
//! # Qué se está midiendo
//!
//! Que `.sddk/version-source.json` pueda **declarar un valor**, y que declararlo
//! sea una de las formas de responder a la misma pregunta que responde un
//! `Cargo.toml` —ni mejor, ni peor, ni con prioridad.
//!
//! MEDIDO, y es lo que motivó el bloque:
//!
//! 1. Un proyecto con un `build.sbt` —Scala, fuera de las trece filas de
//!    `DEFAULT_DECLARATIONS`— que declara su versión aquí **no publicaba**.
//! 2. Peor: en un repo que declara `4.2.0` en `Cargo.toml` **y** aquí, el
//!    `Invalid` de esta rama **tapaba** la `Declared` buena y la puerta
//!    rechazaba. Un fichero que el proyecto escribe para ser más explícito no
//!    puede dejar de ser legible por no caber en lo que el build entendía.
//!
//! **La ley que este fichero existe para falsificar:**
//!
//! > Declarar es **una** forma de declarar. Una que dice otra cosa es una
//! > contradicción, y una contradicción falla cerrada.
//!
//! **Hermético:** la ley vive en `declared_version`, una función pura sobre un
//! `serde_json::Value`. Los mutantes corren sin tocar el disco; el conjunto se
//! mide con ficheros reales en un `TempDir`.

use std::fs;
use std::path::Path;

use sddk_domain::version_authority::{PRODUCT_VERSION_OBSERVATION, ReleaseTarget, VersionProbe};
use sddk_gateway::version_provider::{DECLARED_AUTHORITY_PATH, declared_version};
use tempfile::TempDir;

/// El `provider_id` que lleva el provider real. No importa cuál sea: estos
/// tests miden la RESPUESTA, y el id es procedencia.
const PID: &str = "sddk.gateway/.sddk/version-source.json";

fn doc(cuerpo: &str) -> serde_json::Value {
    serde_json::from_str(cuerpo).expect("json de prueba")
}

/// La razón de un error, sin el provider alrededor.
fn motivo(result: Result<VersionProbe, sddk_domain::version_authority::ProviderError>) -> String {
    result.expect_err("se esperaba un error").to_string()
}

// ── La capacidad ───────────────────────────────────────────────────────────

#[test]
fn e1_el_proyecto_declara_un_valor_y_la_evidencia_dice_de_quien() {
    let probe = declared_version(PID, &doc(r#"{"authority":"version","version":"4.2.0"}"#))
        .expect("una declaracion valida se acepta");

    let VersionProbe::Declared { version, evidence } = &probe else {
        panic!("declarar un valor tiene que producir una Declarada: {probe:?}");
    };
    assert_eq!(version.to_string(), "4.2.0");
    assert_eq!(
        evidence.source_kind, "declaracion-del-proyecto/.sddk/version-source.json",
        "la evidencia tiene que decir QUE lo declaro, porque un valor sin saber \
         de quien es no se puede comprobar"
    );
    assert_eq!(
        evidence.location.as_deref(),
        Some(".sddk/version-source.json"),
        "y DONDE lo declaro"
    );
}

/// `digest: None`, y no por descuido.
///
/// Una declaración **dice** el valor; no lo calcula. Un digest afirmaría que
/// esos bytes determinan la versión sin que nadie lo haya comprobado, que es lo
/// mismo que el provider del build tool rechaza de sí mismo.
#[test]
fn e2_declarar_no_es_calcular_y_por_eso_no_lleva_digest() {
    let probe =
        declared_version(PID, &doc(r#"{"authority":"version","version":"1.0.0"}"#)).expect("ok");
    let VersionProbe::Declared { evidence, .. } = &probe else {
        panic!("{probe:?}");
    };
    assert_eq!(
        evidence.digest, None,
        "una version DECLARADA no sale de calcular nada. El digest del fichero \
         seria una afirmacion que nadie ha comprobado"
    );
}

// ── Los fallos, que son la mitad de la ley ─────────────────────────────────

#[test]
fn e3_authority_version_sin_version_dice_que_campo_falta() {
    let razon = motivo(declared_version(PID, &doc(r#"{"authority":"version"}"#)));
    assert!(
        razon.contains("falta el campo") && razon.contains("version"),
        "el motivo tiene que NOMBRAR el campo que falta: un error que no dice \
         cual es deja al operador buscandolo: {razon}"
    );
    assert!(
        !razon.contains("no se pudo ejecutar"),
        "y tiene que distinguirse de un provider que no arranco: son dos \
         reparaciones distintas. {razon}"
    );
}

/// La ley más importante de este fichero.
///
/// Una versión que no parsea **no** puede degradarse a `Undeclared`, porque
/// `Undeclared` cede el turno al siguiente candidato: el error se volvería
/// silencio y el reducer nunca lo sabría.
#[test]
fn e4_una_version_ilegible_falla_y_no_se_degrada_a_silencio() {
    let razon = motivo(declared_version(
        PID,
        &doc(r#"{"authority":"version","version":"no soy una version"}"#),
    ));
    assert!(
        razon.contains("no es una version"),
        "el motivo tiene que decir que lo declarado no es una version: {razon}"
    );

    // Y la forma del resultado es lo que importa: si esto devolviera
    // `Undeclared`, el proyecto pasaria por "el proyecto no declara nada".
    match declared_version(PID, &doc(r#"{"authority":"version","version":"   "}"#)) {
        Err(_) => {}
        Ok(other) => panic!(
            "una declaracion vacia tiene que fallar, no pasar: {other:?}. Un \
             `Undeclared` aqui seria un error que el reducer no ve"
        ),
    }
}

/// Esta versión iba por `declared_version` y no podia ver nada.
///
/// MEDIDO al escribirla: `declared_version` **no lee `authority`** — esa decisión
/// vive en el `match` de `observe`, que necesita disco. Una rama no mide el
/// dispatcher que la elige, y por eso esta va por el provider real. Es la misma
/// leccion que `d5`/`d6` en VA10, que ejercitaban la ley pura sin ejercitar que
/// el dialecto la usara.
#[test]
fn e5_authority_desconocido_sigue_siendo_error_y_dice_las_que_existen() {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
    fs::write(
        dir.path().join(DECLARED_AUTHORITY_PATH),
        r#"{"schema_version":1,"authority":"bogus"}"#,
    )
    .expect("write declaration");
    fs::write(
        dir.path().join("go.mod"),
        "module example.com/f\n\ngo 1.22\n",
    )
    .expect("write go");

    let autoridad = resolver(dir.path());

    let sddk_domain::version_authority::VersionAuthority::Invalid { failures, .. } = &autoridad
    else {
        panic!(
            "una forma que no existe es un error del proyecto, y falla cerrado: \
                 {autoridad:?}"
        );
    };
    let razon = failures
        .iter()
        .find(|(quien, _)| quien.contains("version-source"))
        .map(|(_, por_que)| por_que.as_str())
        .unwrap_or("no hay fallo del provider de la declaracion");

    assert!(
        razon.contains("\"tag\"") && razon.contains("\"version\""),
        "un nombre que no existe tiene que decir cuales si existen: {razon}"
    );
}

/// La frontera entre las dos variantes, que es lo que las separa.
///
/// MEDIDO: las dos decian «el provider X no se pudo ejecutar», y una de ellas
/// miente, porque el fichero SI se leyo y respondio.
#[test]
fn e12_lo_que_se_leyo_y_no_se_entiende_no_dice_que_no_se_pudo_ejecutar() {
    let razon = motivo(declared_version(
        PID,
        &doc(r#"{"authority":"version","version":"no soy una version"}"#),
    ));
    assert!(
        !razon.contains("no se pudo ejecutar"),
        "el fichero se leyo y respondio: decir que no se pudo ejecutar manda al \
         operador a comprobar el binario cuando el problema esta en su JSON. \
         {razon}"
    );
    assert!(
        razon.contains("no entiende"),
        "y el texto tiene que decir la verdad: contesto algo que este build no \
         entiende. {razon}"
    );

    // Y la otra mitad: `Unavailable` sigue existiendo y sigue diciendo la verdad,
    // porque el binario ausente es un hecho distinto.
    let binario_ausente = sddk_domain::version_authority::ProviderError::Unavailable {
        provider_id: PID.to_owned(),
        reason: "`mvn` no esta instalado, asi que no se pudo ejecutar".to_owned(),
    };
    assert!(
        binario_ausente.to_string().contains("no se pudo ejecutar"),
        "un binario que no se pudo lanzar SI se pudo ejecutar mal, y el mensaje \
         tiene que seguir diciendo eso"
    );
}

/// La no-regresión que el `PRE-FLIGHT` declara como riesgo de este bloque.
#[test]
fn e6_la_declaracion_no_tapa_a_la_declaracion_buena() {
    // MEDIDO, antes del bloque: con `Cargo.toml` en 4.2.0 y esta declaracion, el
    // `Invalid` de la rama tapaba la `Declared` buena y la puerta rechazaba.
    //
    // Aqui lo que se mide es la parte que este provider controla: que la rama
    // `version` produzca una `Declared` y no un error. La combinacion con la
    // tabla se mide en `version_project_declaration_table.rs`.
    let probe = declared_version(PID, &doc(r#"{"authority":"version","version":"4.2.0"}"#)).expect(
        "una declaracion valida NO puede tapar a nadie: tiene que ser \
                 una Declarada, no un error",
    );
    assert!(
        matches!(probe, VersionProbe::Declared { .. }),
        "y es una Declarada: {probe:?}"
    );
}

// ── El conjunto, con ficheros de verdad ────────────────────────────────────

/// Un proyecto cuya tecnologia SDDK no conoce, que publica por declaracion.
///
/// El caso que motivo el bloque: `build.sbt` no esta en las trece filas, y antes
/// de esto no habia ninguna forma de que un proyecto asi publicara.
#[test]
fn e7_un_proyecto_que_sdkk_no_conoce_publica_por_declaracion() {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
    fs::write(
        dir.path().join(DECLARED_AUTHORITY_PATH),
        r#"{"schema_version":1,"authority":"version","version":"7.8.9"}"#,
    )
    .expect("write declaration");
    fs::write(dir.path().join("build.sbt"), "name := \"acme\"\n").expect("write sbt");

    let autoridad = resolver(dir.path());

    let Some(version) = autoridad.version() else {
        panic!(
            "un proyecto que declara su version tiene que resolverla, y este \
             no aparece en ninguna de las trece filas:\n{autoridad:?}"
        );
    };
    assert_eq!(
        version.to_string(),
        "7.8.9",
        "y el valor es el que el proyecto declaro, no uno que SDDK dedujo"
    );
}

/// Y lo que la fila de la tabla hace cuando el proyecto **también** se declara.
#[test]
fn e8_la_declaracion_no_gana_a_la_fila_de_la_tabla() {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
    fs::write(
        dir.path().join(DECLARED_AUTHORITY_PATH),
        r#"{"schema_version":1,"authority":"version","version":"9.9.9"}"#,
    )
    .expect("write declaration");
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"4.2.0\"\n",
    )
    .expect("write cargo");

    let autoridad = resolver(dir.path());

    assert!(
        !autoridad.was_cross_validated(),
        "una discrepancia NO es un cruce: dos fuentes que dicen cosas distintas \
         no se confirman entre si.\n{autoridad:?}"
    );
    assert!(
        matches!(
            autoridad,
            sddk_domain::version_authority::VersionAuthority::Ambiguous { .. }
        ),
        "y el veredicto tiene que ser una ambiguedad —que es lo que el reducer \
         ya sabia hacer— no una eleccion:\n{autoridad:?}"
    );
}

/// Coincidir, en cambio, **sí** es un cruce: dos fuentes independientes que
/// dicen lo mismo es más evidencia, no menos.
#[test]
fn e9_coincidir_con_la_tabla_es_un_cruce_y_no_una_redundancia() {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
    fs::write(
        dir.path().join(DECLARED_AUTHORITY_PATH),
        r#"{"schema_version":1,"authority":"version","version":"4.2.0"}"#,
    )
    .expect("write declaration");
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"4.2.0\"\n",
    )
    .expect("write cargo");

    let autoridad = resolver(dir.path());

    assert!(
        autoridad.was_cross_validated(),
        "el JSON del proyecto y su `Cargo.toml` son dos fuentes independientes: \
         que coincidan es un cruce, y degradarlo a una sola declaracion seria \
         descartar la mitad de la evidencia.\n{autoridad:?}"
    );
}

/// La no-regresión explícita: `authority: "tag"` no se ha tocado.
#[test]
fn e10_authority_tag_no_se_rompio() {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
    fs::write(
        dir.path().join(DECLARED_AUTHORITY_PATH),
        r#"{"schema_version":1,"authority":"tag"}"#,
    )
    .expect("write declaration");
    fs::write(
        dir.path().join("go.mod"),
        "module example.com/f\n\ngo 1.22\n",
    )
    .expect("write go");

    let autoridad = resolver(dir.path());

    assert!(
        matches!(
            autoridad,
            sddk_domain::version_authority::VersionAuthority::ReleaseRefIsAuthority { .. }
        ),
        "`authority: \"tag\"` es una convencion declarada y sigue siendolo: \
         este bloque anadio una forma, no cambio la anterior.\n{autoridad:?}"
    );
}

/// Y que el fichero siga siendo de solo lectura.
#[test]
fn e11_leer_la_declaracion_no_escribe_en_el_arbol() {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join(".sddk")).expect("mkdir");
    fs::write(
        dir.path().join(DECLARED_AUTHORITY_PATH),
        r#"{"schema_version":1,"authority":"version","version":"3.2.1"}"#,
    )
    .expect("write declaration");

    let antes = listar(dir.path());
    resolver(dir.path());
    let despues = listar(dir.path());

    assert_eq!(
        antes, despues,
        "un proyecto que declara su version no tiene que suffer cambios en el \
         arbol para que SDDK la sepa"
    );
}

fn listar(raiz: &Path) -> Vec<String> {
    let mut fuera = Vec::new();
    let mut pendientes = vec![raiz.to_path_buf()];
    while let Some(dir) = pendientes.pop() {
        let Ok(entradas) = fs::read_dir(&dir) else {
            continue;
        };
        for entrada in entradas.flatten() {
            let ruta = entrada.path();
            if ruta.is_dir() {
                pendientes.push(ruta);
            } else {
                fuera.push(format!("{}", ruta.display()));
            }
        }
    }
    fuera.sort();
    fuera
}

/// La version que el registry por defecto resuelve para un directorio.
fn resolver(raiz: &Path) -> sddk_domain::version_authority::VersionAuthority {
    let target = ReleaseTarget::at("raiz", raiz.display().to_string());
    sddk_gateway::version_provider::default_version_registry()
        .resolve(PRODUCT_VERSION_OBSERVATION, &target)
}
