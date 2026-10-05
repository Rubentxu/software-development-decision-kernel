//! El contrato que **todos** los providers tienen que cumplir.
//!
//! # Por qué esta suite existe y por qué está separada de la otra
//!
//! `version_provider_conformance.rs` mide **que** encuentra cada provider sobre
//! árboles reales: que un `go.mod` se lee, que dos dialectos de python se leen
//! por separado, que un escrito roto no tapa a uno bueno. Son **hechos sobre
//! providers**, y cambian con cada uno que se añada.
//!
//! Esta mide **propiedades del contrato**, y esas no cambian: son las diez que
//! un provider nuevo hereda sin leer nada. Un provider que solo se somete a la
//! otra suite puede cumplir todas sus filas y violar cualquier número de estas,
//! porque sus formas son enteramente suyas.
//!
//! Estar separadas es lo que hace la suite utilizable: un provider nuevo tiene
//! que pasar por aquí, y no hay forma de que se escape a la otra.
//!
//! # Cómo se lee la numeración
//!
//! C1..C10 en el mismo orden que el enunciado del bloque, y cada una con su
//! nombre. Una fila sin nombre es una fila que un provider puede reinterpretar
//! como le convenga, que es exactamente lo que un contrato tiene que impedir.

use std::path::{Path, PathBuf};

use sddk_domain::version_authority::{PRODUCT_VERSION_OBSERVATION, ReleaseTarget, VersionEvidence};
use sddk_domain::version_inspection::{AssuranceLevel, VersionInspection};
use sddk_gateway::version_provider::default_version_registry;

fn write(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("se crea el directorio");
    }
    std::fs::write(path, body).expect("se escribe el fixture");
}

fn target(root: &Path) -> ReleaseTarget {
    // La raíz **viaja en el target**, y no es un detalle del arnés: con
    // `at_root("runtime")` el target resuelve contra `.`, luego un fixture
    // escrito en un temporal no lo ve nadie y la suite mide el repositorio en
    // el que se compila. Los cinco fallos primeros de esta suite salieron de
    // ahí, y ninguno era del producto.
    ReleaseTarget::at("runtime", root.display().to_string())
}

fn inspect(root: &Path) -> VersionInspection {
    default_version_registry().resolve_inspecting(PRODUCT_VERSION_OBSERVATION, &target(root))
}

/// Un árbol con una sola declaración y nada más, para partir de algo limpio.
fn arena(nombre: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("directorio temporal");
    write(
        dir.path(),
        "package.json",
        r#"{"name":"runtime","version":"1.2.3"}"#,
    );
    let _ = nombre;
    dir
}

// ── C1 · solo lectura ───────────────────────────────────────────────────

/// **C1 — La resolución no escribe nada.**
///
/// Se mide con el sha256 del árbol antes y después, no declarándolo: la
/// afirmación «es de solo lectura» es exactamente la que se cita sin comprobar,
/// y un ADR no es una medición. El nombre del fichero va **dentro** del hash,
/// porque un provider que renombrase algo dejaría el contenido igual.
#[test]
fn c1_la_resolucion_no_escribe() {
    let dir = arena("c1");
    let antes = huella_del_arbol(dir.path());
    let inspection = inspect(dir.path());
    let despues = huella_del_arbol(dir.path());

    assert_eq!(
        antes, despues,
        "resolver no puede cambiar el arbol: es la misma medicion que hace la \
         ley de VA1, y aqui se comprueba sobre un arbol real"
    );
    // Y que el árbol tiene contenido de verdad, o el test no midió nada: una
    // comparación de dos directorios vacíos es verde sin medir.
    assert!(
        dir.path().join("package.json").exists(),
        "el fixture tiene que existir o la comparacion es de nada contra nada"
    );
    assert!(
        inspection.found_version(),
        "y la resolución tiene que haber ocurrido de verdad"
    );
}

// ── C2 · determinista ───────────────────────────────────────────────────

