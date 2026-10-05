//! Un comando que responde sobre la version **puede preguntar**, y si no pregunto
//! lo dice.
//!
//! # Qué se está midiendo
//!
//! MEDIDO antes del bloque, sobre un `build.gradle` con `version = '1.2.3'`:
//!
//! ```text
//! $ sddk release version matches --tag v1.2.3    # el tag CORRECTO
//! `productVersion: none`
//! matches: false
//! detail: this target declares no product version, so there is nothing to compare against
//!
//! $ sddk release version matches --tag v9.9.9    # el tag que el proyecto contradece
//! `productVersion: none`
//! matches: false
//! detail: this target declares no product version, so there is nothing to compare against
//! ```
//!
//! **Byte a byte, la misma salida.** Eso no es un mensaje impreciso: es un
//! comando cuya respuesta no depende de lo preguntado. Y `detail` afirma un hecho
//! falso —el target sí declara versión, en un fichero que SDDK localizó y decidió
//! no leer, porque la única forma honesta de leerlo es preguntarle a Gradle—.
//!
//! Y el caso que no necesita ninguna herramienta, que es el peor de los cuatro:
//! dos ficheros que se contradicen. `Cargo.toml` en 4.2.0 y la declaración del
//! proyecto en 9.9.9 dan `productVersion: none` y **`this target declares no
//! product version`**, con código de salida 0, sobre un proyecto que no se ha
//! olvidado nada: declara dos cosas y se contradicen.
//!
//! # Hermético
//!
//! Todo ocurre en `TempDir`s y sin red. **No se arranca `gradle`**: una ley que
//! necesita la herramienta instalada para falsificarse es una ley que nadie
//! falsifica, y `crates/sddk-cli/src/dev/update.rs:26` ya dejo escrito que en
//! edition 2024 ningun test puede tocar `PATH` para inyectarla. La rama del build
//! tool se mide por encima, sobre fixtures reales, y ese Wire queda en el
//! `CLOSURE` con los comandos exactos; lo de aqui son las leyes que no la
//! necesitan.

use std::fs;
use std::path::Path;

use sddk_cli::{CommandOutput, run_from};
use tempfile::TempDir;

const DECLARACION: &str = ".sddk/version-source.json";

/// Un repo cuyo unico contenido es un `Cargo.toml` con version, y opcionalmente
/// una declaracion del proyecto al lado.
fn repo(contenido: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    for (ruta, cuerpo) in contenido {
        let completo = dir.path().join(ruta);
        if let Some(padre) = completo.parent() {
            fs::create_dir_all(padre).expect("mkdir");
        }
        fs::write(completo, cuerpo).expect("write");
    }
    dir
}

fn cargo(version: &str) -> TempDir {
    repo(&[(
        "Cargo.toml",
        &format!("[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"{version}\"\n"),
    )])
}

/// Corre un comando de `sddk` contra `dir`.
///
/// `ruta` es la ruta del subcomando **completa y antes de los flags**
/// (`["version", "matches"]`, `["handoff"]`), y `extra` lo que se le anade al
/// final. MEDIDO, al escribir esto: la primera version ponia `--root` antes de
/// la ruta, y los ocho tests que la usaban caian con `unexpected argument
/// '--root' found` —un error de la herramenta que `dice`, que exige 0,
/// reportaba como un fallo de la ley—. Un helper que construye mal la linea de
/// comandos no mide la ley, la esconde detras de un error suyo.
fn salida(dir: &Path, ruta: &[&str], extra: &[&str]) -> CommandOutput {
    let mut todos: Vec<String> = vec!["sddk".into(), "release".into()];
    todos.extend(ruta.iter().map(|s| (*s).to_owned()));
    todos.push("--root".into());
    todos.push(dir.to_str().expect("utf-8").into());
    todos.push("--scope".into());
    todos.push(".".into());
    todos.extend(extra.iter().map(|s| (*s).to_owned()));
    let refs: Vec<&str> = todos.iter().map(String::as_str).collect();
    run_from(refs)
}

fn matches(dir: &Path, tag: &str) -> CommandOutput {
    salida(dir, &["version", "matches"], &["--tag", tag])
}