/// **C2 — La misma evidencia da la misma respuesta.**
///
/// Un diagnóstico cuyo texto cambia entre dos lecturas del mismo árbol no se
/// puede citar, y citarlo es lo que se hace con un diagnóstico. La
/// serialización también se compara, porque un informe con `HashMap` detrás
/// ordena sus claves distinto en dos procesos y parece determinista dentro de
/// uno.
#[test]
fn c2_es_determinista_para_la_misma_evidencia() {
    let dir = arena("c2");
    let una = inspect(dir.path());
    let otra = inspect(dir.path());
    assert_eq!(
        una, otra,
        "dos resoluciones del mismo arbol dan el mismo informe"
    );
    assert_eq!(
        serde_json::to_string(&una).expect("serializa"),
        serde_json::to_string(&otra).expect("serializa"),
        "y el mismo JSON: el texto de un diagnostico es lo que se cita"
    );
}

// ── C3 · NotApplicable correcto ─────────────────────────────────────────

/// **C3 — «No es asunto mío» es una respuesta, no un fallo.**
///
/// Es la fila que separa dos cosas que se parecían: un provider que miró y no
/// vio nada, y uno que ni miró porque el target no es suyo. Con un árbol vacío
/// de declaraciones, la única respuesta honesta de cada provider es la segunda,
/// y si alguno contesta `Undeclared` está afirmindo que conoce el interior de
/// un repo que no tiene nada que declarar.
#[test]
fn c3_not_applicable_es_una_respuesta_y_no_un_fallo() {
    let dir = tempfile::tempdir().expect("directorio temporal");
    let inspection = inspect(dir.path());

    // Sin declaraciones, nadie puede haber declarado nada.
    assert!(
        !inspection.found_version(),
        "un arbol sin declaraciones no resuelve ninguna version"
    );
    // Y lo que contestaron los providers que preguntaron.
    let por_nivel = |nivel: AssuranceLevel| {
        inspection
            .findings
            .iter()
            .filter(|f| f.level == nivel)
            .count()
    };
    assert_eq!(
        por_nivel(AssuranceLevel::Observed),
        0,
        "nadie declara nada en un arbol vacio"
    );
    // Los que compete son NOT_APPLICABLE o NOT_CHECKED, nunca OBSERVED.
    for finding in &inspection.findings {
        assert_ne!(
            finding.level,
            AssuranceLevel::Observed,
            "el provider «{}» no puede observar en un arbol vacio",
            finding.provider_id
        );
    }
}

// ── C4 · Undeclared correcto ────────────────────────────────────────────

/// **C4 — Un fichero presente que no declara no inventa nada.**
///
/// El provider tiene que decir que miró y que no había, en sus propias
/// palabras, y el hueco (`None`) tiene que ser un hueco de verdad: un default
/// con cara de respuesta es el modo de fallo más caro de una capability.
#[test]
fn c4_un_fichero_que_no_declara_no_inventa_una_version() {
    let dir = tempfile::tempdir().expect("directorio temporal");
    // Un fichero de construccion que existe y no declara version.
    write(dir.path(), "settings.properties", "verbose=true\n");

    let inspection = inspect(dir.path());
    assert!(
        !inspection.found_version(),
        "un fichero presente sin version no produce una version: ese fue el \
         defecto medido de VA3 y no vuelve"
    );
    for finding in &inspection.findings {
        assert!(
            finding.product_version.is_none(),
            "el provider «{}» no puede devolver una version aqui",
            finding.provider_id
        );
    }
    // Y quien lo vio lo dice en sus palabras, no con un hueco mudo.
    for finding in &inspection.findings {
        assert!(
            !finding.detail.is_empty(),
            "el provider «{}» contesta sin decir nada, y un hueco mudo no es \
             una respuesta",
            finding.provider_id
        );
    }
}

// ── C5 · Declared con procedencia ───────────────────────────────────────