fn handoff(dir: &Path, tag: &str) -> CommandOutput {
    salida(
        dir,
        &["handoff"],
        &[
            "--tag",
            tag,
            "--external-type",
            "digest-list",
            "--external-digest",
            "deadbeef",
        ],
    )
}

/// `matches` es un **informe**: sale con 0 y dice lo que ha visto. MEDIDO: medir
/// su codigo de salida no mide nada, porque es 0 con el veredicto rojo.
fn dice(out: &CommandOutput) -> &str {
    assert_eq!(
        out.status, 0,
        "matches es un informe y sale con 0 para poder leerlo: {}",
        out.stderr
    );
    &out.stdout
}

// ── La invariante del bloque: la respuesta depende del tag ─────────────────

/// **Este es el falsador del bloque entero.**
///
/// MEDIDO antes: con `v1.2.3` y con `v9.9.9` la salida era identica byte a
/// byte. Si esto pasa, el comando no esta midiendo la pregunta.
#[test]
fn la_respuesta_depende_del_tag() {
    let dir = cargo("4.2.0");

    let correcta = matches(dir.path(), "v4.2.0");
    let erronea = matches(dir.path(), "v9.9.9");

    let correcta = dice(&correcta);
    let erronea = dice(&erronea);

    assert!(
        correcta.contains("matches: true"),
        "el tag que nombra la version declarada tiene que casar:\n{correcta}"
    );
    assert!(
        erronea.contains("matches: false"),
        "el tag que la contradice tiene que no casar:\n{erronea}"
    );
    assert_ne!(
        correcta, erronea,
        "MEDIDO antes del bloque: estas dos salidas eran byte a byte iguales. \
         Un comando cuya respuesta no depende de lo preguntado no mide la pregunta."
    );
    assert!(
        correcta.contains("measured: true") && erronea.contains("measured: true"),
        "con la version declarada esto SI es una medicion, en los dos tags:\n{correcta}\n---\n{erronea}"
    );
}

/// Un `false` que significa «no lo se» y un `false` que significa «no coincide»
/// tienen que ser distinguibles sin leer la prosa. MEDIDO antes: eran el mismo
/// `false`.
#[test]
fn un_falso_que_no_se_y_un_falso_que_no_coincide_se_distinguen() {
    let rust = cargo("4.2.0");
    // El mismo comando, sobre el mismo repo, con un tag que no coincide:
    // `false` medido. Sobre un repo donde nadie pregunto al build tool y nada
    // declara: `false` sin medir.
    let medido = matches(rust.path(), "v9.9.9");

    let gradle = repo(&[("build.gradle", "version = '1.2.3'\n")]);
    let sin_medir = matches(gradle.path(), "v1.2.3");

    assert!(
        dice(&medido).contains("measured: true"),
        "una version observada y un tag que no casa es una medicion:\n{}",
        dice(&medido)
    );
    assert!(
        dice(&sin_medir).contains("measured: false"),
        "nadie pregunto al build tool y nada declaro: eso no es una medicion:\n{}",
        dice(&sin_medir)
    );
}

// ── Los cinco hechos, y por que no son uno ─────────────────────────────────

/// El caso sin herramientas: dos ficheros que se contradicen.
///
/// El texto viejo decia `this target declares no product version`, que es
/// **exactamente lo contrario** de lo que pasa.
#[test]
fn una_contradiccion_no_es_silencio() {
    let dir = repo(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"4.2.0\"\n",
        ),
        (
            DECLARACION,
            r#"{"schema_version":1,"authority":"version","version":"9.9.9"}"#,
        ),
    ]);

    let salida_cruda = matches(dir.path(), "v4.2.0");
    let texto = dice(&salida_cruda);

    assert!(
        texto.contains("measured: true"),
        "dos fuentes que se contradicen son lo MAS mirado que hay, no lo menos:\n{texto}"
    );
    assert!(
        !texto.contains("declares no product version"),
        "MEDIDO antes: este texto decia `this target declares no product version` \
         sobre un repo que declara DOS versiones. Es lo contrario de lo que pasa:\n{texto}"
    );
    // Y nombra las DOS, que es lo que el lector viene a ver. La misma medida que
    // tomo VA10 en el rechazo de una discrepancia: no catorce entradas, las dos
    // que discrepan.
    assert!(
        texto.contains("4.2.0") && texto.contains("9.9.9"),
        "el mensaje tiene que NOMBRAR las dos declaraciones que discrepan:\n{texto}"
    );
    assert!(
        texto.contains("disagree"),
        "y tiene que decir que discrepan:\n{texto}"
    );
}