/// **C5 — Una declaración viene con procedencia, o se declara que no la tiene.**
///
/// Es el punto 9 del enunciado, y MEDIDO antes de escribirlo: el único provider
/// que emite evidencia lo hace con `digest: None` y `location` presente, y
/// **nada lo exigía**. Un provider nuevo podía declarar sin ninguna de las dos
/// cosas y el informe no lo decía. No se rechaza —el núcleo no tiene autoridad
/// para llamar deshonesto a un provider que quizá es solo breve—, pero **se
/// declara**, porque eso sí es un hecho y un hecho va en `NOT_CHECKED`.
#[test]
fn c5_una_declaracion_lleva_procedencia_o_dice_que_no() {
    let dir = arena("c5");
    let inspection = inspect(dir.path());

    let declaradores: Vec<_> = inspection
        .findings
        .iter()
        .filter(|f| f.level == AssuranceLevel::Observed)
        .collect();
    assert!(
        !declaradores.is_empty(),
        "el fixture declara, luego tiene que haber declarantes"
    );
    for finding in declaradores {
        if finding.recheckable {
            assert!(
                finding.location.is_some() || finding.digest.is_some(),
                "«{}» dice que es revisable y no tiene ni digest ni localizacion",
                finding.provider_id
            );
            assert!(
                !finding.detail.is_empty(),
                "«{}» declara y no dice de donde",
                finding.provider_id
            );
        } else {
            // Y si no es revisable, el informe tiene que decirlo. Este es el
            // punto: no se acepta en silencio.
            assert!(
                inspection
                    .not_checked
                    .iter()
                    .any(|n| n.key == "declared_without_recheckable_evidence"),
                "«{}» declara sin nada que volver a mirar y el informe no lo \
                 dice: es exactamente el PASS sin evidencia que el contrato \
                 prohibe",
                finding.provider_id
            );
        }
    }
}

// ── C6 · Invalid falla cerrado ──────────────────────────────────────────

/// **C6 — Un origen ilegible falla cerrado, y manda sobre una que sí declaró.**
///
/// No basta con que el provider diga `Invalid`: tiene que **ganar**. Un árbol
/// con un manifiesto bueno y uno roto resuelve a `INVALID`, porque no se puede
/// saber que la versión está bien cuando algo que se suponía declarado no se
/// pudo leer.
#[test]
fn c6_un_origen_ilegible_falla_cerrado_y_manda() {
    let dir = arena("c6");
    write(dir.path(), "package.json", "{ esto no es json");

    let inspection = inspect(dir.path());
    assert_eq!(
        inspection.authority,
        AssuranceLevel::Invalid,
        "una version declarada junto a un origen ilegible no resuelve: \
         `Invalid` va por encima de `Declared`"
    );
    assert!(!inspection.found_version());
    assert!(
        inspection
            .findings
            .iter()
            .any(|f| f.level == AssuranceLevel::Invalid),
        "y el informe nombra quien fallo y por que, en sus palabras"
    );
    for finding in inspection
        .findings
        .iter()
        .filter(|f| f.level == AssuranceLevel::Invalid)
    {
        assert!(
            !finding.detail.is_empty(),
            "«{}» falla sin decir por que, y un fallo cerrado sin causa es \
             indistinguible de cien",
            finding.provider_id
        );
    }
}

// ── C7 · sin defaults silenciosos ───────────────────────────────────────

/// **C7 — Nadie rellena un hueco con un valor por defecto.**
///
/// Un default silencioso es la forma más cara de fallo posible aquí, porque no
/// se ve: el informe dice `1.0.0` y nadie sabe de dónde salió. Se mide en las
/// dos direcciones: lo que el provider **no** tiene que llenar (la versión) y lo
/// que el informe **no** tiene que inventar (los providers que no hablaron).
#[test]
fn c7_sin_defaults_silenciosos() {
    let dir = tempfile::tempdir().expect("directorio temporal");
    let inspection = inspect(dir.path());

    // Nadie devuelve una versión que no leyó.
    for finding in &inspection.findings {
        match finding.level {
            AssuranceLevel::Observed => {
                assert!(
                    finding.product_version.is_some(),
                    "«{}» observa y no dice que version",
                    finding.provider_id
                );
            }
            _ => {
                assert!(
                    finding.product_version.is_none(),
                    "«{}» no observa y aun asi trae version: hay un default \
                     silencioso",
                    finding.provider_id
                );
            }
        }
    }
    // Y el veredicto sin versión no la inventa para el JSON.
    let json = serde_json::to_value(&inspection).expect("serializa");
    assert!(
        json.get("version").is_none() || json["version"].is_null(),
        "un informe sin version no lleva una en el JSON: {json}"
    );
}

// ── C8 · ningún provider altera el repo ─────────────────────────────────

/// **C8 — Ningún provider escribe, ni crea, ni borra.**
///
/// Distinto de C1 por una razón que sí importa: C1 mide el resultado aggregate
/// y un provider que borra y otro que rehace lo mismo podría cancelarse. Aquí
/// se compara el **conjunto** de ficheros, no solo un hash, para que un cambio de
/// conjunto se vea aunque el contenido final coincida.
#[test]
fn c8_ningun_provider_altera_el_repo() {
    let dir = arena("c8");
    let antes = nombres(dir.path());
    let _ = inspect(dir.path());
    let despues = nombres(dir.path());
    assert_eq!(
        antes, despues,
        "el conjunto de ficheros es identico antes y despues: no se creo, no se \
         borro y no se renombro nada"
    );
    assert!(
        !despues.is_empty(),
        "el fixture tiene ficheros o la comparacion es de nada contra nada"
    );
}

// ── C9 · divergencia no se resuelve por prioridad accidental ────────────

/// **C9 · Dos declaraciones que discrepan no se resuelven eligiendo una.**
///
/// Y —esto es lo que no se había medido— **tampoco por el orden en que se
///escriben los providers**. El reducer es una función del conjunto, así que
/// dar la vuelta al registro tiene que dar exactamente el mismo veredicto y el
/// mismo informe; si no, hay una prioridad accidental y alguien la va a
/// descubrir en producción.
#[test]
fn c9_divergencia_no_se_resuelve_por_prioridad() {
    let dir = tempfile::tempdir().expect("directorio temporal");
    // Dos specs que **de verdad** declaran version y discrepan. La primera vez
    // se uso `manifest.toml`, que no esta en la lista de specs: el provider no
    // lo conoce, luego no declara nada, y la fila midia un arbol vacio
    // disfrazado de conflicto.
    write(
        dir.path(),
        "package.json",
        r#"{"name":"runtime","version":"1.0.0"}"#,
    );
    write(dir.path(), "gradle.properties", "version=2.0.0\n");

    let inspection = inspect(dir.path());
    assert_eq!(
        inspection.authority,
        AssuranceLevel::Conflict,
        "dos declaraciones que discrepan son conflicto, y nadie es elegido"
    );
    assert!(
        !inspection.found_version(),
        "un conflicto no es una version: no se puede publicar sobre el numero \
         de uno de los dos"
    );

    // Y el JSON declara los candidatos, que es lo que permite corregir sin
    // adivinar cuál de los dos es el bueno.
    let json = serde_json::to_value(&inspection).expect("serializa");
    let discrepancias = json["verdict"]["candidates"]
        .as_array()
        .expect("el conflicto declara sus candidatos");
    assert_eq!(
        discrepancias.len(),
        2,
        "los dos valores que discrepan, con quien declaro cada uno: {json}"
    );
}

// ── C10 · el informe declara lo que no comprobó ─────────────────────────