/// `handoff` dice lo mismo por la misma funcion, porque un productor que lee
/// «no hay nada que entregar» cuando lo que hay es una contradiccion va a
/// arreglarlo en el sitio equivocado.
#[test]
fn handoff_no_dice_que_no_declara_cuando_se_contradicen() {
    let dir = repo(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"4.2.0\"\n",
        ),
        (
            DECLARACION,
            r#"{"schema_version":1,"authority":"version","version":"9.9.9"}"#,
        ),
    ]);

    let out = handoff(dir.path(), "v4.2.0");

    assert_ne!(out.status, 0, "handoff sin version no construye un sobre");
    let stderr = &out.stderr;
    assert!(
        !stderr.contains("declares no product version"),
        "MEDIDO antes: el rechazo decia `this target declares no product version` \
         sobre un repo que declara dos versiones que discrepan:\n{stderr}"
    );
    assert!(
        stderr.contains("4.2.0") && stderr.contains("9.9.9"),
        "y tiene que nombrar las dos, como hace `matches`:\n{stderr}"
    );
}

/// MEDIDO, y este defecto lo introdujo el arreglo: `handoff` anadia su propia
/// frase «Run `sddk release version inspect`...» encima de un texto que ya
/// remite ahi. Un mensaje que dice dos veces lo mismo no informa mas: entrena al
/// lector a leer el primero y saltar el segundo. Ese defecto es de VA10 y lo
/// corrigio entonces; repetirlo aqui seria perderlo en la traduccion.
#[test]
fn el_rechazo_no_repite_la_pista_que_ya_lleva_el_texto() {
    let dir = cargo("4.2.0");
    let out = handoff(dir.path(), "v4.2.0");
    let stderr = &out.stderr;

    let veces = stderr.matches("release version inspect").count();
    assert!(
        veces <= 1,
        "la pista a `inspect` sale {veces} veces y solo hace falta una: un mensaje \
         que repite lo mismo no informa mas.\n{stderr}"
    );
}

/// Una fuente que existe y no se pudo leer es ruido, no silencio: es lo contrario.
#[test]
fn una_fuente_ilegible_no_es_silencio() {
    let dir = repo(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"4.2.0\"\n",
        ),
        (
            DECLARACION,
            r#"{"schema_version":99,"authority":"version","version":"4.2.0"}"#,
        ),
    ]);

    let salida_cruda = matches(dir.path(), "v4.2.0");
    let texto = dice(&salida_cruda);

    assert!(
        texto.contains("measured: true"),
        "una fuente que existe y no se pudo leer se ha mirado, y no se ha \
         entendido: eso es un hecho, no una ausencia:\n{texto}"
    );
    assert!(
        texto.contains("could not be read"),
        "y el texto tiene que decir que no se pudo LEER, que no es lo mismo que \
         no declarar:\n{texto}"
    );
}

/// La convencion declarada de Go y Bazel no se rompio: sigue siendo una
/// declaracion —y por tanto una medicion—, no un silencio.
#[test]
fn la_release_ref_que_lleva_la_version_sigue_siendo_una_medicion() {
    let dir = repo(&[
        ("go.mod", "module example.com/f\n\ngo 1.22\n"),
        (DECLARACION, r#"{"schema_version":1,"authority":"tag"}"#),
    ]);

    let salida_cruda = matches(dir.path(), "v1.2.3");
    let texto = dice(&salida_cruda);

    assert!(
        texto.contains("measured: true"),
        "un target que DECLARA que su version la lleva la referencia ha hecho una \
         declaracion, y una declaracion se ha leido:\n{texto}"
    );
    assert!(
        texto.contains("matches: true"),
        "y con esa convencion el tag liga por declaracion:\n{texto}"
    );
    assert!(
        texto.contains("release reference carries it"),
        "el texto de este caso no cambio: era el unico cierto de los cuatro:\n{texto}"
    );
}

/// Sin preguntar, el texto tiene que decirlo **y nombrar la bandera** que lo
/// arregla. Un rechazo sin salida es un rechazo que gasta la confianza del
/// lector en un camino que no lleva a ninguna parte.
#[test]
fn sin_preguntar_el_texto_lo_dice_y_nombra_la_bandera() {
    let dir = repo(&[("build.gradle", "version = '1.2.3'\n")]);

    let salida_cruda = matches(dir.path(), "v1.2.3");
    let texto = dice(&salida_cruda);

    assert!(
        texto.contains("did not ask the build tool"),
        "el texto tiene que decir que no se pregunto:\n{texto}"
    );
    assert!(
        texto.contains("--evaluate-build") && texto.contains("--build-tool"),
        "y nombrar las dos banderas, que es la reparacion:\n{texto}"
    );
    assert!(
        texto.contains("measured: false"),
        "y el campo tiene que decir que no es una medicion, porque sin el campo \
         un `matches: false` por no preguntar es indistinguible de un `false` \
         por no coincidir:\n{texto}"
    );
    assert!(
        !texto.contains("declares no product version"),
        "que es exactamente lo contrario de lo que se puede afirmar de un target \
         que declara su version en `build.gradle`:\n{texto}"
    );
}

// ── La superficie: preguntar sigue siendo una decision del operador ─────────

/// Sin `--evaluate-build` no se pregunta, y `--build-tool` solo sin la primera no
/// abre la puerta por la backdoor: es la misma ley que `release plan`.
#[test]
fn preguntar_sigue_siendo_opt_in() {
    let dir = cargo("4.2.0");

    // Con la bandera sola, sin nombrar herramienta: falla cerrado y lo dice.
    let sin_nombre = salida(
        dir.path(),
        &["version", "matches"],
        &["--tag", "v4.2.0", "--evaluate-build"],
    );
    assert_ne!(
        sin_nombre.status, 0,
        "sin `--build-tool` no se puede preguntar, y un default aqui responderia \
         en Gradle cuando se pidio otra cosa"
    );
    assert!(
        sin_nombre.stderr.contains("--build-tool"),
        "el rechazo tiene que nombrar la bandera que falta:\n{}",
        sin_nombre.stderr
    );

    // Una herramienta que SDDK no sabe preguntar: error de la linea de comandos
    // que dice cuales si.
    let desconocida = salida(
        dir.path(),
        &["version", "matches"],
        &["--tag", "v4.2.0", "--evaluate-build", "--build-tool", "sbt"],
    );
    assert_ne!(
        desconocida.status, 0,
        "una herramienta desconocida es un error"
    );
    assert!(
        desconocida.stderr.contains("gradle") && desconocida.stderr.contains("maven"),
        "y el error dice cuales si se saben preguntar:\n{}",
        desconocida.stderr
    );
}

/// Las cuatro superficies declaran las mismas dos banderas.
///
/// No es una prueba de comportamiento: es una de **superficie**, y existe
/// porque el defecto fue que dos comandos las tenian y dos no. Un gate que solo
/// midiera el comportamiento pasaria si alguien las borrara de `handoff` y
/// ningun test lo notara, porque `handoff` sin preguntar da la misma respuesta
/// que antes —que era el defecto—.
#[test]
fn las_cuatro_superficies_declaran_las_mismas_dos_banderas() {
    for (nombre, args) in [
        ("release plan", vec!["release", "plan", "--help"]),
        (
            "release version inspect",
            vec!["release", "version", "inspect", "--help"],
        ),
        (
            "release version matches",
            vec!["release", "version", "matches", "--help"],
        ),
        ("release handoff", vec!["release", "handoff", "--help"]),
    ] {
        // `--help` no necesita root ni scope, y por eso se llama directo a
        // `run_from`: el helper de arriba anade los dos y `plan --help` los
        // acepta pero `handoff --help` no los necesita. Es la linea de comandos
        // tal cual la ve el operador.
        let mut todos: Vec<String> = vec!["sddk".into()];
        todos.extend(args.iter().map(|s| (*s).to_owned()));
        let refs: Vec<&str> = todos.iter().map(String::as_str).collect();
        let out = run_from(refs);
        let ayuda = out.stdout + &out.stderr;
        assert!(
            ayuda.contains("--evaluate-build"),
            "{nombre} tiene que declarar `--evaluate-build`: un comando que \
             responde sobre la version y no puede preguntar no la mide.\n{ayuda}"
        );
        assert!(
            ayuda.contains("--build-tool"),
            "{nombre} tiene que declarar `--build-tool`.\n{ayuda}"
        );
    }
}
// ── El punto ciego, declarado y con su control ──────────────────────────────

/// MEDIDO al escribir este bloque: `release handoff` resolvia la version con
/// `version_registry()` y **no** con `ask.registry()`.
///
/// Eso es un defecto independiente del de las banderas —arreglar solo las
/// banderas habría dado un `handoff` que acepta `--evaluate-build` y lo ignora,
/// que es peor que no aceptarlas— y **ningun test hermetico lo puede cazar**:
/// `crates/sddk-cli/src/dev/update.rs:26` ya dejo escrito que en edition 2024
/// ningun test puede tocar `PATH`, luego no hay forma de poner una herramienta
/// falsa en el sitio donde el registro la busca. Los dos registros solo se
/// distinguen cuando `evaluate` es verdad, y eso exige arrancar un proceso.
///
/// Asi que se mide sobre el fuente, y **con control**, porque un fitness que no
/// puede ver el defecto no es un fitness: el control planta el defecto en una
/// copia y exige que el escaner lo encuentre, luego lo que se verifica es que el
/// escaner mira, no que el codigo este bien.
///
/// ## Por que un fitness y no una asercion de comportamiento
///
/// MEDIDO antes de este bloque, en el mismo repo: el fitness de VA9 que buscaba
/// `-> VersionNaming` en la linea entera marco `-> Option<VersionNaming>`, que
/// es una funcion capaz de decir «no tengo ninguna» —justo lo **opuesto** de
/// tener un default—. Un fitness que se dispara sobre algo legitimo entrena a su
/// lector a saltarselo, que es como un guard desactivado se parece a uno que
/// pasa. Por eso este mira el **nombre del receptor** de la llamada, no una
/// subcadena cualquiera: `ask.registry()` y `version_registry()` se distinguen en
/// el receptor, y `version_registry_asking` —que si es legitimo— no se llama
/// asi.
#[test]
fn handoff_resuelve_por_el_registro_de_la_pregunta() {
    let fuente = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/release_cmd.rs"),
    )
    .expect("leer release_cmd.rs");

    let (_, cuerpo) = fuente
        .split_once("fn run_release_handoff(")
        .expect("existe run_release_handoff");
    let fin = cuerpo.find("\nfn ").map(|i| i + 1).unwrap_or(cuerpo.len());
    let cuerpo = &cuerpo[..fin];

    let usa_el_registro_de_la_pregunta = cuerpo.contains("ask.registry()");
    assert!(
        usa_el_registro_de_la_pregunta,
        "MEDIDO antes de este bloque: el cuerpo de `run_release_handoff` resolvia \
         con `version_registry()` mientras la pregunta vivia en `ask`. Con las \
         banderas puestas y el registro equivocado, `handoff` acepta \
         `--evaluate-build` y lo ignora.\n{cuerpo}"
    );

    // Y el control: el mismo escaner, sobre un cuerpo donde el defecto SI esta,
    // tiene que verlo. Sin esto, el test de arriba pasa igual de verde con un
    // escaner que no lee nada.
    let planta = cuerpo.replace("ask.registry()", "version_registry()");
    assert_ne!(
        planta, cuerpo,
        "el control necesita que el cuerpo tenga la llamada que se va a \
         sustituir; si no la tiene, este test no mide nada"
    );
    assert!(
        !planta.contains("ask.registry()"),
        "el escaner tiene que dejar de ver `ask.registry()` cuando se sustituye \
         por `version_registry()`. Si sigue viéndolo, no esta mirando el \
         receptor y el test de arriba es verde por casualidad."
    );
}