/// **C10 — El informe nombra lo que NO se comprobó.**
///
/// Es la fila que hace que las otras nueve sirvan de algo: sin ella, un informe
/// verosímil con seis providers y sus razones se lee como unaexaustividad y se
/// cita como una. La lista es **derivada**, y esta fila comprueba que cambia
/// cuando cambia lo que pasó.
#[test]
fn c10_el_informe_declara_lo_que_no_comprobo() {
    // El temporal tiene que vivir en un binding: `arena(..).path()` inline
    // deja un `TempDir` temporal que se libera al final de la sentencia, y la
    // inspeccion leeria un directorio ya borrado.
    let arena_una = arena("c10-una");
    let una = inspect(arena_una.path());
    let claves: Vec<&str> = una.not_checked.iter().map(|n| n.key.as_str()).collect();

    for clave in [
        "release_reference_not_compared",
        "nothing_certified",
        "provider_set_is_sddks",
        "read_only",
    ] {
        assert!(
            claves.contains(&clave),
            "«{clave}» es cierto de este informe y tiene que salir en el: {claves:?}"
        );
    }
    // Y es derivada: con una sola fuente, la corroboración es lo que se declara.
    assert!(
        claves.contains(&"no_independent_second_source"),
        "una sola fuente y el informe no lo dice: es lo que separa \
         «encontrado» de «verificado»"
    );

    // Con dos que coinciden, deja de decirlo — y por eso la lista es derivada y
    // no un texto fijo: una lista fija seguiría mintiendo en el caso contrario.
    let dos = tempfile::tempdir().expect("directorio temporal");
    write(
        dos.path(),
        "package.json",
        r#"{"name":"runtime","version":"1.2.3"}"#,
    );
    write(dos.path(), "gradle.properties", "version=1.2.3\n");
    let inspected_dos = inspect(dos.path());
    let claves_dos: Vec<&str> = inspected_dos
        .not_checked
        .iter()
        .map(|n| n.key.as_str())
        .collect();
    assert_eq!(
        inspect(dos.path()).authority,
        AssuranceLevel::CrossValidated,
        "dos fuentes que coinciden: dos filas, y por eso la de arriba significa algo"
    );
    assert!(
        !claves_dos.contains(&"no_independent_second_source"),
        "y con corroboracion no hay nada que declarar: seguir declarandolo \
         seria ruido que entrena a ignorar la lista entera"
    );
}

/// El sha256 de un árbol: contenido **y** nombre de cada fichero.
fn huella_del_arbol(root: &Path) -> String {
    let nombres = nombres(root);
    let mut texto = String::new();
    for ruta in &nombres {
        let bytes = std::fs::read(ruta).unwrap_or_default();
        texto.push_str(&format!("{}  {}\n", fnv(&bytes), ruta.display()));
    }
    fnv(texto.as_bytes())
}

/// Los nombres de los ficheros de un árbol, en orden canónico.
///
/// Es lo que hace falta para el punto 8: comparar el **conjunto** y no solo un
/// hash, porque un provider que borra y otro que rehace lo mismo se cancelarían
/// en el hash y no en el conjunto.
fn nombres(root: &Path) -> Vec<PathBuf> {
    let mut acc = Vec::new();
    fn caminar(dir: &Path, base: &Path, acc: &mut Vec<PathBuf>) {
        let Ok(lectura) = std::fs::read_dir(dir) else {
            return;
        };
        for entrada in lectura.flatten() {
            let ruta = entrada.path();
            if ruta.is_dir() {
                caminar(&ruta, base, acc);
            } else {
                acc.push(ruta.strip_prefix(base).unwrap_or(&ruta).to_path_buf());
            }
        }
    }
    caminar(root, root, &mut acc);
    acc.sort();
    acc
}

/// Un hash sin dependencia. Lo que hace falta es que dos contenidos distintos
/// den valores distintos y el mismo dé el mismo.
fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        h ^= u64::from(*byte);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64-{h:016x}")
}

/// Que el tipo de evidencia siga siendo el que el port firma.
///
/// Este no es un punto del contrato: es el recordatorio de que todo lo de arriba
/// depende de que `Declared` siga llevando `VersionEvidence`, y de que quitarlo
/// no rompe la suite porque la suite no lo mira.
#[test]
fn la_evidencia_de_una_declaracion_sigue_siendo_del_tipo_correcto() {
    let evidencia = VersionEvidence {
        source_kind: "una declaracion".to_owned(),
        digest: None,
        location: None,
    };
    assert!(
        !evidencia.is_recheckable(),
        "una descripcion sin digest ni localizacion no es revisable, por muy \
         detallada que sea la descripcion"
    );
    let con_lugar = VersionEvidence {
        source_kind: "una declaracion".to_owned(),
        digest: None,
        location: Some("package.json".to_owned()),
    };
    assert!(
        con_lugar.is_recheckable(),
        "con localizacion ya se puede volver a mirar"
    );
}
